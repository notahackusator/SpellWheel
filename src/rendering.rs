use std::cmp::Ordering;
use std::ops::Deref;
use hudhook::{Hudhook, ImguiRenderLoop};
use imgui::Ui;
use lazy_static::lazy_static;
use std::sync::{Arc, RwLock};
use hudhook::hooks::dx12::ImguiDx12Hooks;
use hudhook::windows::Win32::Foundation::HINSTANCE;
use crate::{hmodule, set_selected_spell_index, Spell};

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
        f(&mut *SPELL_WHEEL_DATA.write().unwrap())
    }

    pub fn get<F: FnOnce(&Self) -> T, T>(f: F) -> T {
        f(&*SPELL_WHEEL_DATA.read().unwrap())
    }
}

pub struct SpellWheel {
    did_render: bool
}

impl SpellWheel {
    fn new() -> Self {
        Self {
            did_render: false
        }
    }

    fn switch_spell(spells: &[Spell], ui: &mut Ui) {
        let [sw, sh] = ui.io().display_size;
        let [mx, my] = ui.io().mouse_pos;

        let dx = mx - sw / 2.0;
        let dy = my - sh / 2.0;

        let mouse_angle = dy.atan2(dx);

        let mut min_spell_idx = 0;
        let mut min_dist_squared = f32::INFINITY;

        for (i, spell) in spells.iter().enumerate() {
            let angle = (i as f32 / spells.len() as f32) * std::f32::consts::TAU
                - std::f32::consts::FRAC_PI_2;

            let dist_squared = angle_dist(mouse_angle, angle);
            if dist_squared < min_dist_squared {
                min_dist_squared = dist_squared;
                min_spell_idx = spell.index;
            }
        }

        tracing::info!("Selecting spell at index: {}", min_spell_idx);
        set_selected_spell_index(min_spell_idx as i32);
    }
}

fn angle_dist(a: f32, b: f32) -> f32 {
    let a_cos = a.cos();
    let a_sin = a.sin();

    let b_cos = b.cos();
    let b_sin = b.sin();

    let dx = a_cos - b_cos;
    let dy = a_sin - b_sin;

    dx * dx + dy * dy
}

impl ImguiRenderLoop for SpellWheel {
    fn render(&mut self, ui: &mut Ui) {
        let do_render = SpellWheelData::get(|data| data.do_render);
        let mut spells = SpellWheelData::get(|data| data.spells.clone());

        if self.did_render && !do_render {
            tracing::info!("Switching spells");
            Self::switch_spell(&spells, ui);
        }

        // because imgui is stupid, we need to draw something to the screen, otherwise it will crash
        if !do_render {
            spells = vec![];
        }

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

                let [mx, my] = ui.io().mouse_pos;

                let dx = mx - sw / 2.0;
                let dy = my - sh / 2.0;

                let mouse_angle = dy.atan2(dx);

                let cx = wx + ww / 2.0;
                let cy = wy + wh / 2.0;
                let radius = ww.min(wh) / 2.0 - sw.min(sh) / 4.0; // padding from edge

                let mut min_spell_angle = 0.0;
                let mut min_dist_squared = f32::INFINITY;

                for (i, spell) in spells.iter().enumerate() {
                    let angle = (i as f32 / n as f32) * std::f32::consts::TAU
                        - std::f32::consts::FRAC_PI_2;

                    let dist_squared = angle_dist(mouse_angle, angle);
                    if dist_squared < min_dist_squared {
                        min_dist_squared = dist_squared;
                        min_spell_angle = angle;
                    }

                    let x = cx + angle.cos() * radius;
                    let y = cy + angle.sin() * radius;

                    let text_size = ui.calc_text_size(&spell.name);
                    draw_list.add_text(
                        [x - text_size[0] / 2.0, y - text_size[1] / 2.0],
                        ui.style_color(imgui::StyleColor::Text),
                        &spell.name,
                    );
                }

                if min_dist_squared == f32::INFINITY {
                    return;
                }
                draw_list.add_circle(
                    [cx + min_spell_angle.cos() * radius, cy + min_spell_angle.sin() * radius],
                    sw.min(sh) / 10.0,
                    [1.0, 1.0, 1.0, 0.3]
                ).filled(true).build();
            });

        self.did_render = do_render;
    }
}