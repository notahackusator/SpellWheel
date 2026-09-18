use imgui::{DrawListMut, Ui};
use crate::debugging::read_committed_screen_debug;

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