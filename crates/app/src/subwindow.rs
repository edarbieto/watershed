//! Secondary windows (settings, keyboard shortcuts, about) as windows of the system,
//! with its title bar, instead of egui windows drawn inside the main one.
//!
//! They belong to the main window: one icon in the Dock and in Cmd+Tab on macOS (the
//! system groups windows by app). On Windows they are owned by the main window, so they
//! have no taskbar button, stay out of Alt+Tab, stay above the main window and minimize
//! with it. On Linux (X11) they are dialog windows.
//!
//! Each one is an immediate egui viewport: it is drawn in the same frame as the main
//! window, with the app's state at hand. The window fits its contents.

use eframe::egui;

/// The space around the contents.
const MARGIN: f32 = 16.0;

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
    let viewport = id(name);
    let place = egui::Id::new(("subwindow-place", name));
    if !*open {
        ctx.data_mut(|d| d.remove::<egui::Pos2>(place));
        return;
    }

    let start_width = match width {
        Width::Fixed(w) => w,
        Width::Fit => 480.0,
    };
    let start = egui::vec2(start_width, 320.0);
    // Where it opens: near the top of the main window, centered. Kept while it is
    // open, since a builder that changes moves the window.
    // (egui's locks do not nest: read the main window before taking `data_mut`.)
    let main = ctx.input(|i| i.viewport().outer_rect);
    let opening = main.map_or(egui::pos2(120.0, 120.0), |r| {
        egui::pos2(r.center().x - start.x / 2.0, r.top() + 80.0)
    });
    let position = ctx.data_mut(|d| *d.get_temp_mut_or(place, opening));

    #[allow(unused_mut)]
    let mut builder = egui::ViewportBuilder::default()
        .with_title(title)
        .with_inner_size(start)
        .with_position(position)
        .with_resizable(false)
        .with_minimize_button(false)
        .with_maximize_button(false)
        .with_taskbar(false);
    #[cfg(target_os = "linux")]
    {
        builder = builder.with_window_type(egui::X11WindowType::Dialog);
    }

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
        fit(ui.ctx(), width, used);

        #[cfg(windows)]
        owner::attach(title);

        ui.ctx().input_mut(|i| {
            i.viewport().close_requested()
                || i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
                || i.consume_key(egui::Modifiers::COMMAND, egui::Key::W)
        })
    });
    if close {
        *open = false;
        ctx.data_mut(|d| d.remove::<egui::Pos2>(place));
    }
}

/// Resizes the window to its contents, when they changed size.
fn fit(ctx: &egui::Context, width: Width, used: egui::Rect) {
    let Some(inner) = ctx.input(|i| i.viewport().inner_rect) else {
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
}

/// The main window owns the secondary ones (Windows only).
#[cfg(windows)]
pub mod owner {
    use std::sync::atomic::{AtomicIsize, Ordering};

    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumThreadWindows, GWLP_HWNDPARENT, GetWindowLongPtrW, GetWindowTextW, SetWindowLongPtrW,
    };

    static MAIN: AtomicIsize = AtomicIsize::new(0);

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
        // Safety: `window` is a live window of this thread.
        unsafe {
            if GetWindowLongPtrW(window, GWLP_HWNDPARENT) == 0 {
                SetWindowLongPtrW(window, GWLP_HWNDPARENT, main);
            }
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
