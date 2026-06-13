use std::collections::HashMap;
use std::fs;
use std::fs::read_to_string;
use std::path::Path;
use std::sync::OnceLock;
use imgui::TextureId;
use fstools_formats::bnd4::BND4;
use hudhook::RenderContext;
use lazy_static::lazy_static;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use crate::dynamic_icons::modded_reader;
use crate::icons::{json_loader, modded_loader, AtlasIcon, ModdedSpell};
use crate::paths;
use crate::settings::Settings;

lazy_static!(
    static ref ICON_MANAGER: OnceLock<IconManager> = OnceLock::new();
);

#[derive(Clone, Copy, Debug)]
pub enum IconResult {
    Id(TextureId),
    Atlas(AtlasIcon),
    None,
}

#[derive(Debug)]
pub struct IconManager {
    spell_icons: HashMap<u32, TextureId>,
    json_modded_spell: HashMap<u32, TextureId>,
    dir_modded_spells: HashMap<u32, AtlasIcon>,
}

impl IconManager {
    pub fn get(spell_id: u32) -> IconResult {
        match ICON_MANAGER.get() {
            Some(manager) => manager.get_inner(spell_id),
            None => IconResult::None,
        }
    }
    
    fn get_inner(&self, spell_id: u32) -> IconResult {
        if let Some(&ai) = self.dir_modded_spells.get(&spell_id) {
            return IconResult::Atlas(ai);
        }
        if let Some(&id) = self.json_modded_spell.get(&spell_id).or(self.spell_icons.get(&spell_id)) {
            return IconResult::Id(id);
        }
        IconResult::None
    }

    fn load_modded_spells(render_context: &mut dyn RenderContext) -> (HashMap<u32, TextureId>, HashMap<u32, AtlasIcon>) {
        let mut json_modded_spells = HashMap::new();
        let mut dir_modded_spells = HashMap::new();
        for modded_spells in Settings::read_or_default().modded_spells {
            if modded_spells.ends_with(".json") {
                match read_to_string(paths::spell_icons().join(&modded_spells)) {
                    Ok(json) => {
                        if let Err(err) = json_loader::load_modded_spells(&mut json_modded_spells, render_context, &json) {
                            tracing::error!("Error loading modded spells '{modded_spells}': {err}");
                        }
                    }
                    Err(err) => {
                        tracing::error!("Error trying to load modded spells '{modded_spells}': {err}");
                    }
                }
            } else if modded_spells.ends_with(".toml") {
                todo!()
            } else {
                if let Err(err) = modded_loader::load_modded_spells(&mut dir_modded_spells, render_context, &modded_spells) {
                    tracing::error!("Error loading modded spells '{modded_spells}': {err}");
                }
            }
        }
        (json_modded_spells, dir_modded_spells)
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
        let (json_modded_spell, dir_modded_spells) = Self::load_modded_spells(render_context);
        tracing::info!("Icons loaded");

        Ok(Self {
            spell_icons,
            json_modded_spell,
            dir_modded_spells,
        })
    }
}