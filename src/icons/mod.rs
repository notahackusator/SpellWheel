pub mod icon_manager;
pub mod json_loader;
pub mod modengine_loader;
pub mod modded_loader;
pub mod atlas;

use std::io::{Error, ErrorKind};
use imgui::TextureId;
use roxmltree::Node;
use serde::Deserialize;
use crate::icons::atlas::Atlas;
use crate::util::AddSpan;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtlasIcon {
    pub texture_id: TextureId,
    pub rect: [f32; 4],
}

impl AtlasIcon {
    pub fn try_from_xml(atlas: &Atlas, node: &Node) -> anyhow::Result<Self> {
        let x: f32 = node.attribute("x").ok_or(Error::new(
            ErrorKind::NotFound, format!("No 'x' element in subtexture of {}", atlas.name)))
            .add_span()?
            .parse()
            .add_span()?;
        let y: f32 = node.attribute("y").ok_or(Error::new(
            ErrorKind::NotFound, format!("No 'y' element in subtexture of {}", atlas.name)))
            .add_span()?
            .parse()
            .add_span()?;
        let width: f32 = node.attribute("width").ok_or(Error::new(
            ErrorKind::NotFound, format!("No 'width' element in subtexture of {}", atlas.name)))
            .add_span()?
            .parse()
            .add_span()?;
        let height: f32 = node.attribute("height").ok_or(Error::new(
            ErrorKind::NotFound, format!("No 'height' element in subtexture of {}", atlas.name)))
            .add_span()?
            .parse()
            .add_span()?;

        Ok(AtlasIcon {
            texture_id: atlas.texture_id,
            rect: [
                x / atlas.width as f32,
                y / atlas.height as f32,
                width / atlas.width as f32,
                height / atlas.height as f32
            ]
        })
    }
}

#[derive(Deserialize)]
pub struct ModdedSpell {
    pub id: u32,
    pub path_to_icon: String,
}