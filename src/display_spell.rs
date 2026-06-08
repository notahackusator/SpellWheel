use crate::debugging::{add_to_screen_debug, is_debugging, read_committed_screen_debug};
use crate::icons::IconManager;
use crate::settings::Settings;
use crate::spells::Spell;
use crate::gamepad_state;
use imgui::{DrawListMut, TextureId, Ui};

pub struct DisplaySpell {
    pub index: i32,
    pub texture_id: Option<TextureId>,
    pub spell_name: WrappedText,
    pub is_highlighted: bool,
    pub angle: f32,
    pub pos: [f32; 2],
    pub img_c1: [f32; 2],
    pub img_c2: [f32; 2],
    pub text_pos: [f32; 2],
    pub rect_c1: [f32; 2],
    pub rect_c2: [f32; 2],
    pub thickness: f32,
    pub cos_sin: [f32; 2],
}

impl DisplaySpell {
    pub fn dist(&self, cos: f32, sin: f32) -> f32 {
        let dx = cos - self.cos_sin[0];
        let dy = sin - self.cos_sin[1];
        dx * dx + dy * dy
    }

    // Ok = controller pos
    // Err = cursor pos
    pub fn angle_and_dist_sqr(ui: &Ui, pos: Result<[f32; 2], [f32; 2]>) -> (f32, f32) {
        let [screen_w, screen_h] = ui.io().display_size;
        let [dx, dy] = match pos {
            Ok([x, y]) => [x * screen_w / 2.0, - y * screen_h / 2.0],
            Err([x, y]) => [x - screen_w / 2.0, y - screen_h / 2.0]
        };
        let angle = dy.atan2(dx);
        let dist_sqr = dx * dx + dy * dy;

        (angle, dist_sqr)
    }

