use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::io::{Error, ErrorKind};
use std::sync::{Arc, Mutex};
use fstools_formats::bnd4::BND4Entry;
use hudhook::windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT, DXGI_FORMAT_R8G8B8A8_UNORM};
use roxmltree::Node;
use image_dds::ddsfile::Dds;
use crate::dynamic_icons::read_success::ReadSuccess;
use crate::icons::atlas::Atlas;
use crate::icons::AtlasIcon;
use crate::icons::await_graphics::AwaitGraphics;
use crate::util::AddSpan;

pub fn parse_bnd_and_tpf(await_graphics: &mut Vec<AwaitGraphics>, read_success: &mut ReadSuccess) -> anyhow::Result<AtlasSize> {
    let (atlases, atlas_size) = parse_atlases(await_graphics, read_success)?;

    for entry in read_success.bnd.files.iter() {
        parse_entry(await_graphics, read_success, &atlases, entry).add_span()?;
    }
    Ok(atlas_size)
}

fn parse_entry(await_graphics: &mut Vec<AwaitGraphics>, read_success: &ReadSuccess, atlases: &HashMap<String, Arc<Mutex<Atlas>>>, entry: &BND4Entry) -> anyhow::Result<()> {
    let xml_bytes = read_success.bnd.file_bytes(entry);
    let xml_str = std::str::from_utf8(xml_bytes).add_span()?;

    let doc = roxmltree::Document::parse(xml_str).add_span()?;

    // Gets the atlas
    let image_path = doc.root_element()
        .attribute("imagePath")
        .ok_or(Error::new(ErrorKind::NotFound, "no imagePath attribute")).add_span()?;

    let atlas_name = image_path.trim_end_matches(".png");
    let Some(atlas) = atlases.get(atlas_name) else {
        tracing::info!("Skipping imagePath ({atlas_name}), not mapped to any atlas");
        return Ok(());
    };

    // Parses each subtexture (icon)
    for node in doc.descendants().filter(|node| node.has_tag_name("SubTexture")) {
        if let Err(err) = parse_node(await_graphics, atlas.clone(), node).add_span() {
            tracing::error!("Error loading icons: {err}");
        }
    }
    Ok(())
}

fn parse_node(await_graphics: &mut Vec<AwaitGraphics>, atlas: Arc<Mutex<Atlas>>, node: Node) -> anyhow::Result<()> {
    // parses the item id
    let raw_name = node.attribute("name").unwrap().to_string();
    let Some(id) = parse_name(raw_name) else {
        return Ok(());
    };

    let mut atlas_lock = atlas.lock().unwrap();
    // This prevents unnecessary atlases from loading
    atlas_lock.used = true;
    drop(atlas_lock);

    let rect = AtlasIcon::try_parse_rect(&node)?;

    await_graphics.push(Box::new(move |_, icons| {
        let atlas_lock = atlas.lock().unwrap();
        let icon = AtlasIcon::from_geometry(atlas_lock.clone(), rect).add_span()?;

        if let Some(icon) = icons.get(&id) {
            let this_name = &atlas_lock.name;
            let other_name = &icon.atlas_name;
            tracing::warn!("Overlapping item ID's: {id}. Used in this atlas ({this_name}) and ({other_name})");
        }
        icons.insert(id, icon);
        Ok(())
    }));
    Ok(())
}

fn parse_name(name: String) -> Option<u16> {
    let num_start = name.trim_start_matches("MENU_ItemIcon_");
    if num_start.len() == name.len() {
        return None;
    }

    let num_end = num_start.find(".")?;
    let num_str = num_start.get(..num_end)?;

    num_str.parse().ok()
}

pub type Atlases = HashMap<String, Arc<Mutex<Atlas>>>;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtlasSize {
    pub compressed: u32,
    pub uncompressed: u32,
}

impl Display for AtlasSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut c_f32 = self.compressed as f32;
        let mut c_suffix = "B";

        let mut u_f32 = self.uncompressed as f32;
        let mut u_suffix = "B";

        for (size, suf) in [(1_000_000_000, "GB"), (1_000_000, "MB"), (1_000, "KB")] {
            if self.compressed > size {
                c_f32 = self.compressed as f32 / size as f32;
                c_suffix = suf;
                break;
            }
        }

        for (size, suf) in [(1_000_000_000, "GB"), (1_000_000, "MB"), (1_000, "KB")] {
            if self.uncompressed > size {
                u_f32 = self.uncompressed as f32 / size as f32;
                u_suffix = suf;
                break;
            }
        }

        write!(f, "{u_f32:.2}{u_suffix} uncompressed, {c_f32:.2}{c_suffix} compressed")
    }
}

impl AtlasSize {
    pub fn new(compressed: u32, uncompressed: u32) -> Self {
        Self {
            compressed,
            uncompressed,
        }
    }
}

fn parse_atlases(await_graphics: &mut Vec<AwaitGraphics>, read_success: &mut ReadSuccess) -> anyhow::Result<(Atlases, AtlasSize)> {
    let mut atlases = HashMap::new();

    let mut size_u = 0;
    let mut size_c = 0;

    // Converts TPF to atlases
    for texture in read_success.tpf.textures.iter() {
        let atlas = Arc::new(Mutex::new(Atlas::new(texture.name.clone())));
        atlases.insert(texture.name.clone(), atlas.clone());

        // Reads DDS compressed texture
        let dds_bytes = texture.bytes(&mut read_success.tpf_cursor).add_span()?;
        let dds = Dds::read(dds_bytes.as_slice()).add_span()?;
        match dds.get_dxgi_format() {
            Some(format) => {
                size_c += dds_bytes.len() as u32;
                await_graphics.push(Box::new(move |render_context, _| {
                    let mut atlas = atlas.lock().unwrap();
                    if !atlas.used {
                        return Ok(());
                    }
                    // Stores atlas
                    let texture_id = render_context.load_texture(
                        DXGI_FORMAT(format as i32), dds.get_data(0).unwrap(), dds.get_width(), dds.get_height()
                    )?;
                    atlas.set_texture(texture_id, dds.get_width(), dds.get_height());
                    Ok(())
                }));
            }
            None => {
                // Converts to image
                let surface = image_dds::Surface::from_dds(&dds)?;
                let icon = surface.decode_rgba8().add_span()?;
                let width = icon.width;
                let height = icon.height;

                size_u += width * height * 4;

                await_graphics.push(Box::new(move |render_context, _| {
                    let mut atlas = atlas.lock().unwrap();
                    if !atlas.used {
                        return Ok(());
                    }
                    // Stores atlas
                    let texture_id = render_context.load_texture(DXGI_FORMAT_R8G8B8A8_UNORM, &icon.data, width, height)?;
                    atlas.set_texture(texture_id, width, height);
                    Ok(())
                }));
            }
        }
    }
    Ok((atlases, AtlasSize::new(size_c, size_u)))
}