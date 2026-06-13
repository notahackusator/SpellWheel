pub mod icon_manager;
pub mod json_loader;
pub mod modengine_loader;
pub mod modded_loader;

use hudhook::RenderContext;
use imgui::TextureId;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtlasIcon {
    texture_id: TextureId,
    rect: [f32; 4],
}

#[derive(Deserialize)]
struct ModdedSpell {
    id: u32,
    path_to_icon: String,
}