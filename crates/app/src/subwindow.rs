//! Secondary windows (settings, keyboard shortcuts, about): always above the main
//! window and never a separate entry in the window switchers.
//!
//! - **macOS:** windows of the system, made child windows of the main one: above it,
//!   moved with it, left out of Cmd+` (window cycling), Mission Control and the Window
//!   menu. Cmd+Tab lists apps, so it shows Watershed once (with or without a `.app`).
//! - **Windows:** windows of the system owned by the main one: above it, minimized
//!   with it, no taskbar button, not in Alt+Tab.
//! - **Linux:** egui windows inside the main one. winit has no way to make a window
//!   the child (transient) of another on X11 or Wayland, and only that keeps a window
//!   above its parent and out of the switchers there.
//!
//! They are modal: while one is open, the main window takes no clicks or keys. A click
//! there beeps and brings the open window to the front (macOS: `hold_main_input`;
//! Windows: the system does it for a disabled owner; Linux: an egui modal, which dims
//! the main window).
//!
//! The system windows are immediate egui viewports: drawn in the same frame as the
//! main window, with the app's state at hand. They fit their contents. egui does not
//! hand out a viewport's native handle, so `owner::attach` finds the window by title.

use eframe::egui;

/// The space around the contents.
#[cfg(any(target_os = "macos", windows))]
const MARGIN: f32 = 16.0;
/// For how long after opening a window is kept centered on the main one, while its
/// contents settle its size.
#[cfg(any(target_os = "macos", windows))]
const CENTER_SECS: f64 = 0.5;

/// The window's id, also used to focus it.
pub fn id(name: &str) -> egui::ViewportId {
    egui::ViewportId::from_hash_of(("subwindow", name))
}

/// Brings the window to the front, when it is open already.
pub fn focus(ctx: &egui::Context, name: &str) {
    ctx.send_viewport_cmd_to(id(name), egui::ViewportCommand::Focus);
}

/// How the window is sized.
#[derive(Clone, Copy)]
pub enum Width {
    /// A fixed width; text wraps in it and the height follows.
    Fixed(f32),
    /// As wide as the contents, which do not wrap.
    Fit,
}

/// Shows the window while `open`; closing it (its close button, Esc, Cmd/Ctrl+W)
/// clears `open`. Call it every frame from the main window.
pub fn show(
    ctx: &egui::Context,
    name: &str,
    title: &str,
    width: Width,
    open: &mut bool,
    contents: impl FnOnce(&mut egui::Ui),
) {
    #[cfg(not(any(target_os = "macos", windows)))]
    if *open {
        let modal = egui::Modal::new(egui::Id::new(("subwindow", name))).show(ctx, |ui| {
            if let Width::Fixed(w) = width {
                ui.set_width(w);
            } else {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            }
            let close = ui
                .horizontal(|ui| {
                    ui.heading(title);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.button("Close").clicked()
                    })
                    .inner
                })
                .inner;
            ui.separator();
            contents(ui);
            close
        });
        // A click on the dimmed main window does not close it, as on the other systems.
        let escape = modal.is_top_modal
            && !modal.any_popup_open
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        if modal.inner || escape {
            *open = false;
        }
    }
    #[cfg(any(target_os = "macos", windows))]
    native(ctx, name, title, width, open, contents);
}

