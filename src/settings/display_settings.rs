use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use imgui::{DrawListMut, Ui};
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_F10, VK_LSHIFT};
use crate::keyboard::{setup_listener, ListenerStatus, Listener};
use crate::settings::SettingsContext;
use crate::settings::visual_toml_parser::{check_errors, SettingsError, Span, parse_toml};

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
    toml_parse: Vec<Vec<Span>>,
    listener: ListenerStatus,
    shown: bool,
}

impl DisplaySettings {
    pub fn new() -> Self {
        let text = SettingsContext::read_or_default().src.clone();
        let error = Arc::new(RwLock::new(SettingsErrorHolder::new()));
        let toml_parse = parse_toml(&text);
        let listener = setup_listener(Listener::Released(Box::new([VK_LSHIFT.0 as i32, VK_F10.0 as i32])));
        let shown = false;
        Self {
            text,
            error,
            toml_parse,
            listener,
            shown,
        }
    }

    pub fn update(&mut self) {
        if self.listener.is_active() {
            self.shown = !self.shown;
        }
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
        let origin = ui.window_pos();

        let guard = self.error.read().expect("Couldn't acquire self.error");
        let error = guard.error.as_ref();

        let lines: Vec<&str> = self.text.split('\n').collect();

        let mut text_width = 0.0f32;
        for (i, line) in lines.iter().enumerate() {
            let mut lw = ui.calc_text_size(line)[0];

            if let Some(error) = error {
                if error.line == i {
                    lw += ui.calc_text_size(&error.msg)[0];
                }
            }

            text_width = text_width.max(lw);
        }
        let text_height = lines.len() as f32 * lh;

        let inner_min = [origin[0] + padding, origin[1] + padding];
        let inner_max = [inner_min[0] + text_width, inner_min[1] + text_height];
        let outer_max = [inner_max[0] + padding, inner_max[1] + padding];

        draw_list.add_rect(origin, outer_max, [0.2, 0.2, 0.2, 0.5]).filled(true).build();
        draw_list.add_rect(inner_min, inner_max, [0.1, 0.1, 0.1, 0.5]).filled(true).build();

        for (line_num, line) in lines.iter().enumerate() {
            let pos = [inner_min[0], inner_min[1] + line_num as f32 * lh];

            // Render line text
            if let Some(spans) = self.toml_parse.get(line_num) {
                for sp in spans {
                    let (Some(prefix), Some(text)) = (line.get(..sp.start), line.get(sp.start..sp.end)) else {
                        continue;
                    };
                    let x = pos[0] + ui.calc_text_size(prefix)[0];
                    draw_list.add_text([x, pos[1]], sp.kind.color(), text);
                }
            }

            // Render line error
            if let Some(error) = error {
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
        }
    }
}