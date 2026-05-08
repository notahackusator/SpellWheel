use rayon::iter::ParallelIterator;
use std::collections::HashMap;
use std::fs;
use std::fs::read_to_string;
use std::sync::OnceLock;
use hudhook::RenderContext;
use imgui::TextureId;
use lazy_static::lazy_static;
use rayon::iter::IntoParallelIterator;
use serde::Deserialize;
use crate::paths;
use crate::settings::Settings;

lazy_static!(
    static ref ICON_MANAGER: OnceLock<IconManager> = OnceLock::new();
);

#[derive(Debug)]
pub struct IconManager {
    spell_icons: HashMap<u32, TextureId>,
    modded_spell_icons: HashMap<u32, TextureId>,
}

impl IconManager {
    pub fn get(spell_id: u32) -> Option<TextureId> {
        ICON_MANAGER.get()?.get_inner(spell_id)
    }
    
    fn get_inner(&self, spell_id: u32) -> Option<TextureId> {
        self.modded_spell_icons.get(&spell_id)
            .or(self.spell_icons.get(&spell_id))
            .copied()
    }

    fn load_modded_spell_from_json(all_modded_spells: &mut HashMap<u32, TextureId>,
                                   render_context: &mut dyn RenderContext, modded_spell: &ModdedSpell) -> Result<(), String> {
        let icon = image::open(paths::spell_icons().join(&modded_spell.path_to_icon))
            .map_err(|err| err.to_string())?;
        let texture_id = render_context.load_texture(icon.as_bytes(), icon.width(), icon.height())
            .map_err(|err| err.to_string())?;
        all_modded_spells.insert(modded_spell.id, texture_id);
        Ok(())
    }

    fn load_modded_spells_from_json(all_modded_spells: &mut HashMap<u32, TextureId>,
                                    render_context: &mut dyn RenderContext, json: &str) -> Result<(), String> {
        let json: Vec<ModdedSpell> = serde_json::from_str(&json).map_err(|err| err.to_string())?;
        for modded_spell in json {
            if let Err(err) = Self::load_modded_spell_from_json(all_modded_spells, render_context, &modded_spell) {
                tracing::error!("Error loading modded spell {}: {}", modded_spell.id, err)
            }
        }
        Ok(())
    }

    fn load_modded_spells(render_context: &mut dyn RenderContext) -> HashMap<u32, TextureId> {
        let mut all_modded_spells = HashMap::new();
        for modded_spells in Settings::read_or_default().modded_spells {
            match read_to_string(paths::spell_icons().join(&modded_spells)) {
                Ok(json) => {
                    if let Err(err) = Self::load_modded_spells_from_json(&mut all_modded_spells, render_context, &json) {
                        tracing::error!("Error loading modded spells '{modded_spells}': {err}");
                    }
                }
                Err(err) => {
                    tracing::error!("Error trying to load modded spells '{modded_spells}': {err}");
                }
            }
        }
        all_modded_spells
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
        
        let spell_icons = HashMap::from_iter(
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
        tracing::info!("  Loading modded spells");
        let modded_spell_icons = Self::load_modded_spells(render_context);
        tracing::info!("Icons loaded");

        Ok(Self {
            spell_icons,
            modded_spell_icons,
        })
    }
}

#[derive(Deserialize)]
struct ModdedSpell {
    id: u32,
    path_to_icon: String,
}