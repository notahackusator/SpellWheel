use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::read_to_string;
use std::sync::OnceLock;
use imgui::TextureId;
use hudhook::RenderContext;
use lazy_static::lazy_static;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use crate::dynamic_icons::modded_reader;
use crate::icons::{json_loader, modded_loader, AtlasIcon};
use crate::paths;
use crate::settings::Settings;
use crate::spells::Spell;

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
    dir_modded_spells: HashMap<u16, AtlasIcon>,
}

impl IconManager {
    pub fn get(spell: &Spell) -> IconResult {
        match ICON_MANAGER.get() {
            Some(manager) => manager.get_inner(spell),
            None => IconResult::None,
        }
    }
    
    fn get_inner(&self, spell: &Spell) -> IconResult {
        if let Some(&ai) = self.dir_modded_spells.get(&spell.icon_id()) {
            return IconResult::Atlas(ai);
        }
        if let Some(&id) = self.json_modded_spell.get(&spell.id()).or(self.spell_icons.get(&spell.id())) {
            return IconResult::Id(id);
        }
        IconResult::None
    }

    fn load_from_json(render_context: &mut dyn RenderContext) -> HashMap<u32, TextureId> {
        let mut json_modded_spells = HashMap::new();
        for modded_spells_path in Settings::read_or_default().modded_spells {
            if !modded_spells_path.ends_with(".json") {
                continue;
            }

            match read_to_string(paths::spell_icons().join(&modded_spells_path)) {
                Ok(json) => {
                    if let Err(err) = json_loader::load_modded_spells(&mut json_modded_spells, render_context, &json) {
                        tracing::error!("Error loading modded spells '{modded_spells_path}': {err:?}");
                    }
                }
                Err(err) => {
                    tracing::error!("Error trying to load modded spells '{modded_spells_path}': {err:?}");
                }
            }
        }
        json_modded_spells
    }

    fn load_from_directories(render_context: &mut dyn RenderContext) -> HashMap<u16, AtlasIcon> {
        let mut dir_modded_spells = HashMap::new();

        let mut paths = HashSet::new();
        for modded_spells_path in Settings::read_or_default().modded_spells {
            if modded_spells_path.contains(".") {
                continue;
            }

            paths.insert(modded_spells_path.into());
        }
        if let Some(upstream_mod_folder) = modded_reader::search_for_mod_folder() {
            tracing::info!("Found upstream mod folder: {upstream_mod_folder:?}");
            paths.insert(upstream_mod_folder);
        }

        for modded_spells_path in paths {
            if let Err(err) = modded_loader::load_modded_spells(&mut dir_modded_spells, render_context, &modded_spells_path) {
                tracing::error!("Error loading modded spells '{modded_spells_path:?}': {err:?}");
            }
        }

        dir_modded_spells
    }

    fn load_modded_spells(render_context: &mut dyn RenderContext) -> (HashMap<u32, TextureId>, HashMap<u16, AtlasIcon>) {
        (Self::load_from_json(render_context), Self::load_from_directories(render_context))
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