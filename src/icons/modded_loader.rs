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
use regex::Regex;
use crate::icons::atlas::Atlas;
use crate::util::AddSpan;

pub fn parse_bnd_and_tpf(dir_modded_spells: &mut HashMap<u16, AtlasIcon>,
                         render_context: &mut dyn RenderContext, read_success: &mut ReadSuccess) -> anyhow::Result<()> {
    let atlases = parse_atlases(render_context, read_success)?;

    for entry in read_success.bnd.files.iter() {
        parse_entry(dir_modded_spells, read_success, &atlases, entry).add_span()?;
    }
    Ok(())
}

fn parse_entry(dir_modded_spells: &mut HashMap<u16, AtlasIcon>, read_success: &ReadSuccess, atlases: &HashMap<String, Atlas>, entry: &BND4Entry) -> anyhow::Result<()> {
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
        if let Err(err) = parse_node(dir_modded_spells, atlas, node).add_span() {
            tracing::error!("Error loading directory modded spell: {err:?}");
        }
    }
    Ok(())
}

fn parse_node(dir_modded_spells: &mut HashMap<u16, AtlasIcon>, atlas: &Atlas, node: Node) -> anyhow::Result<()> {
    // creates the icon
    let icon = AtlasIcon::try_from_xml(&atlas, &node).add_span()?;

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

    if dir_modded_spells.contains_key(&id) {
        tracing::warn!("Modded spells HashMap already contains an item with id {id}");
    }
    dir_modded_spells.insert(id, icon);
    Ok(())
}

fn parse_atlases(render_context: &mut dyn RenderContext, read_success: &mut ReadSuccess) -> anyhow::Result<HashMap<String, Atlas>> {
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

        // if is_debugging() {
        //     image::save_buffer(paths::mods().join(format!("{}.png", texture.name)),
        //                        &icon.data, icon.width, icon.height, Rgba8).add_span()?;
        // }

        // Stores atlas
        let texture_id = render_context.load_texture(&icon.data, width, height).add_span()?;
        atlases.insert(texture.name.clone(), Atlas::new(texture.name.clone(), texture_id, width, height));
    }
    Ok(atlases)
}

pub fn load_modded_spells<P>(dir_modded_spells: &mut HashMap<u16, AtlasIcon>,
                             render_context: &mut dyn RenderContext, path: P) -> anyhow::Result<()>
where P: AsRef<Path> {
    let mut read_success = modded_reader::read(path).add_span()?;
    parse_bnd_and_tpf(dir_modded_spells, render_context, &mut read_success)
}