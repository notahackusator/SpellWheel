use crate::debugging::{add_to_screen_debug, is_debugging};
use crate::hwindow::get_window_size;
use crate::icons::icon_manager::IconManager;
use crate::icons::AtlasIcon;
use crate::items::Item;
use crate::rendering::wheel_renderer::arc_bezier;
use crate::rendering::wrapped_text::{Centered, WrappedText};
use crate::settings::{ItemNames, Settings, Style};
use imgui::{DrawListMut, Ui};
use std::fmt::{Debug, Formatter};

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

const BG_UV_OFFSET: f32 = 0.00125;

impl DisplayItem {
    pub fn dist(&self, cos: f32, sin: f32) -> f32 {
        let dx = cos - self.cos_sin[0];
        let dy = sin - self.cos_sin[1];
        dx * dx + dy * dy
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

        let img_dim = Self::img_dim();

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
                let icon = IconManager::get_item(item);
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

    pub fn img_dim() -> f32 {
        let [ww, wh] = get_window_size();
        Settings::read_or_default().icon_scale_multiplier * ww.min(wh)
    }

    pub fn draw(&self, settings: &Settings, ww: f32, wh: f32, img_dim: f32, num_items: usize, ui: &Ui, draw_list: &DrawListMut) {
        let [cx, cy] = [ww / 2.0, wh / 2.0];

        self.draw_controller_arc(settings, ww, wh, img_dim, num_items, draw_list, cx, cy);

        let (
            Some(AtlasIcon { texture_id, rect, primary_color, .. }),
            Some(static_icons)
        ) = (&self.icon, IconManager::get_static_icons()) else {
            draw_list.add_rect(
                self.img_c1,
                self.img_c2,
                [0.5, 0.5, 0.5, 1.0]
            ).build();
            return;
        };

        match settings.style() {
            Style::Pretty => {
                let item_bg = if self.is_highlighted {
                    self.draw_centered_name(settings, ui, draw_list, cx, cy);
                    static_icons.selected_item_bg
                } else {
                    static_icons.item_bg
                };
                let col = u32::from_be_bytes([0xFE, primary_color[2], primary_color[1], primary_color[0]]);
                draw_list.add_image(
                    item_bg,
                    self.img_c1,
                    self.img_c2,
                )
                    .col(col)
                    .uv_min([BG_UV_OFFSET, BG_UV_OFFSET])
                    .uv_max([1.0 - BG_UV_OFFSET, 1.0 - BG_UV_OFFSET])
                    .build();
            }
            Style::Simple => {
                if self.is_highlighted {
                    draw_list.add_rect(
                        self.rect_c1,
                        self.rect_c2,
                        [1.0, 1.0, 1.0, 0.2]
                    ).filled(true).rounding(10.0).build();

                    self.draw_centered_name(settings, ui, draw_list, cx, cy);
                }
            }
        }

        let [x, y, w, h] = *rect;
        draw_list.add_image(
            *texture_id,
            self.img_c1,
            self.img_c2,
        )
            .uv_min([x, y])
            .uv_max([x + w, y + h])
            .col(0xFE_FF_FF_FF)
            .build();

        if let ItemNames::Show = settings.item_names() {
            self.name.add_to_draw_list(draw_list, self.text_pos, ui.style_color(imgui::StyleColor::Text), Centered::X, settings.text_shadows);
        }
    }

    fn draw_centered_name(&self, settings: &Settings, ui: &Ui, draw_list: &DrawListMut, cx: f32, cy: f32) {
        if let ItemNames::Center = settings.item_names() {
            self.name.add_to_draw_list(draw_list, [cx, cy], ui.style_color(imgui::StyleColor::Text), Centered::XY, settings.text_shadows);
        }
    }

    fn draw_controller_arc(&self, settings: &Settings, ww: f32, wh: f32, img_dim: f32, num_items: usize, draw_list: &DrawListMut, cx: f32, cy: f32) {
        if settings.using_controller {
            let thickness = ww.min(wh) / 200.0;

            let radius = settings.radius_multiplier * ww.min(wh) - img_dim;

            let angle_offset = std::f32::consts::PI / num_items.max(2) as f32 - (thickness / radius).atan();
            if is_debugging() {
                add_to_screen_debug(format!("{angle_offset} {thickness} {radius} {}", (thickness / radius).atan()));
            }

            let bezier = arc_bezier(
                cx, cy, radius, self.angle - angle_offset, self.angle + angle_offset
            );
            draw_list.add_bezier_curve(bezier[0], bezier[1], bezier[2], bezier[3], [1.0; 4]).thickness(thickness).build();
        }
    }
}