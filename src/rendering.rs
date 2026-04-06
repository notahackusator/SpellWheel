use std::fs::read;
use std::mem;
use hudhook::{Hudhook, ImguiRenderLoop, RenderContext};
use imgui::{Context, DrawListMut, FontSource, TextureId, Ui};
use lazy_static::lazy_static;
use std::sync::{Arc, RwLock};
use hudhook::hooks::dx12::ImguiDx12Hooks;
use hudhook::windows::Win32::Foundation::HINSTANCE;
use crate::{hmodule, paths, set_selected_spell_index, Spell};
use crate::icons::IconManager;

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
        f(&mut *SPELL_WHEEL_DATA.write().unwrap())
    }

    pub fn get<F: FnOnce(&Self) -> T, T>(f: F) -> T {
        f(&*SPELL_WHEEL_DATA.read().unwrap())
    }
}

struct DisplaySpell {
    index: i32,
    texture_id: Option<TextureId>,
    spell_name: String,
    is_highlighted: bool,
    pos: [f32; 2],
    img_c1: [f32; 2],
    img_c2: [f32; 2],
    text_c1: [f32; 2],
    text_c2: [f32; 2],
    rect_c1: [f32; 2],
    rect_c2: [f32; 2],
    cos_sin: [f32; 2],
}

impl DisplaySpell {
    fn dist(&self, cos: f32, sin: f32) -> f32 {
        let dx = cos - self.cos_sin[0];
        let dy = sin - self.cos_sin[1];
        dx * dx + dy * dy
    }

    fn closest(spells: &[DisplaySpell], angle: f32) -> Option<usize> {
        spells.first().map(|first| {
            let [cos, sin] = [angle.cos(), angle.sin()];

            let mut min = 0;
            let mut min_dist = first.dist(cos, sin);
            for i in 1..spells.len() {
                let spell = &spells[i];
                let dist = spell.dist(cos, sin);

                if dist > min_dist {
                    continue;
                }

                min = i;
                min_dist = dist;
            }

            min
        })
    }

    fn from_spells(ui: &Ui, spells: &[Spell]) -> Vec<DisplaySpell> {
        let n = spells.len();
        if n == 0 {
            return vec![];
        }

        let [screen_w, screen_h] = ui.io().display_size;

        let cx = screen_w / 2.0;
        let cy = screen_h / 2.0;
        let radius = screen_w.min(screen_h) / 4.0;

        let img_dim = img_dim(ui);

        spells.iter().enumerate()
            .map(|(i, spell)| {
                let angle = (i as f32 / n as f32) * std::f32::consts::TAU
                    - std::f32::consts::FRAC_PI_2;

                let cos_sin @ [cos, sin] = [
                    angle.cos(),
                    angle.sin()
                ];

                let pos @ [x, y] = [
                    cx + cos * radius,
                    cy + sin * radius
                ];

                let [text_w, text_h] = ui.calc_text_size(spell.name());

                let img_c1 = [
                    x - img_dim / 2.0,
                    y - (img_dim + text_h) / 2.0
                ];
                let img_c2 = [
                    img_c1[0] + img_dim,
                    img_c1[1] + img_dim,
                ];

                let text_c1 = [
                    x - text_w / 2.0,
                    img_c2[1],
                ];
                let text_c2 = [
                    text_c1[0] + text_w,
                    text_c1[1] + text_h,
                ];

                let max_dx = (text_w / 2.0).max(img_dim / 2.0) + 10.0;
                let max_dy = (img_dim + text_h) / 2.0 + 10.0;
                let rect_c1 = [
                    x - max_dx,
                    y - max_dy
                ];

                let rect_c2 = [
                    x + max_dx,
                    y + max_dy
                ];

                let index = spell.index();
                let texture_id = IconManager::get(spell.id());
                let spell_name = spell.name().to_string();
                let is_highlighted = false;

                DisplaySpell {
                    index,
                    texture_id,
                    spell_name,
                    is_highlighted,
                    pos,
                    img_c1,
                    img_c2,
                    text_c1,
                    text_c2,
                    rect_c1,
                    rect_c2,
                    cos_sin,
                }
            }).collect()
    }

    fn draw_all(spells: &mut [DisplaySpell], ui: &Ui, draw_list: &DrawListMut) {
        let [mx, my] = ui.io().mouse_pos;
        let [sw, sh] = ui.io().display_size;
        let dx = mx - sw / 2.0;
        let dy = my - sh / 2.0;
        let mouse_angle = dy.atan2(dx);

        for spell in spells.iter_mut() {
            spell.is_highlighted = false;
        }
        Self::closest(spells, mouse_angle).map(|idx| spells[idx].is_highlighted = true);

        for spell in spells.iter() {
            spell.draw(ui, draw_list);
        }
    }

    fn draw(&self, ui: &Ui, draw_list: &DrawListMut) {
        if self.is_highlighted {
            draw_list.add_rect(
                self.rect_c1,
                self.rect_c2,
                [1.0, 1.0, 1.0, 0.2]
            ).filled(true).rounding(10.0).build();
        }
        match self.texture_id {
            Some(texture_id) => draw_list.add_image(
                texture_id,
                self.img_c1,
                self.img_c2
            ).build(),
            None => draw_list.add_rect(
                self.img_c1,
                self.img_c2,
                [0.5, 0.5, 0.5, 1.0]
            ).build()
        }
        draw_list.add_text(
            self.text_c1,
            ui.style_color(imgui::StyleColor::Text),
            &self.spell_name,
        );
    }
}

fn img_dim(ui: &Ui) -> f32 {
    let [ww, wh] = ui.io().display_size;
    ww.min(wh) * IMG_DIM_MULTIPLIER
}

const IMG_DIM_MULTIPLIER: f32 = 0.15;

pub struct SpellWheel {
    font: Option<usize>,
    display_spells: Vec<DisplaySpell>,
    did_render: bool,
    prev_size: Option<[f32; 2]>,
}

impl SpellWheel {
    fn new() -> Self {
        Self {
            font: None,
            display_spells: vec![],
            did_render: false,
            prev_size: None,
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
        let global_scale = match self.prev_size {
            Some(prev_size) if prev_size == window_size => ctx.io().font_global_scale,
            _ => ww.min(wh) / DEFAULT_SCREEN_MIN
        };
        ctx.io_mut().font_global_scale = global_scale;
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
        self.try_resize_font(ctx);
    }

    fn render(&mut self, ui: &mut Ui) {
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

        self.display_spells = DisplaySpell::from_spells(&ui, &spells);

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
                DisplaySpell::draw_all(&mut self.display_spells, &ui, &draw_list);
            });

        self.did_render = do_render;
        if let Some(font) = font {
            font.pop();
        }
    }
}