use imgui::Ui;
use unicode_bidi::{BidiClass, bidi_class};

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
            let line = Self::directional_line(line);
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

    fn group_directions<T>(directions: impl Iterator<Item = (T, bool)>) -> Vec<(Vec<T>, bool)> {
        let mut result: Vec<(Vec<T>, bool)> = Vec::new();

        for (t, dir) in directions {
            match result.last_mut() {
                Some((ts, prev_dir)) if *prev_dir == dir => ts.push(t),
                _ => result.push((vec![t], dir)),
            }
        }

        result
    }

    pub fn directional_line(line: &str) -> String {
        let line = Self::group_directions(line.split_whitespace().map(Self::directional_word));
        let first_is_rtl = line.first().is_some_and(|(_, dir)| *dir);
        let sections = line.into_iter()
            .map(|(mut words, is_rtl)| {
                if is_rtl {
                    words.reverse();
                }
                words.join(" ")
            });
        if first_is_rtl {
            sections.rev().collect::<Vec<_>>().join(" ")
        } else {
            sections.collect::<Vec<_>>().join(" ")
        }
    }

    pub fn directional_word(word: &str) -> (String, bool) {
        let mut char_directions = Vec::with_capacity(word.len());

        // Creates LTR and RTL sections
        for ch in word.chars() {
            char_directions.push((ch, matches!(bidi_class(ch), BidiClass::R | BidiClass::AL)));
        }

        // Generates final output string
        let mut out = String::with_capacity(word.len());
        let mut last_section_rtl = false;
        for (chars, is_rtl) in Self::group_directions(char_directions.into_iter()) {
            let section = if is_rtl {
                String::from_iter(chars.iter().rev())
            } else {
                String::from_iter(chars.iter())
            };
            out.push_str(&section);
            last_section_rtl = is_rtl;
        }
        (out, last_section_rtl)
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