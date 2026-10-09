use imgui::{DrawListMut, Ui};
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const POPUP_DURATION: Duration = Duration::from_secs(5);

struct Popup {
    text: String,
    start: Instant,
}

impl Popup {
    fn new(text: String) -> Self {
        Self {
            text,
            start: Instant::now(),
        }
    }
}

lazy_static!(
    static ref POPUPS: Arc<Mutex<Vec<Popup>>> = Arc::new(Mutex::new(Vec::new()));
);

pub fn add_popup(text: String) {
    POPUPS.lock().expect("Couldn't acquire POPUPS").push(Popup::new(text));
}

pub fn draw_popups(ui: &Ui, draw_list: &DrawListMut) {
    let mut popups = POPUPS.lock().expect("Couldn't acquire POPUPS");
    popups.retain(|popup| popup.start.elapsed() <= POPUP_DURATION);
    let hw = ui.window_size()[0] / 2.0;
    let mut y = 20.0;
    for popup in popups.iter() {
        let [w, h] = ui.calc_text_size(&popup.text);
        draw_list.add_rect([hw - w / 2.0 - 5.0, y], [hw + w / 2.0 + 5.0, y + h + 10.0], [0.3, 0.3, 0.3, 1.0])
            .filled(true).build();
        draw_list.add_text([hw - w / 2.0, y + 5.0], [1.0; 4], &popup.text);
        y += h + 20.0;
    }
}