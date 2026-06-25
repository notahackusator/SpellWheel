use std::collections::{HashMap, HashSet};
use std::mem::take;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::Instant;
use imgui::TextureId;
use hudhook::RenderContext;
use lazy_static::lazy_static;
use crate::dynamic_icons::modded_reader;
use crate::icons::{modded_loader, vanilla_loader, AtlasIcon};
use crate::icons::await_graphics::AwaitGraphics;
use crate::settings::Settings;
use crate::spells::Spell;

lazy_static!(
    static ref ICON_MANAGER: OnceLock<Arc<RwLock<IconManager>>> = OnceLock::new();
);

pub struct IconManager {
    await_graphics: Vec<AwaitGraphics>,
    spell_icons: HashMap<u16, AtlasIcon>,
}

impl IconManager {
    pub fn get(spell: &Spell) -> Option<AtlasIcon> {
        ICON_MANAGER.get()
            .and_then(|manager| manager.read().ok())
            .and_then(|manager| manager.get_inner(spell))
    }
    
    fn get_inner(&self, spell: &Spell) -> Option<AtlasIcon> {
        self.spell_icons.get(&spell.icon_id()).cloned()
    }

    pub fn init() {
        if ICON_MANAGER.set(Arc::new(RwLock::new(Self::init_inner()))).is_ok() {
            tracing::info!("IconManager initialization finished");
        } else {
            tracing::error!("IconManager was already initialized");
        }
    }

    fn init_inner() -> Self {
        let mut await_graphics = vec![];

        if let Err(err) = vanilla_loader::load_spells(&mut await_graphics) {
            tracing::error!("Error loading vanilla spells: {err:?}");
        }

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
            if let Err(err) = modded_loader::load_spells(&mut await_graphics, &modded_spells_path) {
                tracing::error!("Error loading modded spells '{modded_spells_path:?}': {err:?}");
            }
        }

        Self {
            await_graphics,
            spell_icons: Default::default(),
        }
    }
    
    pub fn load(render_context: &mut dyn RenderContext) {
        tracing::info!("Loading spell icons...");
        ICON_MANAGER.get().expect("IconManager was never initialized")
            .write()
            .unwrap()
            .load_inner(render_context);
        tracing::info!("Finished loading spell icons");
    }
    
    fn load_inner(&mut self, render_context: &mut dyn RenderContext) {
        let start = Instant::now();
        for await_graphics in take(&mut self.await_graphics) {
            if let Err(err) = await_graphics(render_context, &mut self.spell_icons) {
                tracing::error!("Error loading icons: {err}");
            }
        }
        let time = start.elapsed();
        tracing::info!("Finished loading spells graphics in {time:?}");
    }
}