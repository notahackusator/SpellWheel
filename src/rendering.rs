use std::fs::read;
use std::mem;
use hudhook::{Hudhook, ImguiRenderLoop, RenderContext};
use imgui::{Context, FontSource, Ui};
use lazy_static::lazy_static;
use std::sync::{Arc, RwLock};
use hudhook::hooks::dx12::ImguiDx12Hooks;
use hudhook::windows::Win32::Foundation::HINSTANCE;
use crate::{guard, hmodule, paths, set_selected_spell_index, Spell};
use crate::display_spell::DisplaySpell;
use crate::icons::IconManager;
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
        .with::<ImguiDx12Hooks>(SpellWheel::new())
        .with_hmodule(HINSTANCE(hmodule() as _))
        .build()
        .apply()
    {
        tracing::error!("Couldn't apply hooks: {e:?}");
        hudhook::eject();
    }
}

lazy_static!(
    static ref SPELL_WHEEL_DATA: Arc<RwLock<SpellWheelData>> = Arc::new(RwLock::new(SpellWheelData::new()));
);

pub struct SpellWheelData {
    pub spells: Vec<Spell>,
    pub do_render: bool,
}

impl SpellWheelData {
    fn new() -> Self {
        Self {
            spells: vec![],
            do_render: false,
        }
    }

    pub fn mutate<F: FnOnce(&mut Self)>(f: F) {
        f(&mut SPELL_WHEEL_DATA.write().unwrap())
    }

    pub fn get<F: FnOnce(&Self) -> T, T>(f: F) -> T {
        f(&SPELL_WHEEL_DATA.read().unwrap())
    }
}

pub struct SpellWheel {
    font: Option<usize>,
    display_spells: Vec<DisplaySpell>,
    did_render: bool,
    prev_size: Option<[f32; 2]>,
    prev_spells: Vec<Spell>,
}

impl SpellWheel {
    fn new() -> Self {
        Self {
            font: None,
            display_spells: vec![],
            did_render: false,
            prev_size: None,
            prev_spells: vec![],
        }
    }

    fn switch_spell(&self) {
        if let Some(spell) = self.display_spells.iter()
            .find(|spell| spell.is_highlighted) {

            tracing::info!("Selecting spell at index: {}", spell.index);
            set_selected_spell_index(spell.index);
        }
    }
}

const DEFAULT_FONT_HEIGHT: f32 = 54.0;
const DEFAULT_SCREEN_MIN: f32 = 2160.0;

impl SpellWheel {
    fn try_resize_font(&mut self, ctx: &mut Context) {
        let window_size @ [ww, wh] = ctx.io().display_size;
        ctx.io_mut().font_global_scale = Settings::read_or_default().font_scale_multiplier * ww.min(wh) / DEFAULT_SCREEN_MIN;
        self.prev_size = Some(window_size);
    }
}

impl ImguiRenderLoop for SpellWheel {
    fn initialize<'a>(&'a mut self, ctx: &mut Context, render_context: &'a mut dyn RenderContext) {
        tracing::info!("Initializing spell wheel UI");

        tracing::info!("Loading font...");

        self.font = read(paths::font()).map(|font_data| unsafe {
            mem::transmute(ctx.fonts().add_font(&[FontSource::TtfData {
                data: &font_data,
                size_pixels: DEFAULT_FONT_HEIGHT,
                config: None
            }]))
        }).ok();
        tracing::info!("Font loaded");
        IconManager::load(render_context);
    }

    fn before_render<'a>(&'a mut self, ctx: &mut Context, _render_context: &'a mut dyn RenderContext) {
        guard!(
            self.try_resize_font(ctx);
        );
    }

    fn render(&mut self, ui: &mut Ui) {
        guard!(
            let font = self.font.map(|font| unsafe { ui.push_font(mem::transmute(font)) });
            let do_render = SpellWheelData::get(|data| data.do_render);
            let mut spells = SpellWheelData::get(|data| data.spells.clone());

            if self.did_render && !do_render {
                tracing::info!("Switching spells");
                self.switch_spell();
            }

            // because imgui is stupid, we need to draw something to the screen, otherwise it will crash
            if !do_render {
                spells = vec![];
            }

            if spells != self.prev_spells {
                self.display_spells = DisplaySpell::from_spells(ui, &spells);
            }
            self.prev_spells = spells;

            let [sw, sh] = ui.io().display_size;
            ui.window("Spell Wheel")
                .position([0.0, 0.0], imgui::Condition::Always)
                .size([sw, sh], imgui::Condition::Always)
                .bg_alpha(0.0)
                .no_decoration()
                .no_inputs()
                .movable(false)
                .build(|| {
                    let draw_list = ui.get_window_draw_list();
                    DisplaySpell::draw_all(&mut self.display_spells, ui, &draw_list);
                });

            self.did_render = do_render;
            if let Some(font) = font {
                font.pop();
            }
        );
    }
}