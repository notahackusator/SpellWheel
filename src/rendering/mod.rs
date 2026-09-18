use crate::debugging::{add_to_screen_debug, is_debugging};
use crate::font::FontId;
use crate::glyphs::font_manager::FontManager;
use crate::hwindow::get_window_size;
use crate::icons::icon_manager::IconManager;
use crate::mouse::reset_cursor_pos;
use crate::rendering::wheel_hook::ItemWheelData;
use crate::rendering::wheel_renderer::render_wheel;
use crate::settings::Settings;
use crate::{Item, guard, hmodule, set_hwnd, set_selected_quick_item_index, set_selected_spell_index};
use display_item::DisplayItem;
use hudhook::hooks::dx12::ImguiDx12Hooks;
use hudhook::windows::Win32::Foundation::HINSTANCE;
use hudhook::{Hudhook, ImguiRenderLoop, RenderContext};
use imgui::{Context, Ui, WindowFlags};
use lazy_static::lazy_static;
use std::sync::{Arc, RwLock};
use wheel_hook::ItemWheel;
use wheel_renderer::WheelType;

pub mod display_item;
pub mod wheel_renderer;
pub mod debug_renderer;
mod wrapped_text;
pub mod wheel_hook;

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
    tracing::info!("Init rendering complete");
}

pub fn remove_hudhook() {
    hudhook::eject();
}

lazy_static!(
    static ref ITEM_WHEEL_DATA: Arc<RwLock<ItemWheelData>> = Arc::new(RwLock::new(ItemWheelData::new()));
);