#[cfg(any(target_os = "macos", windows))]
fn native(
    ctx: &egui::Context,
    name: &str,
    title: &str,
    width: Width,
    open: &mut bool,
    contents: impl FnOnce(&mut egui::Ui),
) {
    let viewport = id(name);
    let place = egui::Id::new(("subwindow-place", name));
    if !*open {
        ctx.data_mut(|d| d.remove::<(egui::Pos2, f64)>(place));
        return;
    }

    let start_width = match width {
        Width::Fixed(w) => w,
        Width::Fit => 480.0,
    };
    let start = egui::vec2(start_width, 320.0);
    // Where it opens: centered on the main window. Kept in the builder while it is
    // open, since a builder that changes moves the window; once its contents give it
    // its size, `fit` centers it again.
    // (egui's locks do not nest: read the main window before taking `data_mut`.)
    let main = ctx.input(|i| i.viewport().outer_rect);
    let opening = main.map_or(egui::pos2(120.0, 120.0), |r| r.center() - start / 2.0);
    let now = ctx.input(|i| i.time);
    let (position, opened_at) = ctx.data_mut(|d| *d.get_temp_mut_or(place, (opening, now)));
    // Only right after opening: later the user may have moved it.
    let center_on = main.filter(|_| now - opened_at < CENTER_SECS);

    let builder = egui::ViewportBuilder::default()
        .with_title(title)
        .with_inner_size(start)
        .with_position(position)
        .with_resizable(false)
        .with_minimize_button(false)
        .with_maximize_button(false)
        .with_taskbar(false);
    let mut contents = Some(contents);
    let close = ctx.show_viewport_immediate(viewport, builder, |ui, class| {
        let mut draw = |ui: &mut egui::Ui| {
            if let Width::Fit = width {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            }
            if let Some(contents) = contents.take() {
                contents(ui);
            }
        };

        if class == egui::ViewportClass::EmbeddedWindow {
            // No native windows on this platform: egui wrapped us in a window.
            draw(ui);
            return false;
        }

        let frame = egui::Frame::central_panel(ui.style()).inner_margin(MARGIN);
        let used = egui::CentralPanel::default()
            .frame(frame)
            .show(ui, |ui| ui.scope(|ui| draw(ui)).response.rect)
            .inner;
        fit(ui.ctx(), width, used, center_on);

        owner::attach(title);

        ui.ctx().input_mut(|i| {
            i.viewport().close_requested()
                || i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
                || i.consume_key(egui::Modifiers::COMMAND, egui::Key::W)
        })
    });
    if close {
        #[cfg(windows)]
        owner::release();
        *open = false;
        ctx.data_mut(|d| d.remove::<(egui::Pos2, f64)>(place));
    }
}

/// While the secondary window `open` is open, the main window takes no clicks or keys:
/// they are dropped from its input, and a click or key press beeps and brings `open`
/// to the front, as with a modal window. For `App::raw_input_hook` (macOS: on Windows
/// the system does it for the disabled owner, on Linux the egui modal does it).
#[cfg(target_os = "macos")]
pub fn hold_main_input(ctx: &egui::Context, raw: &mut egui::RawInput, open: &str) {
    use egui::Event as E;
    if raw.viewport_id != egui::ViewportId::ROOT {
        return;
    }
    let refused = raw.events.iter().any(|e| {
        matches!(
            e,
            E::PointerButton { pressed: true, .. } | E::Key { pressed: true, .. }
        )
    });
    let before = raw.events.len();
    raw.events.retain(|e| {
        !matches!(
            e,
            E::PointerButton { .. }
                | E::PointerMoved(_)
                | E::MouseMoved(_)
                | E::MouseWheel { .. }
                | E::Key { .. }
                | E::Text(_)
                | E::Zoom(_)
                | E::Touch { .. }
                | E::Copy
                | E::Cut
                | E::Paste(_)
                | E::Ime(_)
        )
    });
    if raw.events.len() != before {
        // No hover left behind.
        raw.events.push(E::PointerGone);
    }
    if refused {
        objc2_app_kit::NSBeep();
        focus(ctx, open);
    }
}

/// Resizes the window to its contents, when they changed size, and centers it on
/// `center_on` (the main window's outer rect) if given.
#[cfg(any(target_os = "macos", windows))]
fn fit(ctx: &egui::Context, width: Width, used: egui::Rect, center_on: Option<egui::Rect>) {
    let (Some(inner), Some(outer)) =
        ctx.input(|i| (i.viewport().inner_rect, i.viewport().outer_rect))
    else {
        return;
    };
    let wanted = egui::vec2(
        match width {
            Width::Fixed(w) => w,
            Width::Fit => used.max.x + MARGIN,
        },
        used.max.y + MARGIN,
    )
    .ceil();
    if (wanted - inner.size()).abs().max_elem() > 0.5 {
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(wanted));
    }
    if let Some(main) = center_on {
        // The title bar and borders stay as they are.
        let size = wanted + (outer.size() - inner.size());
        let position = (main.center() - size / 2.0).round();
        if (position - outer.min).abs().max_elem() > 0.5 {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(position));
        }
    }
}

