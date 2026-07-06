use std::fmt::{Debug, Formatter};
use crate::debugging::{add_to_screen_debug, is_debugging, read_committed_screen_debug};
use crate::hwindow::get_window_size;
use crate::icons::icon_manager::IconManager;
use crate::icons::AtlasIcon;
use crate::mouse::get_mouse_state;
use crate::settings::{Settings, ItemNames};
use crate::items::Item;
use imgui::{DrawListMut, Ui};
use crate::gamepad::gamepad_state;

pub struct DisplayItem {
    pub index: i32,
    pub icon: Option<AtlasIcon>,
    pub name: WrappedText,
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

impl Debug for DisplayItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "DisplayItem{{index={}, angle={:.3}}}", self.index, self.angle)
    }
}

impl DisplayItem {
    pub fn dist(&self, cos: f32, sin: f32) -> f32 {
        let dx = cos - self.cos_sin[0];
        let dy = sin - self.cos_sin[1];
        dx * dx + dy * dy
    }

    // Ok = controller pos
    // Err = cursor pos
    pub fn angle_and_dist_sqr(ui: &Ui, pos: Result<[f32; 2], [f32; 2]>) -> (f32, f32) {
        if is_debugging() {
            add_to_screen_debug(format!("Window focused? = {}", ui.is_window_focused()));
            add_to_screen_debug(format!("Mouse / right stick pos: {pos:?}"));
            add_to_screen_debug(format!("Mouse state: {:?}", get_mouse_state()));
        }
        let [ww, wh] = get_window_size();
        let [dx, dy] = match pos {
            Ok([x, y]) => [x * ww / 2.0, - y * wh / 2.0],
            Err([x, y]) => [x - ww / 2.0, y - wh / 2.0]
        };
        let angle = dy.atan2(dx);
        let dist_sqr = dx * dx + dy * dy;

        (angle, dist_sqr)
    }

    pub fn closest(items: &[DisplayItem], angle: f32) -> Option<usize> {
        items.first().map(|first| {
            let [cos, sin] = [angle.cos(), angle.sin()];

            let mut min = 0;
            let mut min_dist = first.dist(cos, sin);
            for (i, item) in items.iter().enumerate().skip(1) {
                let dist = item.dist(cos, sin);

                if dist > min_dist {
                    continue;
                }

                min = i;
                min_dist = dist;
            }

            min
        })
    }

