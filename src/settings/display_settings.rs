use crate::keyboard::listener::setup_listener;
use crate::keyboard::listener::{Listener, ListenerStatus};
use crate::settings::SettingsContext;
use crate::settings::visual_toml_parser::{SettingsError, Span, check_errors, parse_toml};
use imgui::{ColorStackToken, DrawListMut, InputTextCallbackHandler, InputTextMultilineCallback, StyleColor, StyleStackToken, StyleVar, TextCallbackData, Ui};
use lazy_static::lazy_static;
use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_F10, VK_LSHIFT};
use crate::icons::icon_manager::IconManager;

lazy_static!(
    static ref IS_OPEN: AtomicBool = AtomicBool::new(false);
);

pub fn display_settings_open() -> bool {
    IS_OPEN.load(Ordering::Relaxed)
}

#[derive(Clone, Debug)]
pub struct EditCallback {
    cursor: Rc<RefCell<usize>>,
    selection: Rc<RefCell<Range<usize>>>,
    scroll: Rc<RefCell<f32>>,
}

impl EditCallback {
    pub fn new() -> Self {
        Self {
            cursor: Rc::new(RefCell::new(0)),
            selection: Rc::new(RefCell::new(0..0)),
            scroll: Rc::new(RefCell::new(0.0)),
        }
    }
}

impl InputTextCallbackHandler for EditCallback {
    fn on_always(&mut self, data: TextCallbackData) {
        self.cursor.replace(data.cursor_pos());
        self.selection.replace(data.selection());
    }
}

pub trait GenericPop {
    fn generic_pop(self: Box<Self>);
}

impl GenericPop for StyleStackToken<'_> {
    fn generic_pop(self: Box<Self>) {
        self.pop()
    }
}

impl GenericPop for ColorStackToken<'_> {
    fn generic_pop(self: Box<Self>) {
        self.pop()
    }
}

pub struct SettingsErrorHolder {
    updated: Instant,
    error: Option<SettingsError>,
}

impl SettingsErrorHolder {
    pub fn new() -> Self {
        Self {
            updated: Instant::now(),
            error: None,
        }
    }
}

pub struct DisplaySettings {
    pub text: String,
    pub error: Option<SettingsError>,
    pub toml_parse: Vec<Vec<Span>>,
    pub should_reload: bool,
    pub listener: ListenerStatus,
    pub shown: bool,
}

impl DisplaySettings {
    pub fn new() -> Self {
        let text = SettingsContext::read_or_default().src.replace("\r", "");
        let error = None;
        let toml_parse = parse_toml(&text);
        let should_reload = false;
        let listener = setup_listener(
            Listener::Released {
                keys: Box::new([VK_LSHIFT.0 as i32, VK_F10.0 as i32]),
            }
        );
        let shown = false;
        Self {
            text,
            error,
            toml_parse,
            should_reload,
            listener,
            shown,
        }
    }

    pub fn update(&mut self) {
        if self.listener.is_active() {
            self.shown = !self.shown;
        }
        IS_OPEN.store(self.shown, Ordering::Relaxed);
        if !self.shown {
            return;
        }
    }

    pub fn update_toml(&mut self) {
        self.toml_parse = parse_toml(&self.text);
        self.error = check_errors(&self.text);
    }

