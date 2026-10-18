//! The desktop widget: a small translucent tracker that floats over the desktop.
//!
//! One Slint window (`ui/widget.slint`), shown beside the main one and fed from the
//! same redraw that feeds the cards, so what it says can never drift from what the
//! window says. Its settings — shown, on top, opacity, and the position it was
//! dragged to — belong to this client rather than to the daemon: the daemon has no
//! window to place, and a widget must survive a daemon that has never heard of it.
//! They live in a small JSON file beside the client's own data, which is also why
//! reading and writing it never leaves this module.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use serde::{Deserialize, Serialize};
use slint::winit_030::WinitWindowAccessor;
use slint::winit_030::winit::window::WindowLevel;
use slint::{ComponentHandle, PhysicalPosition, VecModel};

use crate::ui::{Theme, WidgetDesk, WidgetRow, WidgetWindow};

/// One row per provider, up to a height a desktop corner deserves; anything past
/// the cap becomes a single "and N more" row that opens the window.
pub const MAX_ROWS: usize = 14;

/// The widget's size choices, as the preferences sheet offers them: compact,
/// default, large. The config file carries the percentage itself.
pub const SCALES: [i32; 3] = [90, 100, 115];

/// The client's own widget settings, as the JSON file carries them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct WidgetConfig {
    #[serde(default = "default_enabled")]
    enabled: bool,
    #[serde(default = "default_on_top")]
    on_top: bool,
    #[serde(default = "default_opacity")]
    opacity: i32,
    /// The card's size, as a percentage: one of [`SCALES`].
    #[serde(default = "default_scale")]
    scale: i32,
    /// The card's look: "auto" follows the desktop, "light" and "dark" hold
    /// still. Anything unreadable is auto.
    #[serde(default = "default_look")]
    look: String,
    /// The last physical position the card was dragged to, `[x, y]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    position: Option<[i32; 2]>,
}

fn default_enabled() -> bool {
    true
}

fn default_on_top() -> bool {
    true
}

fn default_opacity() -> i32 {
    85
}

fn default_scale() -> i32 {
    100
}

/// The widget look choices, as the preferences sheet offers them.
pub const LOOKS: [&str; 3] = ["auto", "light", "dark"];

fn default_look() -> String {
    "auto".to_owned()
}

impl WidgetConfig {
    /// The look as it was written, folded into one of [`LOOKS`].
    fn look(&self) -> &'static str {
        LOOKS
            .iter()
            .find(|choice| **choice == self.look)
            .copied()
            .unwrap_or("auto")
    }
}

impl Default for WidgetConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            on_top: default_on_top(),
            opacity: default_opacity(),
            scale: default_scale(),
            look: default_look(),
            position: None,
        }
    }
}

/// Where the widget's settings live. Windows keeps the client's data under
/// `%LOCALAPPDATA%\highwater` (the log already does); XDG sessions under
/// `config/highwater`.
fn config_path() -> Option<std::path::PathBuf> {
    #[cfg(windows)]
    let dir = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .map(|base| base.join("highwater"));
    #[cfg(not(windows))]
    let dir = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .filter(|home| !home.as_os_str().is_empty())
        .or_else(|| {
            std::env::var_os("HOME")
                .map(std::path::PathBuf::from)
                .map(|home| home.join(".config"))
        })
        .filter(|home| !home.as_os_str().is_empty())
        .map(|home| home.join("highwater"));
    Some(dir?.join("widget.json"))
}

/// What the widget wants the rest of the interface to hear about: the setting
/// changed, from the ✕, the tray, or the preferences sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Enabled(bool),
}

/// The listeners one widget event is announced to.
type EventListeners = RefCell<Vec<Rc<dyn Fn(Event)>>>;

