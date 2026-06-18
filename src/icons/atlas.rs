use imgui::TextureId;

pub struct Atlas {
    pub name: String,
    pub texture_id: TextureId,
    pub width: u32,
    pub height: u32,
}

impl Atlas {
    pub fn new(name: String, texture_id: TextureId, width: u32, height: u32) -> Self {
        Self {
            name,
            texture_id,
            width,
            height,
        }
    }
}