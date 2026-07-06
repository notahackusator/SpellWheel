use std::collections::{HashMap, HashSet};
use std::mem::take;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::Instant;
use hudhook::RenderContext;
use lazy_static::lazy_static;
use crate::dynamic_icons::modded_reader;
use crate::icons::{modded_loader, vanilla_loader, AtlasIcon};
use crate::icons::await_graphics::AwaitGraphics;
use crate::settings::Settings;
use crate::items::Item;

lazy_static!(
    static ref ICON_MANAGER: OnceLock<Arc<RwLock<IconManager>>> = OnceLock::new();
);

pub struct IconManager {
    await_graphics: Vec<AwaitGraphics>,
    icons: HashMap<u16, AtlasIcon>,
}

impl IconManager {
    pub fn get(item: &Item) -> Option<AtlasIcon> {
        ICON_MANAGER.get()
            .and_then(|manager| manager.read().ok())
            .and_then(|manager| manager.get_inner(item))
    }
    
    fn get_inner(&self, item: &Item) -> Option<AtlasIcon> {
        self.icons.get(&item.icon_id()).cloned()
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

        if let Err(err) = vanilla_loader::load_icons(&mut await_graphics) {
            tracing::error!("Error loading vanilla icons: {err:?}");
        }

        let mut paths = HashSet::new();
        for modded_icons_path in Settings::read_or_default().modded_spells {
            if modded_icons_path.contains(".") {
                continue;
            }

            paths.insert(modded_icons_path.into());
        }
        if let Some(upstream_mod_folder) = modded_reader::search_for_mod_folder() {
            tracing::info!("Found upstream mod folder: {upstream_mod_folder:?}");
            paths.insert(upstream_mod_folder);
        }

        for modded_icons_path in paths {
            if let Err(err) = modded_loader::load_icons(&mut await_graphics, &modded_icons_path) {
                tracing::error!("Error loading modded icons '{modded_icons_path:?}': {err:?}");
            }
        }

        Self {
            await_graphics,
            icons: Default::default(),
        }
    }
    
    pub fn load(render_context: &mut dyn RenderContext) {
        tracing::info!("Loading icons...");
        ICON_MANAGER.get().expect("IconManager was never initialized")
            .write()
            .unwrap()
            .load_inner(render_context);
        tracing::info!("Finished loading icons");
    }
    
    fn load_inner(&mut self, render_context: &mut dyn RenderContext) {
        let start = Instant::now();
        for await_graphics in take(&mut self.await_graphics) {
            if let Err(err) = await_graphics(render_context, &mut self.icons) {
                tracing::error!("Error loading icons: {err}");
            }
        }
        let time = start.elapsed();
        tracing::info!("Finished loading icon graphics in {time:?}");
    }
}