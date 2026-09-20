use crate::icons::primary_color::PrimaryColors;
use imgui::TextureId;
use std::sync::Arc;
use image_dds::ddsfile::Dds;

#[derive(Clone, Debug, PartialEq)]
pub struct Atlas {
    pub name: Arc<str>,
    pub source: Arc<str>,
    pub atlas_texture: Option<AtlasTexture>,
    pub pc: Option<PrimaryColors>,
    pub used: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasTexture {
    pub texture_id: TextureId,
    pub width: u32,
    pub height: u32,
}

impl Atlas {
    pub fn new(name: String, source: Arc<str>, dds: &Dds) -> Self {
        Self {
            name: name.into(),
            source: source.into(),
            atlas_texture: None,
            pc: PrimaryColors::try_from(dds).ok(),
            used: false,
        }
    }

    pub fn get_primary_color(&self, x: u32, y: u32, w: u32, h: u32) -> Option<[u8; 3]> {
        self.pc.as_ref().and_then(|pc| pc.dominant_pop_color(x, y, x + w, y + h))
    }
    
    pub fn set_texture(&mut self, texture_id: TextureId, width: u32, height: u32) {
        self.atlas_texture = Some(AtlasTexture {
            texture_id,
            width,
            height,
        });
    }
}