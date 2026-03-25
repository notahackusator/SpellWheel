use hudhook::{Hudhook, ImguiRenderLoop};
use imgui::Ui;
use lazy_static::lazy_static;
use std::sync::{Arc, RwLock};
use hudhook::hooks::dx12::ImguiDx12Hooks;
use hudhook::windows::Win32::Foundation::HINSTANCE;
use crate::HMODULE;

pub fn init_rendering() {
    if let Err(e) = Hudhook::builder()
        .with::<ImguiDx12Hooks>(SpellWheel::instance())
        .with_hmodule(HINSTANCE(*HMODULE.get().unwrap() as _))
        .build()
        .apply()
    {
        tracing::error!("Couldn't apply hooks: {e:?}");
        hudhook::eject();
    }
}

lazy_static!(
    static ref SPELL_WHEEL: Arc<RwLock<SpellWheel>> = Arc::new(RwLock::new(SpellWheel::new()));
);

#[derive(Clone)]
pub struct SpellWheel {
    spell_names: Vec<String>,
}

impl SpellWheel {
    fn new() -> Self {
        Self {
            spell_names: vec![]
        }
    }

    pub fn instance() -> Self {
        SPELL_WHEEL.read().unwrap().clone()
    }

    pub fn set_spell_names(spell_names: Vec<String>) {
        SPELL_WHEEL.write().unwrap().spell_names = spell_names;
    }

    pub fn get_spell_names() -> Vec<String> {
        SPELL_WHEEL.read().unwrap().spell_names.clone()
    }
}

impl ImguiRenderLoop for SpellWheel {
    fn render(&mut self, ui: &mut Ui) {
        ui.window("Spell Wheel")
            .size([300.0, 100.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Spells:");
                for spell_name in Self::get_spell_names() {
                    ui.text(spell_name);
                }
            });
    }
}