    pub fn edit(&mut self, ui: &Ui, pos: [f32; 2], size: [f32; 2]) -> EditCallback {
        let mut callback = EditCallback::new();
        let lh = ui.text_line_height();

        let (mut n_lines, mut max_w) = (0, 0.0f32);
        for l in self.text.split('\n') {
            n_lines += 1;
            max_w = max_w.max(ui.calc_text_size(l)[0]);
        }

        let _wp = ui.push_style_var(StyleVar::WindowPadding([0.0; 2]));
        ui.set_cursor_pos(pos);

        ui.child_window("settings scroll")
            .size(size)
            .build(|| {
                let size = [
                    (max_w + lh * 4.0).max(size[0]),
                    ((n_lines as f32 + 4.0) * lh).max(size[1]),
                ];

                let styles: [Box<dyn GenericPop>; _] = [
                    Box::new(ui.push_style_var(StyleVar::FramePadding([0.0; 2]))),
                    Box::new(ui.push_style_color(StyleColor::Text, [0.; 4])),
                    Box::new(ui.push_style_color(StyleColor::FrameBg, [0.; 4])),
                ];

                let changed = ui
                    .input_text_multiline("settings editor", &mut self.text, size)
                    .callback(InputTextMultilineCallback::ALWAYS, callback.clone())
                    .build();
                if changed {
                    self.update_toml();
                }
                callback.scroll.replace(ui.scroll_y());
                for style in styles {
                    style.generic_pop();
                }
            });

        callback
    }
    pub fn draw(&mut self, ui: &Ui, draw_list: &DrawListMut) {
        if !self.shown {
            return;
        }
        let lh = ui.text_line_height();
        let padding_x = 20.0;
        let padding_y = 20.0;
        let element_margin = 20.0;
        let element_padding = 5.0;

        let origin = ui.window_pos();
        let screen_size = ui.window_size();

        let [save_button_min, save_button_max, save_button_text_pos] = Self::calc_button_sizes(
            ui, [origin[0] + padding_x, origin[1] + padding_y], "Save", element_padding
        );
        let [reload_button_min, reload_button_max, reload_button_text_pos] = Self::calc_button_sizes(
            ui, [save_button_max[0] + element_margin, save_button_min[1]], "Reload icons", element_padding
        );
        let screen_min = [origin[0] + padding_x, save_button_max[1] + element_margin];
        let screen_max = [screen_size[0] - padding_x, screen_size[1] - padding_y];

        let cb = self.edit(
            ui,
            [origin[0] + padding_x, save_button_max[1] + element_margin],
            [screen_size[0] - padding_x * 2.0, screen_size[1] - padding_y * 2.0]
        );

        let mut scroll_origin = origin;
        scroll_origin[1] -= cb.scroll.take();
        let mut caret_index = cb.cursor.take() as isize;

        let lines: Vec<&str> = self.text.split('\n').collect();

        // Overall text dimensions
        let mut text_width = 0.0f32;
        for (i, line) in lines.iter().enumerate() {
            let mut lw = ui.calc_text_size(line)[0];

            if let Some(error) = &self.error {
                if error.line == i {
                    lw += ui.calc_text_size(&error.msg)[0];
                }
            }

            text_width = text_width.max(lw);
        }
        let text_height = lines.len() as f32 * lh;

        let inner_min = [
            scroll_origin[0] + padding_x,
            scroll_origin[1] + save_button_max[1] + element_margin
        ];
        let inner_max = [
            inner_min[0] + text_width,
            inner_min[1] + text_height
        ];
        let outer_min = [
            origin[0],
            origin[1]
        ];
        let outer_max = [
            outer_min[0] + text_width + padding_x * 2.0,
            outer_min[1] + text_height + padding_y * 2.0
        ];

        // Backgrounds
        draw_list.add_rect(outer_min, outer_max, [0.2, 0.2, 0.2, 0.5]).filled(true).build();
        draw_list.add_rect(inner_min, inner_max, [0.1, 0.1, 0.1, 0.5]).filled(true).build();

        // Buttons
        if Self::button(ui, draw_list, "##save", save_button_min,
                     save_button_max, save_button_text_pos, "Save") {
            tracing::info!("Saving settings...");
            SettingsContext::save(&self.text);
            tracing::info!("Settings saved");
        }
        if Self::button(ui, draw_list, "##reload", reload_button_min,
                     reload_button_max, reload_button_text_pos, "Reload icons") {
            tracing::info!("Reloading icons...");
            IconManager::reinit();
            tracing::info!("IconManager reinitialized");
            self.should_reload = true;
        }

        draw_list.with_clip_rect(screen_min, screen_max, || {
            for (line_num, line) in lines.iter().enumerate() {
                let pos = [inner_min[0], inner_min[1] + line_num as f32 * lh];

                // Render line text
                if let Some(spans) = self.toml_parse.get(line_num) {
                    for span in spans {
                        let (Some(prefix), Some(text)) = (line.get(..span.start), line.get(span.start..span.end)) else {
                            continue;
                        };
                        let x = pos[0] + ui.calc_text_size(prefix)[0];
                        draw_list.add_text([x, pos[1]], span.kind.color(), text);
                    }
                }

                // Render line error
                if let Some(error) = &self.error {
                    if error.line == line_num {
                        let lw = ui.calc_text_size(line)[0];
                        let [ew, eh] = ui.calc_text_size(&error.msg);
                        let c1 = [pos[0] + lw, pos[1]];
                        draw_list
                            .add_rect(c1, [c1[0] + ew, c1[1] + eh], [1.0, 0.0, 0.0, 1.0])
                            .filled(true)
                            .build();
                        draw_list.add_text(c1, [1.0; 4], &error.msg);
                    }
                }

                // Render caret
                let line_len_with_newline = line.len() as isize + 1;
                if 0 <= caret_index && caret_index < line_len_with_newline {
                    let caret_pos = ui.calc_text_size(&line[..caret_index as usize]);
                    let c1 = [pos[0] + caret_pos[0], pos[1]];
                    let c2 = [c1[0] + 4.0, pos[1] + caret_pos[1]];
                    draw_list.add_rect(c1, c2, [1.0; 4])
                        .filled(true)
                        .build();
                }

                caret_index -= line_len_with_newline;
            }
        });
    }

    fn button(ui: &Ui, draw_list: &DrawListMut, id: &str, c1: [f32; 2], c2: [f32; 2], text_pos: [f32; 2], text: &str) -> bool {
        ui.set_cursor_screen_pos(c1);
        draw_list.add_rect(c1, c2, [0.3, 0.3, 0.3, 1.0]).filled(true).build();
        draw_list.add_text(text_pos, 0xFF_FF_FF_FF, text);
        ui.invisible_button(id, [c2[0] - c1[0], c2[1] - c1[1]])
    }

    fn calc_button_sizes(ui: &Ui, start_pos: [f32; 2], text: &str, element_padding: f32) -> [[f32; 2]; 3] {
        let text_size = ui.calc_text_size(text);
        let button_max = [
            start_pos[0] + text_size[0] + element_padding * 2.0,
            start_pos[1] + text_size[1] + element_padding * 2.0
        ];
        let button_text_pos = [
            start_pos[0] + element_padding,
            start_pos[1] + element_padding
        ];
        [start_pos, button_max, button_text_pos]
    }
}