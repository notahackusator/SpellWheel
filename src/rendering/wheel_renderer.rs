use crate::debugging::{add_to_screen_debug, is_debugging};
use crate::gamepad::gamepad_state;
use crate::hwindow::get_window_size;
use crate::icons::icon_manager::IconManager;
use crate::mouse::get_mouse_state;
use crate::rendering::debug_renderer::draw_debug;
use crate::rendering::display_item::DisplayItem;
use crate::settings::{Settings, Style};
use imgui::{DrawListMut, Ui};

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

pub fn closest_item_to(items: &[DisplayItem], angle: f32) -> Option<usize> {
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

pub fn rotated_image(cx: f32, cy: f32, width: f32, height: f32, angle: f32) -> [[f32; 2]; 4] {
    let sin = angle.sin();
    let cos = angle.cos();

    let hw = width / 2.0;
    let hh = height / 2.0;

    let corners = [
        [ hw, -hh],
        [ hw,  hh],
        [-hw,  hh],
        [-hw, -hh],
    ];

    corners.map(|[x, y]| [
        cx + x * cos - y * sin,
        cy + x * sin + y * cos,
    ])
}

pub fn render_selector(settings: &Settings, items: &[DisplayItem], ww: f32, wh: f32, img_dim: f32, draw_list: &DrawListMut, angle: f32, can_select: bool) {
    match settings.style() {
        Style::Pretty => {
            let Some(static_icons) = IconManager::get_static_icons() else {
                return;
            };

            let thickness = ww.min(wh) / 200.0;
            let radius = settings.radius_multiplier * ww.min(wh) - img_dim - thickness * 2.0;

            let [cx, cy] = [ww / 2.0, wh / 2.0];

            let [r, g, b, _, angle] = items.iter()
                .map(|di| (
                    settings.normalized_highlight_time(di.highlight_time),
                    di.angle,
                    di.icon.as_ref().map(|icon| icon.primary_color)
                ))
                .filter_map(|(highlight_time, angle, col)| col.map(|col| [
                    highlight_time * col[0] as f32,
                    highlight_time * col[1] as f32,
                    highlight_time * col[2] as f32,
                    highlight_time, // this will become col[3]
                    angle,          // this will become col[4]
                ]))
                .reduce(|col1, col2| {
                    let x = col1[4].cos() * col1[3] + col2[4].cos() * col2[3];
                    let y = col1[4].sin() * col1[3] + col2[4].sin() * col2[3];
                    [
                        col1[0] + col2[0],
                        col1[1] + col2[1],
                        col1[2] + col2[2],
                        col1[3] + col2[3],
                        y.atan2(x),
                    ]
                })
                .unwrap_or([255.0, 255.0, 255.0, 0.0, 0.0]);

            let rotated = rotated_image(
                cx, cy, radius * 2.0, radius * 2.0, angle
            );

            let col = u32::from_be_bytes([0xFE, b as u8, g as u8, r as u8]);
            draw_list.add_image_quad(
                static_icons.selector,
                rotated[0],
                rotated[1],
                rotated[2],
                rotated[3],
            )
                .col(col)
                .build();
        }
        Style::Simple => {
            if !settings.using_controller || !can_select {
                return;
            }

            let thickness = ww.min(wh) / 200.0;
            let radius = settings.radius_multiplier * ww.min(wh) - img_dim - thickness * 2.0;

            let [cx, cy] = [ww / 2.0, wh / 2.0];

            let bezier = arc_bezier(
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
    }
}

pub fn render_wheel(items: &mut [DisplayItem], ui: &Ui, draw_list: &DrawListMut) {
    draw_debug(ui, draw_list);
    if items.is_empty() {
        return;
    }
    let settings = Settings::read_or_default();

    let (angle, dist_sqr) = angle_and_dist_sqr(ui, match settings.using_controller {
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
        if let Some(closest_idx) = closest_item_to(items, angle) {
            if is_debugging() {
                add_to_screen_debug(format!("Closest item index: {closest_idx}"))
            }
            items[closest_idx].is_highlighted = true;
        } else if is_debugging() {
            add_to_screen_debug("No item highlighted, but within range.".to_string());
        }
    }

    let img_dim = DisplayItem::img_dim();
    render_selector(&settings, items, ww, wh, img_dim, draw_list, angle, can_select);

    let num_items = items.len();
    for item in items.iter_mut() {
        item.tick(&settings);
        item.draw(&settings, ww, wh, img_dim, num_items, ui, draw_list);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WheelType {
    Spells,
    QuickItems,
    None
}