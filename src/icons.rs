use rayon::iter::ParallelIterator;
use std::collections::HashMap;
use std::fs;
use std::sync::OnceLock;
use hudhook::RenderContext;
use imgui::TextureId;
use lazy_static::lazy_static;
use rayon::iter::IntoParallelIterator;
use crate::paths;

lazy_static!(
    static ref ICON_MANAGER: OnceLock<IconManager> = OnceLock::new();
);

#[derive(Debug)]
pub struct IconManager {
    spell_icons: HashMap<u32, TextureId>
}

impl IconManager {
    pub fn get(spell_id: u32) -> Option<TextureId> {
        ICON_MANAGER.get()?.get_inner(spell_id)
    }
    
    fn get_inner(&self, spell_id: u32) -> Option<TextureId> {
        self.spell_icons.get(&spell_id).map(|id| id.clone())
    }
    
    pub fn load(render_context: &mut dyn RenderContext) {
        tracing::info!("Loading spell icons...");
        ICON_MANAGER.set(Self::load_inner(render_context).expect("Could not load ImageManager"))
            .expect("Could not load image manager");
    }
    
    fn load_inner(render_context: &mut dyn RenderContext) -> Result<Self, String> {
        // get the files
        tracing::info!("  Files found");
        let files = fs::read_dir(paths::spell_icons()).map_err(|err| err.to_string())?
            .filter_map(|x| x.ok())
            .collect::<Vec<_>>();
        
        let images = files.into_par_iter()
            .filter_map(|file| {
                // convert file name (OsString) to spell id (u32)
                file.file_name().to_str()
                    .and_then(|str| str.split(".").next())
                    .and_then(
                        |str| str.parse().ok().map(|spell_id| (
                            spell_id, image::open(file.path())
                        ))
                    )
            })
            .collect::<Vec<_>>();
        tracing::info!("  Images loaded");
        
        let images = HashMap::from_iter(
            images.into_iter()
                .filter_map(|(spell_id, img)|
                    // try to bind the image
                    img.ok()
                        .and_then(
                            |img| render_context.load_texture(img.as_bytes(), img.width(), img.height()).ok()
                        )
                        .map(|texture_id| (spell_id, texture_id))
                )
                .collect::<Vec<_>>()
        );
        tracing::info!("  Textures loaded");
        tracing::info!("Icons loaded");

        Ok(Self {
            spell_icons: images
        })
    }
}