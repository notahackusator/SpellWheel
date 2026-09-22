use imgui::{DrawListMut, Ui};

pub struct WrappedText {
    pub lines: Vec<String>,
    pub widths: Vec<f32>,
    pub line_height: f32,
    pub scale: f32,
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

        let scale = 1.0;

        Self {
            lines,
            widths,
            line_height,
            scale,
        }
    }

    pub fn draw(&self, ui: &Ui, pos: [f32; 2], color: [f32; 4], centered: Centered, shadow: bool) {
        ui.group(|| {
            ui.set_window_font_scale(self.scale);

            for (i, (line, width)) in self.lines.iter().zip(self.widths.iter()).enumerate() {
                let width = width * self.scale;
                let line_height = self.line_height * self.scale;
                let x = if centered.x() {
                    pos[0] - width / 2.0
                } else {
                    pos[0]
                };
                let y = if centered.y() {
                    pos[1] + i as f32 * line_height - (self.lines.len() as f32 / 2.0) * line_height
                } else {
                    pos[1] + i as f32 * line_height
                };

                if shadow {
                    const SHADOW_DELTAS: [[f32; 2]; 4] = [
                        [-1.0, -1.0], /*[0.0, -1.0],*/ [1.0, -1.0],
                        /*[-1.0,  0.0], [0.0,  0.0], [1.0,  0.0],*/
                        [-1.0, 1.0], /*[0.0,  1.0],*/ [1.0, 1.0],
                    ];

                    for [dx, dy] in SHADOW_DELTAS {
                        ui.set_cursor_pos([x + dx, y + dy]);
                        ui.text_colored(
                            [0.0, 0.0, 0.0, 1.0],
                            line,
                        );
                    }
                }

                ui.set_cursor_pos([x, y]);
                ui.text_colored(
                    color,
                    line,
                );
            }
        });
    }

    #[deprecated]
    pub fn add_to_draw_list(&self, draw_list: &DrawListMut, pos: [f32; 2], color: [f32; 4], centered: Centered, shadow: bool) {
        for (i, (line, width)) in self.lines.iter().zip(self.widths.iter()).enumerate() {
            let x = if centered.x() {
                pos[0] - width / 2.0
            } else {
                pos[0]
            };
            let y = if centered.y() {
                pos[1] + i as f32 * self.line_height - (self.lines.len() as f32 / 2.0) * self.line_height
            } else {
                pos[1] + i as f32 * self.line_height
            };

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Centered {
    XY,
    X,
    Y,
    None,
}

impl Centered {
    pub fn x(&self) -> bool {
        matches!(self, Self::X | Self::XY)
    }

    pub fn y(&self) -> bool {
        matches!(self, Self::Y | Self::XY)
    }
}