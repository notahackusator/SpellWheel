use imgui::TextureId;

pub struct Atlas {
    pub name: String,
    pub atlas_texture: Option<AtlasTexture>,
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
            name,
            atlas_texture: None,
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