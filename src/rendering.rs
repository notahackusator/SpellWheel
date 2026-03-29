use hudhook::{Hudhook, ImguiRenderLoop};
use imgui::Ui;
use lazy_static::lazy_static;
use std::sync::{Arc, RwLock};
use hudhook::hooks::dx12::ImguiDx12Hooks;
use hudhook::windows::Win32::Foundation::HINSTANCE;
use crate::{hmodule, Spell};

static mut INIT: bool = false;
pub fn try_init_rendering() {
    if unsafe { INIT } {
        return;
    }
    unsafe {
        INIT = true;
    }
    tracing::info!("Init rendering called");
    if let Err(e) = Hudhook::builder()
        .with::<ImguiDx12Hooks>(SpellWheel)
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
        f(&mut *SPELL_WHEEL_DATA.write().unwrap())
    }

    pub fn get<F: FnOnce(&Self) -> T, T>(f: F) -> T {
        f(&*SPELL_WHEEL_DATA.read().unwrap())
    }
}

pub struct SpellWheel;

impl ImguiRenderLoop for SpellWheel {
    fn render(&mut self, ui: &mut Ui) {
        let do_render = SpellWheelData::get(|data| data.do_render);
        let spells = match do_render {
            true => SpellWheelData::get(|data| data.spells.clone()),
            false => vec![], // because imgui is stupid, we need to draw something to the screen, otherwise it will crash
        };

        let [sw, sh] = ui.io().display_size;

        ui.window("Spell Wheel")
            .position([0.0, 0.0], imgui::Condition::Always)
            .size([sw, sh], imgui::Condition::Always)
            .bg_alpha(0.0)
            .no_decoration()
            .no_inputs()
            .movable(false)
            .build(|| {
                let n = spells.len();
                if n == 0 {
                    return;
                }

                let draw_list = ui.get_window_draw_list();
                let [wx, wy] = ui.window_pos();
                let [ww, wh] = ui.window_size();

                let cx = wx + ww / 2.0;
                let cy = wy + wh / 2.0;
                let radius = ww.min(wh) / 2.0 - sw.min(sh) / 4.0; // padding from edge

                for (i, spell) in spells.iter().enumerate() {
                    let angle = (i as f32 / n as f32) * std::f32::consts::TAU
                        - std::f32::consts::FRAC_PI_2;

                    let x = cx + angle.cos() * radius;
                    let y = cy + angle.sin() * radius;

                    let text_size = ui.calc_text_size(&spell.name);
                    draw_list.add_text(
                        [x - text_size[0] / 2.0, y - text_size[1] / 2.0],
                        ui.style_color(imgui::StyleColor::Text),
                        &spell.name,
                    );
                }
            });
    }
}