pub struct WidgetPanel {
    ui: slint::Weak<WidgetWindow>,
    rows: Rc<VecModel<WidgetRow>>,
    config: RefCell<WidgetConfig>,
    /// The position as it was last written, so a quiet card writes nothing.
    saved_at: Cell<Option<(i32, i32)>>,
    /// The desktop's own dark choice, which "auto" follows.
    desktop_dark: Cell<bool>,
    events: EventListeners,
    /// Debounces a header drag: the system reports no drag's end, only moves,
    /// and a settings file is not written sixty times a second.
    save_timer: slint::Timer,
}

impl std::fmt::Debug for WidgetPanel {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WidgetPanel")
            .field("config", &self.config.borrow())
            .finish_non_exhaustive()
    }
}

impl WidgetPanel {
    /// Builds the widget window and applies what the settings file says. The
    /// window is created whether or not it is shown: showing it later must not
    /// depend on being the first thing after startup.
    pub fn start(translucent: bool, on_open: impl Fn() + 'static) -> Rc<Self> {
        let ui = WidgetWindow::new().expect("the widget window compiles");
        let panel = Rc::new(Self {
            ui: ui.as_weak(),
            rows: Rc::new(VecModel::default()),
            config: RefCell::new(Self::load()),
            saved_at: Cell::default(),
            desktop_dark: Cell::new(false),
            events: RefCell::default(),
            save_timer: slint::Timer::default(),
        });

        let desk = ui.global::<WidgetDesk>();
        desk.set_acrylic(translucent);
        // The card's corners, shadow and clear gutter exist only where the
        // surface can carry them; an opaque surface gets a square, solid card
        // that fills its window edge to edge.
        desk.set_translucent(translucent);
        desk.set_opacity(panel.config.borrow().opacity);
        desk.set_scale(panel.config.borrow().scale as f32 / 100.0);
        desk.set_rows(slint::ModelRc::from(Rc::clone(&panel.rows)));
        let weak = Rc::downgrade(&panel);
        desk.on_hide(move || {
            if let Some(panel) = weak.upgrade() {
                panel.set_enabled(false);
            }
        });
        desk.on_moved({
            let weak = Rc::downgrade(&panel);
            move || {
                if let Some(panel) = weak.upgrade() {
                    panel.schedule_save();
                }
            }
        });
        desk.on_open_main(on_open);

        // The widget shares the palette's rules, not the main window's globals:
        // each Slint instance carries its own, so the desktop's appearance is
        // copied in here whenever it changes (see `set_appearance`). The palette's
        // own defaults are light and GNOME blue; the first watcher tick corrects
        // both, exactly as it does for the main window.
        panel.apply_on_top();
        panel.skip_taskbar();
        if panel.config.borrow().enabled {
            panel.show();
        }
        panel
    }

