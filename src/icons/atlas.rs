use std::sync::Arc;
use imgui::TextureId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Atlas {
    pub name: Arc<str>,
    pub atlas_texture: Option<AtlasTexture>,
    pub used: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasTexture {
    pub texture_id: TextureId,
    pub width: u32,
    pub height: u32,
}

impl Atlas {
    pub fn new(name: String) -> Self {
        Self {
            name: name.into(),
            atlas_texture: None,
            used: false,
        }
    }
    
    pub fn set_texture(&mut self, texture_id: TextureId, width: u32, height: u32) {
        self.atlas_texture = Some(AtlasTexture {
            texture_id,
            width,
            height,
        });
    }
}