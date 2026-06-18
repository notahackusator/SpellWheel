pub mod icon_manager;
pub mod json_loader;
pub mod modengine_loader;
pub mod modded_loader;

use imgui::TextureId;
use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtlasIcon {
    pub texture_id: TextureId,
    pub rect: [f32; 4],
}

#[derive(Deserialize)]
pub struct ModdedSpell {
    pub id: u32,
    pub path_to_icon: String,
}