    pub fn closest(spells: &[DisplaySpell], angle: f32) -> Option<usize> {
        spells.first().map(|first| {
            let [cos, sin] = [angle.cos(), angle.sin()];

            let mut min = 0;
            let mut min_dist = first.dist(cos, sin);
            for (i, spell) in spells.iter().enumerate().skip(1) {
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

    pub fn from_spells(ui: &Ui, spells: &[Spell]) -> Vec<DisplaySpell> {
        let n = spells.len();
        if n == 0 {
            return vec![];
        }

        let [screen_w, screen_h] = ui.io().display_size;

        let cx = screen_w / 2.0;
        let cy = screen_h / 2.0;
        let radius = Settings::read_or_default().radius_multiplier * screen_w.min(screen_h);

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

                let spell_name = WrappedText::new(ui, img_dim, spell.name());
                let [text_w, text_h] = [img_dim, spell_name.line_height * spell_name.lines.len() as f32];

                let img_c1 = [
                    x - img_dim / 2.0,
                    y - (img_dim + text_h) / 2.0
                ];
                let img_c2 = [
                    img_c1[0] + img_dim,
                    img_c1[1] + img_dim,
                ];

                let text_pos = [
                    x,
                    img_c2[1],
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

                let thickness = ((max_dx * 2.0).powi(2) + (max_dy * 2.0).powi(2)).sqrt();

                let index = spell.index();
                let texture_id = IconManager::get(spell.id());
                let is_highlighted = false;

                DisplaySpell {
                    index,
                    texture_id,
                    spell_name,
                    is_highlighted,
                    angle,
                    pos,
                    img_c1,
                    img_c2,
                    text_pos,
                    rect_c1,
                    rect_c2,
                    thickness,
                    cos_sin,
                }
            })
            .collect()
    }

    pub fn draw_all(spells: &mut [DisplaySpell], ui: &Ui, draw_list: &DrawListMut) {
        Self::draw_debug(ui, draw_list);
        if spells.is_empty() {
            return;
        }
        let settings = Settings::read_or_default();

        let (angle, dist_sqr) = Self::angle_and_dist_sqr(ui, match settings.using_controller {
            true => Ok(gamepad_state().right_stick),
            false => Err(ui.io().mouse_pos),
        });

        let [screen_w, screen_h] = ui.io().display_size;
        let min_radius_sqr = (
            settings.min_radius * settings.radius_multiplier * screen_w.min(screen_h)
        ).powi(2);

        // Only select closest IF far enough away from the center
        let can_select = dist_sqr >= min_radius_sqr;
        if can_select {
            for spell in spells.iter_mut() {
                spell.is_highlighted = false;
            }
            if let Some(closest_idx) = Self::closest(spells, angle) {
                if is_debugging() {
                    add_to_screen_debug(format!("Closest spell index: {closest_idx}"))
                }
                spells[closest_idx].is_highlighted = true;
            } else if is_debugging() {
                add_to_screen_debug("No spell highlighted, but within range.".to_string());
            }
        }

        Self::draw_selector(ui, draw_list, angle, can_select);

        for spell in spells.iter() {
            spell.draw(spells.len(), ui, draw_list);
        }
    }

    pub fn draw_selector(ui: &Ui, draw_list: &DrawListMut, angle: f32, can_select: bool) {
        let settings = Settings::read_or_default();
        if !settings.using_controller || !can_select {
            return;
        }

        let [screen_w, screen_h] = ui.io().display_size;
        let thickness = screen_w.min(screen_h) / 200.0;
        let radius = settings.radius_multiplier * screen_w.min(screen_h) - img_dim(ui) - thickness * 2.0;

        let [cx, cy] = [screen_w / 2.0, screen_h / 2.0];

        let bezier = Self::arc_bezier(
            cx, cy, radius, angle - 0.125 * std::f32::consts::TAU, angle + 0.125 * std::f32::consts::TAU
        );
        draw_list.add_bezier_curve(bezier[0], bezier[1], bezier[2], bezier[3], [1.0; 4]).thickness(thickness).build();

        let triangle_center_base_radius = radius + thickness;
        let [triangle_cx, triangle_cy] = [
            cx + triangle_center_base_radius * angle.cos(),
            cy + triangle_center_base_radius * angle.sin()
        ];

        let circle_third = 2.0 * std::f32::consts::FRAC_PI_3;
        draw_list.add_triangle(
            [
                triangle_cx + thickness * angle.cos(),
                triangle_cy + thickness * angle.sin()
            ],
            [
                triangle_cx + thickness * (angle + circle_third).cos(),
                triangle_cy + thickness * (angle + circle_third).sin()
            ],
            [
                triangle_cx + thickness * (angle + 2.0 * circle_third).cos(),
                triangle_cy + thickness * (angle + 2.0 * circle_third).sin()
            ],
            [1.0; 4]
        ).filled(true).build();
    }

    pub fn arc_bezier(cx: f32, cy: f32, radius: f32, start_angle: f32, end_angle: f32) -> [[f32; 2]; 4] {
        let theta = end_angle - start_angle;

        let k = (4.0 / 3.0) * (theta / 4.0).tan();

        let (s0, c0) = start_angle.sin_cos();
        let (s1, c1) = end_angle.sin_cos();

        let p0 = [
            cx + radius * c0,
            cy + radius * s0,
        ];

        let p3 = [
            cx + radius * c1,
            cy + radius * s1,
        ];

        // Tangent vectors
        let t0 = [-s0, c0];
        let t1 = [-s1, c1];

        let p1 = [
            p0[0] + radius * k * t0[0],
            p0[1] + radius * k * t0[1],
        ];

        let p2 = [
            p3[0] - radius * k * t1[0],
            p3[1] - radius * k * t1[1],
        ];

        [p0, p1, p2, p3]
    }

    pub fn draw(&self, num_spells: usize, ui: &Ui, draw_list: &DrawListMut) {
        let settings = Settings::read_or_default();
        if settings.using_controller {
            let [screen_w, screen_h] = ui.io().display_size;

            let [cx, cy] = [screen_w / 2.0, screen_h / 2.0];

            let thickness = screen_w.min(screen_h) / 200.0;

            let radius = settings.radius_multiplier * screen_w.min(screen_h) - img_dim(ui);

            let angle_offset = std::f32::consts::PI / num_spells as f32 - (thickness / radius).atan();
            if is_debugging() {
                add_to_screen_debug(format!("{angle_offset} {thickness} {radius} {}", (thickness / radius).atan()));
            }

            let bezier = Self::arc_bezier(
                cx, cy, radius, self.angle - angle_offset, self.angle + angle_offset
            );
            draw_list.add_bezier_curve(bezier[0], bezier[1], bezier[2], bezier[3], [1.0; 4]).thickness(thickness).build();
        }
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
        self.spell_name.add_to_draw_list(draw_list, self.text_pos, ui.style_color(imgui::StyleColor::Text), true);
    }

    pub fn draw_debug(ui: &Ui, draw_list: &DrawListMut) {
        let mut pos = [0.0; 2];
        for str in read_committed_screen_debug() {
            draw_list.add_text(
                pos,
                ui.style_color(imgui::StyleColor::Text),
                &str,
            );
            pos[1] += ui.text_line_height();
        }
    }
}

pub struct WrappedText {
    lines: Vec<String>,
    widths: Vec<f32>,
    line_height: f32,
}

impl WrappedText {
    pub fn new<S: AsRef<str>>(ui: &Ui, max_width: f32, value: S) -> Self {
        let mut lines = Vec::new();
        let mut widths = Vec::new();
        let mut line_height = 0.0;

        for line in value.as_ref().lines() {
            let mut current_line = String::new();

            for word in line.split_whitespace() {
                let candidate = if current_line.is_empty() {
                    word.to_string()
                } else {
                    format!("{} {}", current_line, word)
                };

                let [width, this_line_height] = ui.calc_text_size(&candidate);
                line_height = this_line_height;

                if width > max_width && !current_line.is_empty() {
                    let [width, _] = ui.calc_text_size(&current_line);
                    lines.push(current_line);
                    widths.push(width);

                    current_line = word.to_string();
                } else {
                    current_line = candidate;
                }
            }

            let [width, _] = ui.calc_text_size(&current_line);
            lines.push(current_line);
            widths.push(width);
        }

        Self {
            lines,
            widths,
            line_height,
        }
    }

    pub fn add_to_draw_list(&self, draw_list: &DrawListMut, pos: [f32; 2], color: [f32; 4], centered: bool) {
        for (i, (line, width)) in self.lines.iter().zip(self.widths.iter()).enumerate() {
            let x = if centered {
                pos[0] - width / 2.0
            } else {
                pos[0]
            };
            draw_list.add_text(
                [x, pos[1] + i as f32 * self.line_height],
                color,
                line,
            );
        }
    }
}

fn img_dim(ui: &Ui) -> f32 {
    let [ww, wh] = ui.io().display_size;
    Settings::read_or_default().icon_scale_multiplier * ww.min(wh)
}