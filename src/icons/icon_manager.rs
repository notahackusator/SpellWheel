use std::collections::{HashMap, HashSet};
use std::mem::take;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::Instant;
use hudhook::RenderContext;
use hudhook::windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_R8G8B8A8_UNORM;
use image::EncodableLayout;
use imgui::TextureId;
use lazy_static::lazy_static;
use crate::dynamic_icons::modded_reader;
use crate::icons::{modded_loader, vanilla_loader, AtlasIcon};
use crate::icons::await_graphics::AwaitGraphics;
use crate::settings::Settings;
use crate::items::Item;

lazy_static!(
    static ref ICON_MANAGER: OnceLock<Arc<RwLock<IconManager>>> = OnceLock::new();
);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StaticIcons {
    pub item_bg: TextureId,
    pub selected_item_bg: TextureId,
}

impl StaticIcons {
    pub const fn new() -> Self {
        Self {
            item_bg: TextureId::new(0),
            selected_item_bg: TextureId::new(0),
        }
    }
}

pub struct IconManager {
    await_graphics: Vec<AwaitGraphics>,
    static_icons: StaticIcons,
    icons: HashMap<u16, AtlasIcon>,
}

impl IconManager {
    pub fn get_static_icons() -> Option<StaticIcons> {
        ICON_MANAGER.get()
            .and_then(|manager| manager.read().ok())
            .map(|manager| manager.static_icons)
    }

    pub fn get_item(item: &Item) -> Option<AtlasIcon> {
        ICON_MANAGER.get()
            .and_then(|manager| manager.read().ok())
            .and_then(|manager| manager.get_item_inner(item))
    }
    
    fn get_item_inner(&self, item: &Item) -> Option<AtlasIcon> {
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
        #[cfg(feature = "atlas-dump")]
        crate::icons::atlas_dump::delete_previous_dumps();

        let mut await_graphics = vec![];

        if let Err(err) = vanilla_loader::load_icons(&mut await_graphics) {
            tracing::error!("Error loading vanilla icons: {err:?}");
        }

        let mut paths = HashSet::new();
        for modded_icons_path in Settings::read_or_default().mods() {
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
            static_icons: StaticIcons::new(),
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

        #[cfg(feature = "atlas-dump")]
        crate::icons::atlas_dump::dump_icon_data();
    }
    
    fn load_inner(&mut self, render_context: &mut dyn RenderContext) {
        let start = Instant::now();
        for await_graphics in take(&mut self.await_graphics) {
            if let Err(err) = await_graphics(render_context, &mut self.icons) {
                tracing::error!("Error loading icons: {err}");
            }
        }
        if let Err(err) = self.load_static_icons(render_context) {
            tracing::error!("Error loading item background: {err}");
        }
        let time = start.elapsed();
        tracing::info!("Finished loading icon graphics in {time:?}");
    }

    fn load_static_icons(&mut self, render_context: &mut dyn RenderContext) -> anyhow::Result<()> {
        let item_bg = image::load_from_memory(include_bytes!("../../assets/item_bg.png"))?
            .to_rgba8();
        self.static_icons.item_bg = render_context.load_texture(
            DXGI_FORMAT_R8G8B8A8_UNORM, item_bg.as_bytes(), item_bg.width(), item_bg.height()
        )?;

        let selected_item_bg = image::load_from_memory(include_bytes!("../../assets/selected_item_bg.png"))?
            .to_rgba8();
        self.static_icons.selected_item_bg = render_context.load_texture(
            DXGI_FORMAT_R8G8B8A8_UNORM, selected_item_bg.as_bytes(), selected_item_bg.width(), selected_item_bg.height()
        )?;

        Ok(())
    }
}