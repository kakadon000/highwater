//! The Windows 10/11 acrylic that the preferences sheet floats on.
//!
//! The sheet paints itself translucent on the composited path (the Direct3D 12
//! presentation that keeps the surface's alpha), and DWM fills the rest of every
//! such pixel with its own backdrop. Windows 11 22H2 and newer take the documented
//! system backdrop; older builds fall back to the composition attribute that acrylic
//! applications have driven since Windows 10. Either way the backdrop only shows
//! where the window is not opaque, so the main view keeps its solid look and the
//! effect exists only behind the sheet.
//!
//! Applied when preferences open, through the same winit window the rest of the
//! client talks to. Both calls are idempotent, so a later open re-affirms what the
//! first one set.

// Raw DWM calls and an undocumented user32 export: the audited exception this module
// exists for. The workspace denies unsafe everywhere else (workspace lints).
#![allow(unsafe_code)]

use core::ffi::c_void;
use core::mem::size_of;

use slint::ComponentHandle;
use slint::winit_030::WinitWindowAccessor;
use slint::winit_030::winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::winit_030::winit::window::Window;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{
    DWM_SYSTEMBACKDROP_TYPE, DWMSBT_TRANSIENTWINDOW, DWMWA_SYSTEMBACKDROP_TYPE,
    DwmSetWindowAttribute,
};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetWindowLongPtrW, SetWindowLongPtrW, WS_EX_NOACTIVATE,
};
use windows::core::{s, w};

use crate::AppWindow;

/// `WCA_ACCENT_POLICY`: the composition attribute that carries the Windows 10 acrylic.
const WCA_ACCENT_POLICY: u32 = 26;
/// `ACCENT_ENABLE_ACRYLICBLURBEHIND`.
const ACCENT_ENABLE_ACRYLICBLURBEHIND: u32 = 4;
/// The sheet's dark colour (`#222226`) at a third of its opacity, in the attribute's
/// ABGR packing. The acrylic keeps most of its blur; the translucent sheet above it
/// supplies the rest of the tint.
const ACCENT_TINT: u32 = 0x59262222;

/// Arms the backdrop on the window the preferences sheet opens in.
pub fn attach(ui: &AppWindow) {
    if ui.window().with_winit_window(attach_window) != Some(true) {
        tracing::warn!(
            "the preferences sheet stays opaque: no Win32 handle, or the backdrop was refused"
        );
    }
}

/// Arms the backdrop on any one winit window: the preferences sheet and the
/// desktop widget float over the same system acrylic. True when a backdrop
/// path took — a false means the surface keeps nothing behind it, and a card
/// that counts on translucency had better draw opaque instead.
pub fn attach_window(window: &Window) -> bool {
    let Some(hwnd) = hwnd(window) else {
        tracing::warn!("the window has no Win32 handle; it stays opaque");
        return false;
    };
    // SAFETY: DWM attribute writes on a window this process owns. A build older than
    // Windows 11 22H2 rejects the attribute and takes the accent route below instead.
    let backdrop = DWM_SYSTEMBACKDROP_TYPE(DWMSBT_TRANSIENTWINDOW.0);
    let armed = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &backdrop as *const DWM_SYSTEMBACKDROP_TYPE as *const c_void,
            size_of::<DWM_SYSTEMBACKDROP_TYPE>() as u32,
        )
    };
    if armed.is_err() {
        // SAFETY: see `acrylic_behind`.
        unsafe { acrylic_behind(hwnd) }
    } else {
        true
    }
}

/// The Windows 10 route: the accent policy draws the same blur behind the window.
/// The API is undocumented; its shape has been stable since 2015 and is what every
/// acrylic application on Windows 10 drives. Windows ships the export in user32.dll
/// but not in user32.lib, so it is resolved by name at runtime — which is also why
/// the static link cannot be the way in.
///
/// # Safety
/// A user32 call on a window this process owns, with pointers to stack data the call
/// copies out before it returns.
unsafe fn acrylic_behind(hwnd: HWND) -> bool {
    #[repr(C)]
    struct AccentPolicy {
        accent_state: u32,
        // The flag every acrylic driver sets: the blur covers the whole window rather
        // than the borders alone.
        accent_flags: u32,
        gradient_color: u32,
        animation_id: i32,
    }

    #[repr(C)]
    struct CompositionAttributeData {
        attribute: u32,
        data: *mut c_void,
        size: usize,
    }

    type SetCompositionAttribute =
        unsafe extern "system" fn(HWND, *const CompositionAttributeData) -> i32;

    let policy = AccentPolicy {
        accent_state: ACCENT_ENABLE_ACRYLICBLURBEHIND,
        accent_flags: 2,
        gradient_color: ACCENT_TINT,
        animation_id: 0,
    };
    let data = CompositionAttributeData {
        attribute: WCA_ACCENT_POLICY,
        data: &policy as *const AccentPolicy as *mut c_void,
        size: size_of::<AccentPolicy>(),
    };
    // SAFETY: user32 is loaded in every GUI process and never unloaded here.
    let module = unsafe { GetModuleHandleW(w!("user32.dll")) };
    let Ok(module) = module else {
        tracing::debug!("user32 is not loaded, which a GUI process does not do; stopping");
        return false;
    };
    // SAFETY: a plain export lookup by name.
    let address = unsafe { GetProcAddress(module, s!("SetWindowCompositionAttribute")) };
    let Some(address) = address else {
        tracing::debug!("this user32 has no composition attribute export; staying opaque");
        return false;
    };
    // SAFETY: the address came from GetProcAddress on user32, and the signature is the
    // one the export has shipped with since Windows 10 1507.
    let set = unsafe {
        core::mem::transmute::<unsafe extern "system" fn() -> isize, SetCompositionAttribute>(
            address,
        )
    };
    // SAFETY: see the function contract. A zero return means no blur: the sheet's own
    // translucency then lands on the plain window, which is the pre-2016 look and
    // readable all the same.
    if unsafe { set(hwnd, &data) } == 0 {
        tracing::debug!("the Windows 10 accent route refused the acrylic; staying opaque");
        return false;
    }
    true
}

/// The window's Win32 handle, through winit's raw-window-handle bridge.
fn hwnd(window: &Window) -> Option<HWND> {
    let handle = window.window_handle().ok()?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Some(HWND(handle.hwnd.get() as *mut c_void)),
        _ => None,
    }
}

/// A desktop gadget never takes the focus: clicking the widget must not yank
/// the keyboard out of whatever the user was typing. winit offers no focusable
/// switch on an existing window, so this sets the style such a gadget wears —
/// `WS_EX_NOACTIVATE` — straight onto the Win32 window. Idempotent: the style
/// bit is set again, not toggled.
pub fn set_no_activate(window: &Window) {
    let Some(hwnd) = hwnd(window) else {
        return;
    };
    // SAFETY: a style write on a window this process owns; GetWindowLongPtrW
    // reads the very style the next call rewrites, so the other bits survive.
    unsafe {
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_NOACTIVATE.0 as isize);
    }
}
