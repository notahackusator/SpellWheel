use std::mem;
use hudhook::{Hudhook, ImguiRenderLoop, RenderContext};
use imgui::{Context, FontSource, Ui, WindowFlags};
use lazy_static::lazy_static;
use std::sync::{Arc, RwLock};
use hudhook::hooks::dx12::ImguiDx12Hooks;
use hudhook::windows::Win32::Foundation::HINSTANCE;
use crate::{guard, hmodule, set_selected_quick_item_index, set_selected_spell_index, Item, HWND};
use crate::debugging::{add_to_screen_debug, is_debugging};
use crate::display_item::DisplayItem;
use crate::font::{create_font_sources, FontId};
use crate::hwindow::{get_process_window, get_window_size};
use crate::icons::icon_manager::IconManager;
use crate::settings::Settings;

static mut INIT: bool = false;
pub fn try_init_rendering() {
    unsafe {
        if INIT {
            return;
        }
        INIT = true;
    }
    tracing::info!("Init rendering called");
    if let Err(e) = Hudhook::builder()
        .with::<ImguiDx12Hooks>(ItemWheel::new())
        .with_hmodule(HINSTANCE(hmodule() as _))
        .build()
        .apply()
    {
        tracing::error!("Couldn't apply hooks: {e:?}");
        hudhook::eject();
    }
}

pub fn remove_hudhook() {
    hudhook::eject();
}

lazy_static!(
    static ref ITEM_WHEEL_DATA: Arc<RwLock<ItemWheelData >> = Arc::new(RwLock::new(ItemWheelData::new()));
);

pub struct ItemWheelData {
    pub spells: Vec<Item>,
    pub quick_items: Vec<Item>,
    pub wheel_type: WheelType,
}

impl ItemWheelData {
    fn new() -> Self {
        Self {
            spells: vec![],
            quick_items: vec![],
            wheel_type: WheelType::None,
        }
    }

    pub fn mutate<F: FnOnce(&mut Self)>(f: F) {
        f(&mut ITEM_WHEEL_DATA.write().unwrap())
    }

    pub fn get<F: FnOnce(&Self) -> T, T>(f: F) -> T {
        f(&ITEM_WHEEL_DATA.read().unwrap())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WheelType {
    Spells,
    QuickItems,
    None
}

pub struct ItemWheel {
    font_bytes: Vec<Vec<u8>>,
    font: FontId,
    display_spells: Vec<DisplayItem>,
    display_quick_items: Vec<DisplayItem>,
    prev_type: WheelType,
    prev_spells: Vec<Item>,
    prev_quick_items: Vec<Item>,
}

impl ItemWheel {
    fn new() -> Self {
        Self {
            font_bytes: vec![],
            font: FontId::none(),
            display_spells: vec![],
            display_quick_items: vec![],
            prev_type: WheelType::None,
            prev_spells: vec![],
            prev_quick_items: vec![],
        }
    }

    fn switch_item(&self) {
        match self.prev_type {
            WheelType::Spells => {
                if let Some(item) = self.display_spells.iter()
                    .find(|item| item.is_highlighted) {

                    set_selected_spell_index(item.index);
                }
            }
            WheelType::QuickItems => {
                if let Some(item) = self.display_quick_items.iter()
                    .find(|item| item.is_highlighted) {

                    set_selected_quick_item_index(item.index);
                }
            }
            WheelType::None => {}
        }
    }
}

const DEFAULT_SCREEN_MIN: f32 = 2160.0;

impl ItemWheel {
    fn try_resize_font(&mut self, ctx: &mut Context) {
        let [ww, wh] = get_window_size();
        ctx.io_mut().font_global_scale = Settings::read_or_default().font_scale_multiplier * ww.min(wh) / DEFAULT_SCREEN_MIN;
    }
}

impl ImguiRenderLoop for ItemWheel {
    fn initialize<'a>(&'a mut self, ctx: &mut Context, render_context: &'a mut dyn RenderContext) {
        guard!(
            tracing::info!("Initializing item wheel UI");

            tracing::info!("Setting HWND...");
            HWND.set(unsafe { mem::transmute(get_process_window().expect("Could not find HWND")) }).expect("Count not set HWND");
            tracing::info!("Set HWND");

            tracing::info!("Loading font...");

            create_font_sources!(font_bytes, font_data; then: {
                self.font = ctx.fonts().add_font(&font_data).into();
                self.font_bytes = font_bytes;
            });
            tracing::info!("Font loaded");
            IconManager::load(render_context);
        );
    }

    fn before_render<'a>(&'a mut self, ctx: &mut Context, _render_context: &'a mut dyn RenderContext) {
        guard!(
            self.try_resize_font(ctx);
        );
    }

    fn render(&mut self, ui: &mut Ui) {
        guard!(
            let font = ui.push_font(self.font.into());
            let (wheel_type, quick_items, spells) = ItemWheelData::get(|data|
                (data.wheel_type, data.quick_items.clone(), data.spells.clone())
            );

            let switch_items = Settings::read_or_default().switch_instantly || (self.prev_type != wheel_type);
            if switch_items {
                self.switch_item();
            }

            let [sw, sh] = ui.io().display_size;
            ui.window("Item Wheel")
                .position([0.0, 0.0], imgui::Condition::Always)
                .size([sw, sh], imgui::Condition::Always)
                .flags(
                    WindowFlags::NO_TITLE_BAR |
                    WindowFlags::NO_RESIZE |
                    WindowFlags::NO_SCROLLBAR |
                    WindowFlags::NO_SCROLL_WITH_MOUSE |
                    WindowFlags::NO_BACKGROUND
                )
                .bg_alpha(0.0)
                .no_decoration()
                .no_inputs()
                .movable(false)
                .build(|| {
                    if self.prev_spells != spells {
                        if is_debugging() {
                            tracing::info!("Remaking display spells");
                        }
                        self.display_spells = DisplayItem::from_items(ui, &spells);
                    }
                    if self.prev_quick_items != quick_items {
                        if is_debugging() {
                            tracing::info!("Remaking display quick items");
                        }
                        self.display_quick_items = DisplayItem::from_items(ui, &quick_items);
                    }

                    if is_debugging() {
                        add_to_screen_debug(format!("Spells: {:?}", spells));
                        add_to_screen_debug(format!("Quick items: {:?}", quick_items));
                        add_to_screen_debug(format!("Display spells: {:?}", self.display_spells));
                        add_to_screen_debug(format!("Display quick items: {:?}", self.display_quick_items));
                    }

                    let display_items = match wheel_type {
                        WheelType::Spells => &mut self.display_spells,
                        WheelType::QuickItems => &mut self.display_quick_items,
                        WheelType::None => &mut vec![],
                    };

                    if is_debugging() {
                        add_to_screen_debug(format!("Display items: {display_items:?}"));
                    }

                    self.prev_spells = spells;
                    self.prev_quick_items = quick_items;

                    let draw_list = ui.get_window_draw_list();
                    DisplayItem::draw_all(display_items, ui, &draw_list);
                });

            self.prev_type = wheel_type;
            font.pop();
        );
    }
}