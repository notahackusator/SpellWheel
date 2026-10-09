use std::sync::{Arc, Mutex};
use hudhook::RenderContext;
use hudhook::windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT;
use imgui::TextureId;
use lazy_static::lazy_static;

lazy_static!(
    static ref TEXTURE_MANAGER: Arc<Mutex<TextureManager>> = Arc::new(Mutex::new(TextureManager::new()));
);

pub struct TextureManager {
    loaded_textures: Vec<TextureId>,
    available_textures: Vec<TextureId>,
}

impl TextureManager {
    pub fn new() -> Self {
        Self {
            loaded_textures: Vec::new(),
            available_textures: Vec::new()
        }
    }

    pub fn load(render_context: &mut dyn RenderContext, format: DXGI_FORMAT,
                data: &[u8], width: u32, height: u32) -> anyhow::Result<TextureId> {
        TEXTURE_MANAGER.lock().expect("Couldn't acquire TEXTURE_MANAGER")
            .load_inner(render_context, format, data, width, height)
    }

    pub fn load_inner(&mut self, render_context: &mut dyn RenderContext, format: DXGI_FORMAT,
                data: &[u8], width: u32, height: u32) -> anyhow::Result<TextureId> {
        if let Some(texture) = self.available_textures.pop() {
            render_context.replace_texture(
                texture, format, data, width, height
            )?;
            self.loaded_textures.push(texture);
            Ok(texture)
        } else {
            let texture = render_context.load_texture(
                format, data, width, height
            )?;
            self.loaded_textures.push(texture);
            Ok(texture)
        }
    }

    pub fn unload(texture: TextureId) {
        TEXTURE_MANAGER.lock().expect("Couldn't acquire TEXTURE_MANAGER")
            .unload_inner(texture)
    }

    pub fn unload_inner(&mut self, texture: TextureId) {
        if let Some(idx) = self.loaded_textures.iter().position(|x| x == &texture) {
            self.loaded_textures.remove(idx);
            self.available_textures.push(texture);
        }
    }

    pub fn unload_all() {
        TEXTURE_MANAGER.lock().expect("Couldn't acquire TEXTURE_MANAGER")
            .unload_all_inner()
    }

    pub fn unload_all_inner(&mut self) {
        self.available_textures.append(&mut self.loaded_textures);
    }
}