/// The secondary windows become child windows of the main one (macOS).
#[cfg(target_os = "macos")]
pub mod owner {
    use std::sync::atomic::{AtomicIsize, Ordering};

    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApp, NSView, NSWindowCollectionBehavior, NSWindowOrderingMode};

    /// The main window's number (0: unknown).
    static MAIN: AtomicIsize = AtomicIsize::new(0);

    /// Remembers the main window, from the app's creation.
    pub fn init(cc: &eframe::CreationContext) {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        if let Ok(handle) = cc.window_handle()
            && let RawWindowHandle::AppKit(handle) = handle.as_raw()
        {
            // Safety: the handle's view is the main window's live NSView, used on the
            // main thread (the app is created there).
            let view = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
            if let Some(window) = view.window() {
                MAIN.store(window.windowNumber(), Ordering::Relaxed);
            }
        }
    }

    /// Makes the window titled `title` a child of the main window, if it is not one
    /// yet: above the main window, moved with it, out of Cmd+`, Mission Control and the
    /// Window menu.
    pub fn attach(title: &str) {
        let main = MAIN.load(Ordering::Relaxed);
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        if main == 0 {
            return;
        }
        let windows = NSApp(mtm).windows();
        let parent = windows.iter().find(|w| w.windowNumber() == main);
        let child = windows
            .iter()
            .find(|w| w.windowNumber() != main && w.title().to_string() == title);
        let (Some(parent), Some(child)) = (parent, child) else {
            return;
        };
        if child.parentWindow().is_some() {
            return;
        }
        child.setCollectionBehavior(
            child.collectionBehavior()
                | NSWindowCollectionBehavior::IgnoresCycle
                | NSWindowCollectionBehavior::Transient,
        );
        child.setExcludedFromWindowsMenu(true);
        // Safety: both are live windows of this app, on the main thread.
        unsafe { parent.addChildWindow_ordered(&child, NSWindowOrderingMode::Above) };
    }
}

/// The main window owns the secondary ones (Windows).
#[cfg(windows)]
pub mod owner {
    use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};

    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumThreadWindows, GWLP_HWNDPARENT, GetWindowLongPtrW, GetWindowTextW, SetWindowLongPtrW,
    };

    static MAIN: AtomicIsize = AtomicIsize::new(0);
    /// Whether the main window is disabled by a secondary one.
    static DISABLED: AtomicBool = AtomicBool::new(false);

    /// Remembers the main window, from the app's creation.
    pub fn init(cc: &eframe::CreationContext) {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        if let Ok(handle) = cc.window_handle()
            && let RawWindowHandle::Win32(handle) = handle.as_raw()
        {
            MAIN.store(handle.hwnd.get(), Ordering::Relaxed);
        }
    }

    struct Search {
        title: Vec<u16>,
        found: HWND,
    }

    /// Makes the main window the owner of the window titled `title`, if it is not
    /// owned yet. egui does not hand out a viewport's handle, so it is found by its
    /// title among this thread's windows (winit makes them all on the main thread).
    pub fn attach(title: &str) {
        let main = MAIN.load(Ordering::Relaxed);
        if main == 0 {
            return;
        }
        let mut search = Search {
            title: title.encode_utf16().collect(),
            found: std::ptr::null_mut(),
        };
        // Safety: the callback only reads windows of this thread and writes `search`,
        // which outlives the call.
        unsafe {
            EnumThreadWindows(
                GetCurrentThreadId(),
                Some(matches),
                &mut search as *mut Search as LPARAM,
            );
        }
        let window = search.found;
        if window.is_null() || window as isize == main {
            return;
        }
        // Safety: `window` and `main` are live windows of this thread.
        unsafe {
            if GetWindowLongPtrW(window, GWLP_HWNDPARENT) == 0 {
                SetWindowLongPtrW(window, GWLP_HWNDPARENT, main);
                // Modal: a disabled owner takes no input; a click on it beeps and
                // flashes the window it owns.
                EnableWindow(main as HWND, 0);
                DISABLED.store(true, Ordering::Relaxed);
            }
        }
    }

    /// Enables the main window again, before the secondary one closes (else Windows
    /// activates another app's window).
    pub fn release() {
        let main = MAIN.load(Ordering::Relaxed);
        if main != 0 && DISABLED.swap(false, Ordering::Relaxed) {
            // Safety: `main` is the live main window, on its thread.
            unsafe { EnableWindow(main as HWND, 1) };
        }
    }

    unsafe extern "system" fn matches(window: HWND, data: LPARAM) -> i32 {
        // Safety: `data` is the `Search` passed by `attach`.
        let search = unsafe { &mut *(data as *mut Search) };
        let mut text = [0u16; 256];
        // Safety: the buffer is as long as said.
        let len = unsafe { GetWindowTextW(window, text.as_mut_ptr(), text.len() as i32) };
        if len > 0 && text[..len as usize] == search.title[..] {
            search.found = window;
            return 0; // stop
        }
        1
    }
}