    pub fn from_items(ui: &Ui, items: &[Item]) -> Vec<DisplayItem> {
        let n = items.len();
        if n == 0 {
            return vec![];
        }

        let settings = Settings::read_or_default();

        let [ww, wh] = get_window_size();

        let cx = ww / 2.0;
        let cy = wh / 2.0;
        let radius = settings.radius_multiplier * ww.min(wh);

        let img_dim = img_dim();

        items.iter().enumerate()
            .map(|(i, item)| {
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

                let name = WrappedText::new(ui, img_dim, item.name());
                let [text_w, text_h] = match settings.item_names() {
                    ItemNames::Show => [img_dim, name.line_height * name.lines.len() as f32],
                    _ => [0.0; 2]
                };

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

                let index = item.index();
                let icon = IconManager::get(item);
                let is_highlighted = false;

                DisplayItem {
                    index,
                    icon,
                    name,
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

    pub fn draw_all(items: &mut [DisplayItem], ui: &Ui, draw_list: &DrawListMut) {
        Self::draw_debug(ui, draw_list);
        if items.is_empty() {
            return;
        }
        let settings = Settings::read_or_default();

        let (angle, dist_sqr) = Self::angle_and_dist_sqr(ui, match settings.using_controller {
            true => Ok(gamepad_state().right_stick),
            false => Err(get_mouse_state().mouse_pos()),
        });

        let [ww, wh] = get_window_size();
        let min_radius_sqr = (
            settings.min_radius * settings.radius_multiplier * ww.min(wh)
        ).powi(2);

        // Only select closest IF far enough away from the center
        let can_select = dist_sqr >= min_radius_sqr;
        if can_select {
            for item in items.iter_mut() {
                item.is_highlighted = false;
            }
            if let Some(closest_idx) = Self::closest(items, angle) {
                if is_debugging() {
                    add_to_screen_debug(format!("Closest item index: {closest_idx}"))
                }
                items[closest_idx].is_highlighted = true;
            } else if is_debugging() {
                add_to_screen_debug("No item highlighted, but within range.".to_string());
            }
        }

        let img_dim = img_dim();
        Self::draw_selector(&settings, ww, wh, img_dim, draw_list, angle, can_select);

        for item in items.iter() {
            item.draw(&settings, ww, wh, img_dim, items.len(), ui, draw_list);
        }
    }

    pub fn draw_selector(settings: &Settings, ww: f32, wh: f32, img_dim: f32, draw_list: &DrawListMut, angle: f32, can_select: bool) {
        if !settings.using_controller || !can_select {
            return;
        }

        let thickness = ww.min(wh) / 200.0;
        let radius = settings.radius_multiplier * ww.min(wh) - img_dim - thickness * 2.0;

        let [cx, cy] = [ww / 2.0, wh / 2.0];

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

    pub fn draw(&self, settings: &Settings, ww: f32, wh: f32, img_dim: f32, num_items: usize, ui: &Ui, draw_list: &DrawListMut) {
        let [cx, cy] = [ww / 2.0, wh / 2.0];

        if settings.using_controller {
            let thickness = ww.min(wh) / 200.0;

            let radius = settings.radius_multiplier * ww.min(wh) - img_dim;

            let angle_offset = std::f32::consts::PI / num_items as f32 - (thickness / radius).atan();
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

            if let ItemNames::Center = settings.item_names() {
                self.name.add_to_draw_list(draw_list, [cx, cy], ui.style_color(imgui::StyleColor::Text), true, settings.text_shadows);
            }
        }
        match self.icon {
            Some(AtlasIcon { texture_id, rect, .. }) => {
                let [x, y, w, h] = rect;
                draw_list.add_image(
                    texture_id,
                    self.img_c1,
                    self.img_c2,
                )
                    .uv_min([x, y])
                    .uv_max([x + w, y + h])
                    .build()
            },
            None => draw_list.add_rect(
                self.img_c1,
                self.img_c2,
                [0.5, 0.5, 0.5, 1.0]
            ).build()
        }
        if let ItemNames::Show = settings.item_names() {
            self.name.add_to_draw_list(draw_list, self.text_pos, ui.style_color(imgui::StyleColor::Text), true, settings.text_shadows);
        }
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

    pub fn add_to_draw_list(&self, draw_list: &DrawListMut, pos: [f32; 2], color: [f32; 4], centered: bool, shadow: bool) {
        for (i, (line, width)) in self.lines.iter().zip(self.widths.iter()).enumerate() {
            let x = if centered {
                pos[0] - width / 2.0
            } else {
                pos[0]
            };
            let y = pos[1] + i as f32 * self.line_height;

            if shadow {
                const SHADOW_DELTAS: [[f32; 2]; 4] = [
                    [-1.0, -1.0], /*[0.0, -1.0],*/ [1.0, -1.0],
                    /*[-1.0,  0.0], [0.0,  0.0], [1.0,  0.0],*/
                    [-1.0,  1.0], /*[0.0,  1.0],*/ [1.0,  1.0],
                ];

                for [dx, dy] in SHADOW_DELTAS {
                    draw_list.add_text(
                        [x + dx, y + dy],
                        [0.0, 0.0, 0.0, 1.0],
                        line,
                    );
                }
            }

            draw_list.add_text(
                [x, y],
                color,
                line,
            );
        }
    }
}

fn img_dim() -> f32 {
    let [ww, wh] = get_window_size();
    Settings::read_or_default().icon_scale_multiplier * ww.min(wh)
}