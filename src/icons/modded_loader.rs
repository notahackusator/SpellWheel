use crate::dynamic_icons::modded_reader;
use crate::dynamic_icons::read_success::ReadSuccess;
use crate::icons::AtlasIcon;
use hudhook::RenderContext;
use image_dds::ddsfile::Dds;
use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::path::Path;
use fstools_formats::bnd4::BND4Entry;
use imgui::TextureId;
use roxmltree::Node;

pub fn parse_bnd_and_tpf(dir_modded_spells: &mut HashMap<u32, AtlasIcon>,
                         render_context: &mut dyn RenderContext, read_success: &mut ReadSuccess) -> anyhow::Result<()> {
    let atlases = parse_atlases(render_context, &read_success)?;

    for entry in read_success.bnd.files.iter() {
        parse_entry(dir_modded_spells, read_success, &atlases, entry)?;
    }
    Ok(())
}

fn parse_entry(dir_modded_spells: &mut HashMap<u32, AtlasIcon>, read_success: &mut ReadSuccess, atlases: &HashMap<String, TextureId>, entry: &BND4Entry) -> anyhow::Result<()> {
    let xml_bytes = read_success.bnd.file_bytes(entry);
    let xml_str = std::str::from_utf8(xml_bytes)?;

    let doc = roxmltree::Document::parse(xml_str)?;

    // Gets the atlas
    let image_path = doc.root_element()
        .attribute("imagePath")
        .ok_or(Error::new(ErrorKind::NotFound, "no imagePath attribute"))?;

    let atlas_name = image_path.trim_end_matches(".png");
    let atlas = *atlases.get(atlas_name)
        .ok_or(Error::new(ErrorKind::NotFound, format!("imagePath ({atlas_name}) not mapped to any atlas ({atlases:?})")))?;

    // Parses each subtexture (icon)
    for n in doc.descendants().filter(|n| n.has_tag_name("SubTexture")) {
        parse_node(dir_modded_spells, atlas_name, atlas, n)?;
    }
    Ok(())
}

fn parse_node(dir_modded_spells: &mut HashMap<u32, AtlasIcon>, atlas_name: &str, atlas: TextureId, n: Node) -> anyhow::Result<()> {
    // creates the icon
    let icon = AtlasIcon {
        texture_id: atlas,
        rect: [
            n.attribute("x").ok_or(Error::new(
                ErrorKind::NotFound, format!("No 'x' element in subtexture of {}", atlas_name)))?.parse()?,
            n.attribute("y").ok_or(Error::new(
                ErrorKind::NotFound, format!("No 'y' element in subtexture of {}", atlas_name)))?.parse()?,
            n.attribute("width").ok_or(Error::new(
                ErrorKind::NotFound, format!("No 'width' element in subtexture of {}", atlas_name)))?.parse()?,
            n.attribute("height").ok_or(Error::new(
                ErrorKind::NotFound, format!("No 'height' element in subtexture of {}", atlas_name)))?.parse()?,
        ]
    };

    // parses the item id
    let raw_name = n.attribute("name").unwrap().to_string();
    let num_start = raw_name.rfind("_").map(|idx| idx + 1).unwrap_or(0);
    let num_end = raw_name.find(".").unwrap_or(raw_name.len());
    let id = raw_name[num_start..num_end].parse()?;

    dir_modded_spells.insert(id, icon);
    Ok(())
}

fn parse_atlases(render_context: &mut dyn RenderContext, read_success: &&mut ReadSuccess) -> anyhow::Result<HashMap<String, TextureId>> {
    let mut atlases = HashMap::new();

    // Converts TPF to atlases
    for texture in read_success.tpf.textures {
        // Reads DDS compressed texture
        let mut dds_bytes = texture.bytes(&mut read_success.tpf_cursor)?;
        let dds = Dds::read(dds_bytes.as_mut_slice())?;

        // Converts to image
        let surface = image_dds::Surface::from_dds(&dds)?;
        let icon = surface.decode_rgba8()?;

        // Stores atlas
        let texture_id = render_context.load_texture(icon.as_bytes(), icon.width, icon.height)?;
        atlases.insert(texture.name, texture_id);
    }
    Ok(atlases)
}

pub fn load_modded_spells<P>(dir_modded_spells: &mut HashMap<u32, AtlasIcon>,
                             render_context: &mut dyn RenderContext, path: P) -> anyhow::Result<()>
where P: AsRef<Path> {
    let mut read_success = modded_reader::read(path)?;
    parse_bnd_and_tpf(dir_modded_spells, render_context, &mut read_success)
}