    /// Everything the panel now shows, one row per visible provider card.
    pub fn publish(&self, rows: Vec<WidgetRow>, connected: bool) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        ui.global::<WidgetDesk>().set_connected(connected);
        self.rows.set_vec(rows);
        // The winit backend sizes a window only when it shows it, and this one
        // shows before the first readings land: without resizing here the card
        // clips inside the frame it was created with, and every account added
        // or removed after keeps the stale frame. The card's own arithmetic
        // comes back through the window's public functions, so this cannot
        // drift from what the .slint lays out.
        ui.window().set_size(slint::LogicalSize::new(
            ui.invoke_preferred_window_width(),
            ui.invoke_preferred_window_height(),
        ));
        self.place();
    }

    /// Copies the desktop's dark choice onto this window's own palette, as the
    /// card's look resolves it: "auto" follows the desktop, "light" and "dark"
    /// hold still.
    pub fn set_appearance(&self, dark: bool) {
        self.desktop_dark.set(dark);
        self.apply_look();
    }

    /// Puts the card on the look its setting asks for. A fresh Slint instance
    /// starts light, so the first tick after a dark launch lands here too.
    fn apply_look(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let dark = match self.config.borrow().look() {
            "light" => false,
            "dark" => true,
            _ => self.desktop_dark.get(),
        };
        ui.global::<Theme>().set_dark(dark);
    }

    pub fn look(&self) -> &'static str {
        self.config.borrow().look()
    }

    /// Sets the card's look — the preferences sheet's Auto / Light / Dark. An
    /// unknown word is auto, the same way an unreadable settings file is.
    pub fn set_look(&self, look: &str) {
        let look = LOOKS
            .iter()
            .find(|choice| **choice == look)
            .copied()
            .unwrap_or("auto");
        {
            let mut config = self.config.borrow_mut();
            if config.look() == look {
                return;
            }
            config.look = look.to_owned();
        }
        self.persist();
        self.apply_look();
    }

    /// The look as the preferences sheet's menu should name it: an index into
    /// [`LOOKS`].
    pub fn look_choice(&self) -> usize {
        LOOKS
            .iter()
            .position(|choice| *choice == self.look())
            .unwrap_or_default()
    }

    /// Copies the desktop's accent onto this window's own palette.
    pub fn set_accent(&self, accent: [f64; 3]) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let channel = |value: f64| (value * 255.0).round() as u8;
        ui.global::<Theme>().set_accent(slint::Color::from_rgb_u8(
            channel(accent[0]),
            channel(accent[1]),
            channel(accent[2]),
        ));
    }

    pub fn enabled(&self) -> bool {
        self.config.borrow().enabled
    }

    /// Shows or hides the widget, remembers it, and says so.
    pub fn set_enabled(self: &Rc<Self>, enabled: bool) {
        {
            let mut config = self.config.borrow_mut();
            if config.enabled == enabled {
                return;
            }
            config.enabled = enabled;
        }
        self.persist();
        if enabled {
            self.show();
        } else if let Some(ui) = self.ui.upgrade() {
            self.save_position(false);
            if let Err(error) = ui.hide() {
                tracing::warn!(%error, "the widget window would not hide");
            }
        }
        self.announce(Event::Enabled(enabled));
    }

    /// The tray's row for it: off to on, or on to off.
    pub fn toggle(self: &Rc<Self>) {
        let enabled = !self.enabled();
        self.set_enabled(enabled);
    }

    pub fn set_on_top(&self, on_top: bool) {
        {
            let mut config = self.config.borrow_mut();
            if config.on_top == on_top {
                return;
            }
            config.on_top = on_top;
        }
        self.persist();
        self.apply_on_top();
    }

    pub fn on_top(&self) -> bool {
        self.config.borrow().on_top
    }

    pub fn set_opacity(&self, opacity: i32) {
        {
            let mut config = self.config.borrow_mut();
            let opacity = opacity.clamp(50, 100);
            if config.opacity == opacity {
                return;
            }
            config.opacity = opacity;
        }
        self.persist();
        if let Some(ui) = self.ui.upgrade() {
            let opacity = self.config.borrow().opacity;
            ui.global::<WidgetDesk>().set_opacity(opacity);
        }
    }

    pub fn opacity(&self) -> i32 {
        self.config.borrow().opacity
    }

    /// The card's size, as the percentage the config carries.
    pub fn scale(&self) -> i32 {
        self.config.borrow().scale
    }

    /// Resizes the card to the nearest size the widget offers, so a config
    /// written by hand cannot land between the choices.
    pub fn set_scale(&self, scale: i32) {
        let scale = SCALES
            .iter()
            .copied()
            .min_by_key(|choice| (choice - scale).abs())
            .unwrap_or_default();
        {
            let mut config = self.config.borrow_mut();
            if config.scale == scale {
                return;
            }
            config.scale = scale;
        }
        self.persist();
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<WidgetDesk>().set_scale(scale as f32 / 100.0);
        }
    }

    /// Forgets where the card was dragged and puts it back at its desktop
    /// corner, the way a first launch would find it.
    pub fn reset_position(&self) {
        self.config.borrow_mut().position = None;
        self.persist();
        self.saved_at.set(None);
        if let Some(ui) = self.ui.upgrade()
            && ui.window().is_visible()
        {
            self.place();
        }
    }

    /// Hears about the widget from elsewhere — the interface keeps the tray's
    /// checkbox and the preferences rows in step with what actually happened.
    pub fn on_event(&self, listener: Rc<dyn Fn(Event)>) {
        self.events.borrow_mut().push(listener);
    }

    fn announce(&self, event: Event) {
        for listener in self.events.borrow().iter() {
            listener(event);
        }
    }

    /// Writes the position if it has moved since the last write, or always with
    /// `force`. Called from the ✕, the closing path, and a quiet half-minute
    /// timer; a drag lands here through `schedule_save` instead.
    pub fn save_position(&self, force: bool) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        if !ui.window().is_visible() {
            return;
        }
        let PhysicalPosition { x, y } = ui.window().position();
        let position = (x, y);
        if !force && self.saved_at.get() == Some(position) {
            return;
        }
        self.saved_at.set(Some(position));
        self.config.borrow_mut().position = Some([x, y]);
        self.persist();
    }

    /// A drag just moved the card: write where it settled, once, a moment after
    /// the last report. The pointer holds no "released" event the compositor
    /// hands back, so this is the honest end of the story.
    fn schedule_save(self: &Rc<Self>) {
        // A weak self: the timer outlives the panel only as a callback that
        // finds nothing to save.
        let weak = Rc::downgrade(self);
        self.save_timer.start(
            slint::TimerMode::SingleShot,
            std::time::Duration::from_millis(600),
            move || {
                if let Some(panel) = weak.upgrade() {
                    panel.save_position(false);
                }
            },
        );
    }

    /// Shows the window and puts it where it was last dragged — or at a
    /// desktop corner the first time. It takes no focus: it is a gadget.
    fn show(self: &Rc<Self>) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        if let Err(error) = ui.show() {
            tracing::warn!(%error, "the widget window would not show");
            return;
        }
        self.arm_backdrop();
        self.place();
        // The monitor a window belongs to is not always known the moment it is
        // shown; once the event loop has turned the wheel, place it again. A
        // card that landed on the primary monitor in between moves quietly.
        let weak = Rc::downgrade(self);
        slint::Timer::single_shot(std::time::Duration::from_millis(60), move || {
            if let Some(panel) = weak.upgrade() {
                panel.apply_on_top();
                panel.place();
            }
        });
    }

    /// Puts the card where the settings file says, clamped into the monitor it
    /// last sat on so a removed screen cannot lose it off the edge of the world.
    fn place(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let saved = self.config.borrow().position;
        ui.window().with_winit_window(|window| {
            use slint::winit_030::winit::dpi::PhysicalPosition as WinitPosition;
            let monitor = window
                .current_monitor()
                .or_else(|| window.primary_monitor());
            let Some(monitor) = monitor else {
                return;
            };
            let size = window.outer_size();
            let scale = window.scale_factor();
            let margin = (24.0 * scale) as i32;
            let fallback = [
                monitor.position().x + monitor.size().width as i32 - size.width as i32 - margin,
                monitor.position().y + monitor.size().height as i32 - size.height as i32 - margin,
            ];
            let position = saved
                .map(|[x, y]| {
                    let within = |value: i32, room: i32| value.clamp(0, room.max(0));
                    let left = monitor.position().x;
                    let top = monitor.position().y;
                    [
                        left + within(x - left, monitor.size().width as i32 - size.width as i32),
                        top + within(y - top, monitor.size().height as i32 - size.height as i32),
                    ]
                })
                .unwrap_or(fallback);
            window.set_outer_position(WinitPosition::new(position[0], position[1]));
            self.saved_at.set(Some((position[0], position[1])));
        });
    }

    fn apply_on_top(&self) {
        let on_top = self.config.borrow().on_top;
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        ui.window().with_winit_window(|window| {
            window.set_window_level(if on_top {
                WindowLevel::AlwaysOnTop
            } else {
                WindowLevel::Normal
            });
        });
    }

    /// A desktop widget is not an application to the taskbar: it has no taskbar
    /// entry and no alt-tab stop, like every gadget that ever sat on a desktop.
    /// On Windows it never takes the focus either, so toggling it never yanks
    /// the keyboard out of whatever the user was typing.
    fn skip_taskbar(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        #[cfg(windows)]
        {
            use slint::winit_030::winit::platform::windows::WindowExtWindows;
            ui.window()
                .with_winit_window(|window| window.set_skip_taskbar(true));
            ui.window()
                .with_winit_window(crate::backdrop::set_no_activate);
        }
        let _ = &ui;
    }

    /// The system backdrop behind the card, exactly as the preferences sheet has
    /// it: the documented attribute on Windows 11 22H2+, the composition accent
    /// below that. Both are idempotent, so every show re-affirms them.
    #[cfg(windows)]
    fn arm_backdrop(&self) {
        if let Some(ui) = self.ui.upgrade()
            && ui
                .window()
                .with_winit_window(crate::backdrop::attach_window)
                .is_none()
        {
            tracing::warn!("the widget has no Win32 handle; it stays opaque");
        }
    }

    #[cfg(not(windows))]
    fn arm_backdrop(&self) {}

    fn load() -> WidgetConfig {
        let Some(path) = config_path() else {
            return WidgetConfig::default();
        };
        match std::fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice(&bytes) {
                Ok(config) => config,
                Err(error) => {
                    tracing::warn!(
                        %error,
                        path = %path.display(),
                        "the widget settings are unreadable; defaults win"
                    );
                    WidgetConfig::default()
                }
            },
            Err(_) => WidgetConfig::default(),
        }
    }

    fn persist(&self) {
        let Some(path) = config_path() else {
            return;
        };
        if let Some(parent) = path.parent()
            && let Err(error) = std::fs::create_dir_all(parent)
        {
            tracing::warn!(%error, "the widget settings have nowhere to live");
            return;
        }
        let json = match serde_json::to_string_pretty(&*self.config.borrow()) {
            Ok(json) => json,
            Err(error) => {
                tracing::warn!(%error, "the widget settings would not serialize");
                return;
            }
        };
        if let Err(error) = std::fs::write(&path, json) {
            tracing::warn!(%error, path = %path.display(), "the widget settings would not save");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_ask_for_the_widget_shown_on_top_and_mostly_solid() {
        let config = WidgetConfig::default();
        assert!(config.enabled);
        assert!(config.on_top);
        assert_eq!(config.opacity, 85);
        assert_eq!(config.scale, 100);
        assert_eq!(config.look(), "auto");
        assert_eq!(config.position, None);
    }

    #[test]
    fn an_unreadable_look_is_auto() {
        let config: WidgetConfig = serde_json::from_str(r#"{"look": "sepia"}"#).unwrap();
        assert_eq!(config.look(), "auto");
        let config: WidgetConfig = serde_json::from_str(r#"{"look": "dark"}"#).unwrap();
        assert_eq!(config.look(), "dark");
    }

    #[test]
    fn a_size_never_lands_between_the_choices() {
        let panel_scale = |asked: i32| {
            SCALES
                .iter()
                .copied()
                .min_by_key(|choice| (choice - asked).abs())
                .unwrap_or_default()
        };
        assert_eq!(panel_scale(90), 90);
        assert_eq!(panel_scale(95), 90);
        assert_eq!(panel_scale(100), 100);
        assert_eq!(panel_scale(107), 100);
        assert_eq!(panel_scale(115), 115);
        assert_eq!(panel_scale(130), 115);
    }

    #[test]
    fn a_setting_file_without_the_new_fields_still_loads() {
        let config: WidgetConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(config, WidgetConfig::default());
    }

    #[test]
    fn the_position_survives_a_round_trip() {
        let config: WidgetConfig =
            serde_json::from_str(r#"{"enabled": true, "position": [100, 200]}"#).unwrap();
        assert!(config.enabled);
        assert_eq!(config.position, Some([100, 200]));
    }
}
