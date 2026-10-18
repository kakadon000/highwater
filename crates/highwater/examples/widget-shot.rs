//! A stand-in for the whole interface, for looking at the desktop widget alone.
//!
//! The widget's design decisions — how much a row breathes, whether the real
//! company mark reads at a glance, what the card does when the daemon has not
//! answered — cannot be judged from the window running among every other
//! surface, and a full build of the interface is more machine than a look at
//! one card needs. So this compiles `ui/widget.slint` on its own and shows the
//! very component the app shows, fed with invented readings that cover the
//! tones a real day produces: a comfortable window, a nearly spent one, a
//! rate-limited one, and an account that has never answered.
//!
//! ```sh
//! cargo run -p highwater --example widget-shot
//! ```
//!
//! Environment: `HW_SHOT_LIGHT=1` keeps the light look (dark otherwise),
//! `HW_SHOT_EMPTY=1` shows the waiting-for-the-daemon state instead of rows,
//! `HW_SHOT_OPACITY=50..100`, and `HW_SHOT_X` / `HW_SHOT_Y` place the window.
//! It is a development aid, not a test fixture: nothing in the suite depends
//! on it.

use std::rc::Rc;

use slint::{ComponentHandle, VecModel};

slint::slint! {
    import { WidgetWindow, WidgetDesk, WidgetRow, Theme } from "../ui/widget.slint";
    export { WidgetWindow, WidgetDesk, WidgetRow, Theme }
}

fn env_number(name: &str, default: i32) -> i32 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn reading(name: &str, value: &str, percent: &str, gauge: f32, tone: i32) -> WidgetRow {
    WidgetRow {
        name: name.into(),
        initial: name.chars().next().unwrap_or('?').to_string().into(),
        mark: slint::Image::default(),
        has_mark: false,
        value: value.into(),
        percent: percent.into(),
        has_gauge: gauge > 0.0,
        gauge_percent: gauge,
        gauge_tone: tone,
        gauge_blocked: false,
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let dark = std::env::var("HW_SHOT_LIGHT").is_err();
    let empty = std::env::var("HW_SHOT_EMPTY").is_ok();
    let opacity = env_number("HW_SHOT_OPACITY", 100).clamp(50, 100);
    let x = env_number("HW_SHOT_X", 0);
    let y = env_number("HW_SHOT_Y", 0);

    let ui = WidgetWindow::new()?;
    let desk = ui.global::<WidgetDesk>();
    desk.set_acrylic(true);
    desk.set_translucent(true);
    desk.set_opacity(opacity);
    desk.set_connected(true);
    let rows = if empty {
        Vec::new()
    } else {
        vec![
            reading("Claude", "12.1k / 225k · 5h", "42%", 42.0, 0),
            reading("Codex", "87.9k / 90k · 1w", "97%", 97.5, 2),
            reading("Kimi", "8.4k / 10k · 3h", "84%", 84.0, 1),
            reading("Antigravity", "waiting…", "", 0.0, 0),
        ]
    };
    desk.set_rows(slint::ModelRc::from(Rc::new(VecModel::from(rows))));
    ui.global::<Theme>().set_dark(dark);

    ui.show()?;
    ui.window().set_position(slint::PhysicalPosition::new(x, y));
    slint::run_event_loop()
}
