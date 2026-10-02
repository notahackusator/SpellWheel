use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use imgui::{DrawListMut, Ui};
use crate::settings::SettingsContext;
use crate::settings::visual_toml_parser::{check_errors, SettingsError};

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
    text: Arc<str>,
    error: Arc<RwLock<SettingsErrorHolder>>,
    shown: bool,
}

impl DisplaySettings {
    pub fn new() -> Self {
        let text = SettingsContext::read_or_default().src.clone();
        let error = Arc::new(RwLock::new(SettingsErrorHolder::new()));
        let shown = false;
        Self {
            text,
            error,
            shown,
        }
    }

    pub fn update(&mut self) {
        if !self.shown {
            return;
        }
        self.text = SettingsContext::read_or_default().src.clone();
        if self.error.read().expect("Couldn't acquire self.error").updated.elapsed() >= Duration::from_millis(250) {
            // consider using another thread if this becomes too expensive
            let mut err = self.error.write().expect("Couldn't acquire self.error");
            err.updated = Instant::now();
            err.error = check_errors(&self.text);
        }
    }

    pub fn draw(&self, ui: &Ui, draw_list: &DrawListMut) {
        if !self.shown {
            return;
        }
        let lh = ui.text_line_height();
        let padding = 20.0;
        let mut cursor = [padding; 2];

        let [w, h] = ui.calc_text_size(&self.text);
        draw_list.add_rect([0.0; 2], [w + padding * 2.0, h + padding * 2.0], [0.2, 0.2, 0.2, 0.5])
            .filled(true)
            .build();
        draw_list.add_rect([padding; 2], [w, h], [0.1, 0.1, 0.1, 0.5])
            .filled(true)
            .build();

        for (i, line) in self.text.split("\n").into_iter().enumerate() {
            ui.set_cursor_pos(cursor);
            ui.text(line);

            if let Some(error) = &self.error.read().expect("Couldn't acquire self.error").error {
                if i == error.line {
                    let [lw, _] = ui.calc_text_size(line);
                    let [ew, eh] = ui.calc_text_size(&error.msg);
                    let c1 = [cursor[0] + lw, cursor[1]];
                    let c2 = [c1[0] + ew, c1[1] + eh];
                    ui.set_cursor_pos(c1);
                    draw_list.add_rect(c1, c2, [1.0, 0.0, 0.0, 1.0])
                        .filled(true)
                        .build();
                    ui.text_colored([1.0; 4], &error.msg);
                }
            }

            cursor[1] += lh;
        }
    }
}