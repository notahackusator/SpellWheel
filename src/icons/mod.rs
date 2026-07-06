pub mod icon_manager;
pub mod modengine_loader;
pub mod modded_loader;
pub mod atlas;
pub mod await_graphics;
mod generic_loader;
pub mod vanilla_loader;

use std::io;
use crate::icons::atlas::{Atlas, AtlasTexture};
use crate::util::AddSpan;
use imgui::TextureId;
use roxmltree::Node;
use std::io::{Error, ErrorKind};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub struct AtlasIcon {
    pub atlas_name: Arc<str>,
    pub texture_id: TextureId,
    pub rect: [f32; 4],
}

impl AtlasIcon {
    pub fn try_parse_rect(node: &Node) -> anyhow::Result<[f32; 4]> {
        let x: f32 = node.attribute("x")
            .ok_or(Error::new(ErrorKind::NotFound, "No 'x' element".to_string()))
            .add_span()?
            .parse()
            .add_span()?;
        let y: f32 = node.attribute("y")
            .ok_or(Error::new(ErrorKind::NotFound, "No 'y' element".to_string()))
            .add_span()?
            .parse()
            .add_span()?;
        let w: f32 = node.attribute("width")
            .ok_or(Error::new(ErrorKind::NotFound, "No 'width' element".to_string()))
            .add_span()?
            .parse()
            .add_span()?;
        let h: f32 = node.attribute("height")
            .ok_or(Error::new(ErrorKind::NotFound, "No 'height' element".to_string()))
            .add_span()?
            .parse()
            .add_span()?;

        Ok([x, y, w, h])
    }

    pub fn from_geometry(atlas: Atlas, rect: [f32; 4]) -> anyhow::Result<Self> {
        let atlas_texture = atlas.atlas_texture
            .ok_or(Error::new(ErrorKind::NotFound, "Expected atlas texture to be initialized"))?;
        Ok(AtlasIcon {
            atlas_name: atlas.name,
            texture_id: atlas_texture.texture_id,
            rect: [
                rect[0] / atlas_texture.width as f32,
                rect[1] / atlas_texture.height as f32,
                rect[2] / atlas_texture.width as f32,
                rect[3] / atlas_texture.height as f32
            ]
        })
    }
}