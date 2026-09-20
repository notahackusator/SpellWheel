use crate::rendering::wheel_hook::ItemWheelData;
use crate::hmodule;
use hudhook::hooks::dx12::ImguiDx12Hooks;
use hudhook::windows::Win32::Foundation::HINSTANCE;
use hudhook::Hudhook;
use lazy_static::lazy_static;
use std::sync::{Arc, RwLock};
use wheel_hook::ItemWheel;

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