use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::sync::{Arc, Mutex};
use fstools_formats::bnd4::BND4Entry;
use roxmltree::Node;
use regex::Regex;
use image_dds::ddsfile::Dds;
use crate::dynamic_icons::read_success::ReadSuccess;
use crate::icons::atlas::Atlas;
use crate::icons::AtlasIcon;
use crate::icons::await_graphics::AwaitGraphics;
use crate::util::AddSpan;

pub fn parse_bnd_and_tpf(await_graphics: &mut Vec<AwaitGraphics>, read_success: &mut ReadSuccess) -> anyhow::Result<()> {
    let atlases = parse_atlases(await_graphics, read_success)?;

    for entry in read_success.bnd.files.iter() {
        parse_entry(await_graphics, read_success, &atlases, entry).add_span()?;
    }
    Ok(())
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
            tracing::error!("Error loading spells: {err}");
        }
    }
    Ok(())
}

fn parse_node(await_graphics: &mut Vec<AwaitGraphics>, atlas: Arc<Mutex<Atlas>>, node: Node) -> anyhow::Result<()> {
    // parses the item id
    let raw_name = node.attribute("name").unwrap().to_string();
    if !raw_name.contains("MENU_ItemIcon") {
        return Ok(());
    }
    let id_regex = Regex::new("[0-9]+").add_span()?;
    let Some(id) = id_regex.find(&raw_name) else {
        return Ok(());
    };
    let Ok(id) = id.as_str().parse() else {
        return Ok(())
    };

    let rect = AtlasIcon::try_parse_rect(&node)?;

    await_graphics.push(Box::new(move |_, spell_icons| {
        let atlas_texture = atlas.lock().unwrap().atlas_texture
            .ok_or(Error::new(ErrorKind::InvalidData, "AtlasIcon texture should be initialized"))?;
        let icon = AtlasIcon::from_geometry(&atlas_texture, rect).add_span()?;

        spell_icons.insert(id, icon);
        Ok(())
    }));
    Ok(())
}

fn parse_atlases(await_graphics: &mut Vec<AwaitGraphics>, read_success: &mut ReadSuccess) -> anyhow::Result<HashMap<String, Arc<Mutex<Atlas>>>> {
    let mut atlases = HashMap::new();

    // Converts TPF to atlases
    for texture in read_success.tpf.textures.iter() {
        // Reads DDS compressed texture
        let dds_bytes = texture.bytes(&mut read_success.tpf_cursor).add_span()?;
        let dds = Dds::read(dds_bytes.as_slice()).add_span()?;

        // Converts to image
        let surface = image_dds::Surface::from_dds(&dds)?;
        let icon = surface.decode_rgba8().add_span()?;
        let width = icon.width;
        let height = icon.height;

        let atlas = Arc::new(Mutex::new(Atlas::new(texture.name.clone())));
        atlases.insert(texture.name.clone(), atlas.clone());
        await_graphics.push(Box::new(move |render_context, _| {
            // Stores atlas
            let texture_id = render_context.load_texture(&icon.data, width, height)?;
            atlas.lock().unwrap().set_texture(texture_id, width, height);
            Ok(())
        }));
    }
    Ok(atlases)
}