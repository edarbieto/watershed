//! The app's menus: File, Edit, Playback, View, Help (and the Watershed and Window
//! menus on macOS). One model, shown natively where the system has menus: the menu
//! bar on macOS, the window's menu bar on Windows (both through `muda`). On Linux
//! the app draws the same menus in a bar of its own.
//!
//! Every command in the menus also exists somewhere in the window or as a key; the
//! menus make them discoverable. Keys without Cmd / Ctrl (Space, A, M, …) are the
//! app's, so they still type into text fields; the menus only show them (macOS as key
//! equivalents, which the window gets first; Windows as text).

use std::path::{Path, PathBuf};

use eframe::egui::{self, Key, KeyboardShortcut, Modifiers};

use crate::stems::song_title;

/// Recent songs listed in File > Open Recent.
const RECENT_ITEMS: usize = 10;
/// The project's page, and where to report problems.
pub const PROJECT_URL: &str = "https://github.com/edarbieto/watershed";
pub const ISSUES_URL: &str = "https://github.com/edarbieto/watershed/issues/new";

/// What a menu item does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MenuCommand {
    About,
    CheckForUpdates,
    Settings,
    Quit,
    OpenFolder,
    SeparateSong,
    /// The recent song at this index of the config's list.
    OpenRecent(usize),
    ClearRecent,
    Reload,
    ShowInFolder,
    Undo,
    Redo,
    AddMarker,
    AddChange,
    PreviousTrack,
    NextTrack,
    MuteTrack,
    SoloTrack,
    DeleteTrack,
    RestoreTracks,
    SetA,
    SetB,
    ClearLoop,
    SaveLoop,
    PlayPause,
    PlayStop,
    Stop,
    ToStart,
    Back,
    Forward,
    Loop,
    Click,
    CountIn,
    TapTempo,
    ResetTempo,
    ResetPitch,
    TempoUp,
    TempoDown,
    PitchUp,
    PitchDown,
    ZoomIn,
    ZoomOut,
    ZoomToFit,
    Grid,
    Snap,
    Dynamic,
    Chords,
    SidePanel,
    Shortcuts,
    Website,
    ReportProblem,
}

/// How an item looks now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status {
    pub enabled: bool,
    /// For a check item: whether it is checked.
    pub checked: bool,
}

/// The key of an item.
#[derive(Clone, Copy)]
enum Keys {
    None,
    /// With Cmd (Ctrl elsewhere) or a function key: the native menu takes it on
    /// macOS; the app handles it elsewhere.
    Shortcut(KeyboardShortcut),
    /// A key without Cmd / Ctrl (Shift at most), which the app handles itself, so it
    /// still types into text fields. The menus only show it: macOS gives such key
    /// equivalents to the window first (checked), Windows shows it as text.
    Key(KeyboardShortcut),
}

#[derive(Clone, Copy)]
enum Entry {
    Item(MenuCommand, &'static str, Keys),
    Check(MenuCommand, &'static str, Keys),
    Separator,
    /// File > Open Recent.
    Recent,
    /// macOS's own Hide, Hide Others, Show All, Minimize, Zoom, Full Screen.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    System(System),
}

#[derive(Clone, Copy)]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
enum System {
    Hide,
    HideOthers,
    ShowAll,
    Minimize,
    Zoom,
    FullScreen,
}

const fn key(key: Key) -> Keys {
    Keys::Key(KeyboardShortcut::new(Modifiers::NONE, key))
}

const fn shift(key: Key) -> Keys {
    Keys::Key(KeyboardShortcut::new(Modifiers::SHIFT, key))
}

/// The key that deletes: ⌫ on a Mac keyboard, Delete elsewhere (the app takes both).
const DELETE: Key = if cfg!(target_os = "macos") {
    Key::Backspace
} else {
    Key::Delete
};

const fn cmd(key: Key) -> Keys {
    Keys::Shortcut(KeyboardShortcut::new(Modifiers::COMMAND, key))
}

const fn cmd_alt(key: Key) -> Keys {
    Keys::Shortcut(KeyboardShortcut::new(
        Modifiers {
            alt: true,
            ..Modifiers::COMMAND
        },
        key,
    ))
}

const fn cmd_shift(key: Key) -> Keys {
    Keys::Shortcut(KeyboardShortcut::new(
        Modifiers {
            shift: true,
            ..Modifiers::COMMAND
        },
        key,
    ))
}

/// The menus, in order: a title and its entries.
fn menus() -> Vec<(&'static str, Vec<Entry>)> {
    use Entry::{Check, Item, Separator};
    use MenuCommand as C;
    let mut file = vec![
        Item(C::OpenFolder, "Open Folder…", cmd(Key::O)),
        Item(C::SeparateSong, "Separate Song…", cmd_shift(Key::O)),
        Entry::Recent,
        Separator,
        Item(C::Reload, "Reload Folder", cmd(Key::R)),
        Item(C::ShowInFolder, SHOW_IN_FOLDER, cmd_alt(Key::R)),
    ];
    let edit = vec![
        Item(C::Undo, "Undo", cmd(Key::Z)),
        Item(C::Redo, "Redo", cmd_shift(Key::Z)),
        Separator,
        Item(C::AddMarker, "Add Marker", shift(Key::M)),
        Item(
            C::AddChange,
            "Add Tempo or Time Signature Change",
            shift(Key::C),
        ),
        Separator,
        Item(C::PreviousTrack, "Select Previous Track", key(Key::ArrowUp)),
        Item(C::NextTrack, "Select Next Track", key(Key::ArrowDown)),
        Check(C::MuteTrack, "Mute Track", key(Key::M)),
        Check(C::SoloTrack, "Solo Track", key(Key::S)),
        Item(C::DeleteTrack, "Delete Track", key(DELETE)),
        Item(C::RestoreTracks, "Restore Deleted Tracks", Keys::None),
        Separator,
        Item(C::SetA, "Set Loop Start", key(Key::A)),
        Item(C::SetB, "Set Loop End", key(Key::B)),
        Item(C::ClearLoop, "Clear Loop", shift(Key::L)),
        Item(C::SaveLoop, "Save Loop", cmd(Key::L)),
    ];
    let playback = vec![
        Item(C::PlayPause, "Play / Pause", key(Key::Enter)),
        Item(C::PlayStop, "Play / Return", key(Key::Space)),
        Item(C::Stop, "Stop", Keys::None),
        Separator,
        Item(C::ToStart, "To Start", key(Key::Home)),
        Item(C::Back, "Back", key(Key::ArrowLeft)),
        Item(C::Forward, "Forward", key(Key::ArrowRight)),
        Separator,
        Check(C::Loop, "Loop", key(Key::L)),
        Check(C::Click, "Click", key(Key::C)),
        Check(C::CountIn, "Count-in", key(Key::K)),
        Separator,
        Item(C::TapTempo, "Tap Tempo", key(Key::T)),
        Item(C::TempoUp, "Faster (+5 %)", shift(Key::ArrowRight)),
        Item(C::TempoDown, "Slower (−5 %)", shift(Key::ArrowLeft)),
        Item(C::ResetTempo, "Reset Tempo", shift(Key::T)),
        Separator,
        Item(C::PitchUp, "Pitch Up a Semitone", shift(Key::ArrowUp)),
        Item(C::PitchDown, "Pitch Down a Semitone", shift(Key::ArrowDown)),
        Item(C::ResetPitch, "Reset Pitch", shift(Key::P)),
    ];
    let view = vec![
        Item(C::ZoomIn, "Zoom In", key(Key::Plus)),
        Item(C::ZoomOut, "Zoom Out", key(Key::Minus)),
        Item(C::ZoomToFit, "Zoom to Fit", cmd(Key::Num0)),
        Separator,
        Check(C::Grid, "Beat Grid", shift(Key::G)),
        Check(C::Snap, "Snap to Grid", key(Key::G)),
        Check(C::Dynamic, "Dynamic Tempo", key(Key::D)),
        Check(C::Chords, "Chords", key(Key::H)),
        Separator,
        Check(C::SidePanel, "Markers and Loops Panel", cmd_alt(Key::S)),
    ];
    let mut help = vec![
        Item(C::Shortcuts, "Keyboard Shortcuts", key(Key::Questionmark)),
        Item(C::Website, "Watershed on GitHub", Keys::None),
        Item(C::ReportProblem, "Report a Problem…", Keys::None),
    ];
    let mut all = Vec::new();
    if cfg!(target_os = "macos") {
        all.push((
            "Watershed",
            vec![
                Item(C::About, "About Watershed", Keys::None),
                Item(C::CheckForUpdates, "Check for Updates…", Keys::None),
                Separator,
                Item(C::Settings, "Settings…", cmd(Key::Comma)),
                Separator,
                Entry::System(System::Hide),
                Entry::System(System::HideOthers),
                Entry::System(System::ShowAll),
                Separator,
                Item(C::Quit, "Quit Watershed", cmd(Key::Q)),
            ],
        ));
    } else {
        file.extend([
            Separator,
            Item(C::Settings, "Settings…", cmd(Key::Comma)),
            Separator,
            Item(C::Quit, "Quit", cmd(Key::Q)),
        ]);
        help.extend([
            Separator,
            Item(C::CheckForUpdates, "Check for Updates…", Keys::None),
            Item(C::About, "About Watershed", Keys::None),
        ]);
    }
    all.extend([
        ("File", file),
        ("Edit", edit),
        ("Playback", playback),
        ("View", view),
    ]);
    if cfg!(target_os = "macos") {
        all.push((
            "Window",
            vec![
                Entry::System(System::Minimize),
                Entry::System(System::Zoom),
                Separator,
                Entry::System(System::FullScreen),
            ],
        ));
    }
    all.push(("Help", help));
    all
}

const SHOW_IN_FOLDER: &str = if cfg!(target_os = "macos") {
    "Show in Finder"
} else if cfg!(windows) {
    "Show in Explorer"
} else {
    "Open the Song's Folder"
};

/// Every command an item has, for the app to give each its status.
pub fn commands(recent: usize) -> Vec<MenuCommand> {
    let mut list: Vec<MenuCommand> = menus()
        .into_iter()
        .flat_map(|(_, entries)| entries)
        .filter_map(|entry| match entry {
            Entry::Item(command, ..) | Entry::Check(command, ..) => Some(command),
            _ => None,
        })
        .collect();
    list.extend((0..recent.min(RECENT_ITEMS)).map(MenuCommand::OpenRecent));
    list.push(MenuCommand::ClearRecent);
    list
}

/// The command whose Cmd / Ctrl shortcut was just pressed, consumed; commands for
/// which `skip` is true are left alone (their keys go on to the widgets). On macOS
/// the native menu takes these keys before the app sees them.
pub fn shortcut_pressed(
    ctx: &egui::Context,
    skip: impl Fn(MenuCommand) -> bool,
) -> Option<MenuCommand> {
    let mut shortcuts: Vec<(KeyboardShortcut, MenuCommand)> = menus()
        .into_iter()
        .flat_map(|(_, entries)| entries)
        .filter_map(|entry| match entry {
            Entry::Item(command, _, Keys::Shortcut(shortcut))
            | Entry::Check(command, _, Keys::Shortcut(shortcut)) => {
                (!skip(command)).then_some((shortcut, command))
            }
            _ => None,
        })
        .collect();
    most_modifiers_first(&mut shortcuts);
    shortcuts.into_iter().find_map(|(shortcut, command)| {
        ctx.input_mut(|i| i.consume_shortcut(&shortcut))
            .then_some(command)
    })
}

/// The keys without Cmd / Ctrl that the app handles itself (see [`Keys::Key`]), with
/// their commands, the ones with Shift first.
pub fn key_commands() -> Vec<(KeyboardShortcut, MenuCommand)> {
    let mut keys: Vec<(KeyboardShortcut, MenuCommand)> = menus()
        .into_iter()
        .flat_map(|(_, entries)| entries)
        .filter_map(|entry| match entry {
            Entry::Item(command, _, Keys::Key(shortcut))
            | Entry::Check(command, _, Keys::Key(shortcut)) => Some((shortcut, command)),
            _ => None,
        })
        .collect();
    most_modifiers_first(&mut keys);
    keys
}

/// egui matches Shift and Alt loosely (Cmd+O also matches Shift+Cmd+O), so the
/// shortcuts with more modifiers must be tried first.
fn most_modifiers_first(shortcuts: &mut [(KeyboardShortcut, MenuCommand)]) {
    let count = |m: Modifiers| u8::from(m.shift) + u8::from(m.alt);
    shortcuts.sort_by_key(|(shortcut, _)| std::cmp::Reverse(count(shortcut.modifiers)));
}

/// The menus, native or drawn by the app.
pub struct Menus {
    #[cfg(any(target_os = "macos", windows))]
    native: Option<native::NativeMenu>,
    /// Commands chosen in the native menus, sent from their event handler.
    #[cfg_attr(not(any(target_os = "macos", windows)), allow(dead_code))]
    chosen: std::sync::mpsc::Receiver<MenuCommand>,
}

impl Menus {
    /// Installs the native menus where there are some.
    #[cfg_attr(not(any(target_os = "macos", windows)), allow(unused_variables))]
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let (sender, chosen) = std::sync::mpsc::channel();
        #[cfg(any(target_os = "macos", windows))]
        let native = native::NativeMenu::install(cc, sender);
        #[cfg(not(any(target_os = "macos", windows)))]
        drop(sender);
        Self {
            #[cfg(any(target_os = "macos", windows))]
            native,
            chosen,
        }
    }

    /// Whether the menus are the system's, so the app draws none.
    pub fn is_native(&self) -> bool {
        #[cfg(any(target_os = "macos", windows))]
        return self.native.is_some();
        #[cfg(not(any(target_os = "macos", windows)))]
        false
    }

    /// Light or dark menus, where the app colors them (the Windows menu bar; macOS
    /// draws its menus as the system is).
    #[cfg_attr(not(windows), allow(unused_variables, clippy::unused_self))]
    pub fn set_theme(&self, theme: egui::SystemTheme) {
        #[cfg(windows)]
        if let Some(native) = &self.native {
            native.set_theme(theme);
        }
    }

    /// The commands chosen in the native menus since the last call.
    pub fn chosen(&self) -> Vec<MenuCommand> {
        self.chosen.try_iter().collect()
    }

    /// Brings the native menus up to date: enabled and checked items, the recent
    /// songs. Only what changed is sent to the system.
    #[cfg_attr(
        not(any(target_os = "macos", windows)),
        allow(unused_variables, clippy::unused_self)
    )]
    pub fn update(&mut self, status: &dyn Fn(MenuCommand) -> Status, recent: &[PathBuf]) {
        #[cfg(any(target_os = "macos", windows))]
        if let Some(native) = &mut self.native {
            native.update(status, recent);
        }
    }

    /// The app's own menu bar, where the system has none. Returns the command
    /// chosen.
    pub fn bar(
        &self,
        ui: &mut egui::Ui,
        status: &dyn Fn(MenuCommand) -> Status,
        recent: &[PathBuf],
    ) -> Option<MenuCommand> {
        let mut chosen = None;
        egui::MenuBar::new().ui(ui, |ui| {
            for (title, entries) in menus() {
                ui.menu_button(title, |ui| {
                    for entry in entries {
                        if let Some(command) = draw_entry(ui, entry, status, recent) {
                            chosen = Some(command);
                            ui.close();
                        }
                    }
                });
            }
        });
        chosen
    }
}

/// One entry of the app's own menu bar; the command, when clicked.
fn draw_entry(
    ui: &mut egui::Ui,
    entry: Entry,
    status: &dyn Fn(MenuCommand) -> Status,
    recent: &[PathBuf],
) -> Option<MenuCommand> {
    let keys_text = |ui: &egui::Ui, keys: Keys| match keys {
        Keys::None => String::new(),
        Keys::Shortcut(shortcut) => ui.ctx().format_shortcut(&shortcut),
        Keys::Key(shortcut) => ui.ctx().format_shortcut(&shortcut),
    };
    match entry {
        Entry::Item(command, label, keys) => {
            let button = egui::Button::new(label).shortcut_text(keys_text(ui, keys));
            ui.add_enabled(status(command).enabled, button)
                .clicked()
                .then_some(command)
        }
        Entry::Check(command, label, keys) => {
            let Status { enabled, checked } = status(command);
            let button =
                egui::Button::selectable(checked, label).shortcut_text(keys_text(ui, keys));
            ui.add_enabled(enabled, button).clicked().then_some(command)
        }
        Entry::Separator => {
            ui.separator();
            None
        }
        Entry::Recent => {
            let mut chosen = None;
            ui.add_enabled_ui(!recent.is_empty(), |ui| {
                ui.menu_button("Open Recent", |ui| {
                    for (index, path) in recent.iter().take(RECENT_ITEMS).enumerate() {
                        let command = MenuCommand::OpenRecent(index);
                        if ui
                            .add_enabled(
                                status(command).enabled,
                                egui::Button::new(song_title(path)),
                            )
                            .on_hover_text(path.display().to_string())
                            .clicked()
                        {
                            chosen = Some(command);
                        }
                    }
                    ui.separator();
                    if ui.button("Clear Menu").clicked() {
                        chosen = Some(MenuCommand::ClearRecent);
                    }
                });
            });
            chosen
        }
        Entry::System(_) => None,
    }
}

/// Opens a folder in the system's file manager.
pub fn show_in_file_manager(folder: &Path) {
    let program = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    if let Err(e) = std::process::Command::new(program).arg(folder).spawn() {
        log::warn!("could not open {}: {e}", folder.display());
    }
}

#[cfg(any(target_os = "macos", windows))]
mod native {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::mpsc::Sender;

    use eframe::egui::{Key, KeyboardShortcut};
    #[cfg(target_os = "macos")]
    use muda::accelerator::KeyAccelerator;
    use muda::accelerator::{Accelerator, CMD_OR_CTRL, Code, Modifiers};
    use muda::{
        AboutMetadata, CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuId, MenuItem,
        PredefinedMenuItem, Submenu,
    };

    use super::{Entry, Keys, MenuCommand, RECENT_ITEMS, Status, System, menus};
    use crate::stems::song_title;

    const RECENT_PREFIX: &str = "recent:";
    const CLEAR_RECENT: &str = "recent-clear";

    enum Item {
        Normal(MenuItem),
        Check(CheckMenuItem),
    }

    pub struct NativeMenu {
        /// Kept alive: the system holds the menus only while this exists.
        _menu: Menu,
        items: Vec<(MenuCommand, Item)>,
        recent_menu: Option<Submenu>,
        recent_shown: Option<Vec<PathBuf>>,
        last: HashMap<MenuCommand, Status>,
        /// The main window, which holds the menu bar on Windows.
        #[cfg(windows)]
        hwnd: isize,
    }

    impl NativeMenu {
        /// Builds the menus and hands them to the system. Chosen commands go to
        /// `sender`, and wake the UI.
        pub fn install(cc: &eframe::CreationContext, sender: Sender<MenuCommand>) -> Option<Self> {
            let menu = Menu::new();
            let mut items = Vec::new();
            let mut ids = HashMap::new();
            let mut recent_menu = None;
            for (title, entries) in menus() {
                let submenu = Submenu::new(title, true);
                for entry in entries {
                    let item: Box<dyn IsMenuItem> = match entry {
                        // The system's own About window.
                        Entry::Item(MenuCommand::About, ..) => Box::new(about()),
                        Entry::Item(command, label, keys) => {
                            let item = MenuItem::new(text(label, keys), true, accelerator(keys));
                            #[cfg(target_os = "macos")]
                            show_key(keys, |k| item.set_key_accelerator(Some(k)));
                            ids.insert(item.id().clone(), command);
                            items.push((command, Item::Normal(item.clone())));
                            Box::new(item)
                        }
                        Entry::Check(command, label, keys) => {
                            let item = CheckMenuItem::new(
                                text(label, keys),
                                true,
                                false,
                                accelerator(keys),
                            );
                            #[cfg(target_os = "macos")]
                            show_key(keys, |k| item.set_key_accelerator(Some(k)));
                            ids.insert(item.id().clone(), command);
                            items.push((command, Item::Check(item.clone())));
                            Box::new(item)
                        }
                        Entry::Separator => Box::new(PredefinedMenuItem::separator()),
                        Entry::Recent => {
                            let recent = Submenu::new("Open Recent", true);
                            recent_menu = Some(recent.clone());
                            Box::new(recent)
                        }
                        Entry::System(system) => Box::new(system_item(system)),
                    };
                    if let Err(e) = submenu.append(item.as_ref()) {
                        log::warn!("menu: {e}");
                    }
                }
                if let Err(e) = menu.append(&submenu) {
                    log::warn!("menu: {e}");
                }
                #[cfg(target_os = "macos")]
                match title {
                    "Window" => submenu.set_as_windows_menu_for_nsapp(),
                    "Help" => submenu.set_as_help_menu_for_nsapp(),
                    _ => {}
                }
            }

            let ctx = cc.egui_ctx.clone();
            MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
                let id = event.id().0.as_str();
                let command = if id == CLEAR_RECENT {
                    Some(MenuCommand::ClearRecent)
                } else if let Some(index) = id.strip_prefix(RECENT_PREFIX) {
                    index.parse().ok().map(MenuCommand::OpenRecent)
                } else {
                    ids.get(event.id()).copied()
                };
                if let Some(command) = command {
                    let _ = sender.send(command);
                    ctx.request_repaint();
                }
            }));

            #[cfg(target_os = "macos")]
            {
                let _ = cc;
                menu.init_for_nsapp();
            }
            #[cfg(windows)]
            let hwnd = {
                use raw_window_handle::{HasWindowHandle, RawWindowHandle};
                let Ok(handle) = cc.window_handle() else {
                    log::warn!("menu: no window handle");
                    return None;
                };
                let RawWindowHandle::Win32(handle) = handle.as_raw() else {
                    return None;
                };
                let hwnd = handle.hwnd.get();
                // Safety: the window outlives the app, which owns the menu.
                let installed =
                    unsafe { menu.init_for_hwnd_with_theme(hwnd, muda::MenuTheme::Auto) };
                if let Err(e) = installed {
                    log::warn!("menu: {e}");
                    return None;
                }
                hwnd
            };
            Some(Self {
                _menu: menu,
                items,
                recent_menu,
                recent_shown: None,
                last: HashMap::new(),
                #[cfg(windows)]
                hwnd,
            })
        }

        #[cfg(windows)]
        pub fn set_theme(&self, theme: eframe::egui::SystemTheme) {
            let theme = match theme {
                eframe::egui::SystemTheme::Light => muda::MenuTheme::Light,
                eframe::egui::SystemTheme::Dark => muda::MenuTheme::Dark,
                eframe::egui::SystemTheme::SystemDefault => muda::MenuTheme::Auto,
            };
            // Safety: the window outlives the app, which owns the menu.
            if let Err(e) = unsafe { self._menu.set_theme_for_hwnd(self.hwnd, theme) } {
                log::warn!("menu: {e}");
            }
        }

        pub fn update(&mut self, status: &dyn Fn(MenuCommand) -> Status, recent: &[PathBuf]) {
            for (command, item) in &self.items {
                let now = status(*command);
                if self.last.get(command) == Some(&now) {
                    continue;
                }
                match item {
                    Item::Normal(item) => item.set_enabled(now.enabled),
                    Item::Check(item) => {
                        item.set_enabled(now.enabled);
                        item.set_checked(now.checked);
                    }
                }
                self.last.insert(*command, now);
            }
            let shown: Vec<PathBuf> = recent.iter().take(RECENT_ITEMS).cloned().collect();
            if self.recent_shown.as_ref() != Some(&shown)
                && let Some(menu) = &self.recent_menu
            {
                while menu.remove_at(0).is_some() {}
                for (index, path) in shown.iter().enumerate() {
                    let item = MenuItem::with_id(
                        format!("{RECENT_PREFIX}{index}"),
                        song_title(path),
                        path.is_dir(),
                        None,
                    );
                    let _ = menu.append(&item);
                }
                let _ = menu.append(&PredefinedMenuItem::separator());
                let _ = menu.append(&MenuItem::with_id(
                    MenuId::new(CLEAR_RECENT),
                    "Clear Menu",
                    !shown.is_empty(),
                    None,
                ));
                menu.set_enabled(!shown.is_empty());
                self.recent_shown = Some(shown);
            }
        }
    }

    fn system_item(system: System) -> PredefinedMenuItem {
        match system {
            System::Hide => PredefinedMenuItem::hide(None),
            System::HideOthers => PredefinedMenuItem::hide_others(None),
            System::ShowAll => PredefinedMenuItem::show_all(None),
            System::Minimize => PredefinedMenuItem::minimize(None),
            System::Zoom => PredefinedMenuItem::maximize(None),
            System::FullScreen => PredefinedMenuItem::fullscreen(None),
        }
    }

    /// The standard About window.
    fn about() -> PredefinedMenuItem {
        PredefinedMenuItem::about(
            None,
            Some(AboutMetadata {
                name: Some("Watershed".into()),
                version: Some(env!("CARGO_PKG_VERSION").into()),
                comments: Some(env!("CARGO_PKG_DESCRIPTION").into()),
                ..AboutMetadata::default()
            }),
        )
    }

    /// The label, with a key the app handles shown as text on Windows (a native
    /// accelerator there would take the key from text fields).
    fn text(label: &str, keys: Keys) -> String {
        match keys {
            Keys::Key(shortcut) if cfg!(windows) => {
                let names = eframe::egui::ModifierNames::NAMES;
                format!("{label}\t{}", shortcut.format(&names, false))
            }
            _ => label.to_owned(),
        }
    }

    /// Shows a key the app handles beside the item (macOS). AppKit offers a key
    /// without Cmd to the window before the menus, so the app still gets it, and text
    /// fields too; the menu never runs the item for it.
    #[cfg(target_os = "macos")]
    fn show_key(keys: Keys, set: impl FnOnce(KeyAccelerator) -> muda::Result<()>) {
        let Keys::Key(KeyboardShortcut {
            modifiers,
            logical_key,
        }) = keys
        else {
            return;
        };
        use muda::accelerator::{Key as K, NamedKey as N};
        let named = |key| Some(K::Named(key));
        let key = match logical_key {
            Key::ArrowUp => named(N::ArrowUp),
            Key::ArrowDown => named(N::ArrowDown),
            Key::ArrowLeft => named(N::ArrowLeft),
            Key::ArrowRight => named(N::ArrowRight),
            Key::Enter => named(N::Enter),
            Key::Home => named(N::Home),
            Key::Backspace => named(N::Backspace),
            Key::Delete => named(N::Delete),
            Key::F5 => named(N::F5),
            Key::Space => Some(K::Character(" ".into())),
            Key::Plus => Some(K::Character("+".into())),
            Key::Minus => Some(K::Character("-".into())),
            Key::Questionmark => Some(K::Character("?".into())),
            other => {
                let name = other.name();
                (name.chars().count() == 1).then(|| K::Character(name.to_lowercase()))
            }
        };
        let Some(key) = key else {
            log::warn!("menu: no key equivalent for {logical_key:?}");
            return;
        };
        let mods = if modifiers.shift {
            Modifiers::SHIFT
        } else {
            Modifiers::empty()
        };
        if let Err(e) = set(KeyAccelerator::new(mods, key)) {
            log::warn!("menu: {e}");
        }
    }

    /// The native shortcut for an item with Cmd / Ctrl.
    fn accelerator(keys: Keys) -> Option<Accelerator> {
        let Keys::Shortcut(KeyboardShortcut {
            modifiers,
            logical_key,
        }) = keys
        else {
            return None;
        };
        let code = match logical_key {
            Key::O => Code::KeyO,
            Key::Q => Code::KeyQ,
            Key::R => Code::KeyR,
            Key::L => Code::KeyL,
            Key::S => Code::KeyS,
            Key::Num0 => Code::Digit0,
            Key::Z => Code::KeyZ,
            Key::Comma => Code::Comma,
            _ => {
                log::warn!("menu: no accelerator for {logical_key:?}");
                return None;
            }
        };
        let mut mods = CMD_OR_CTRL;
        if modifiers.shift {
            mods |= Modifiers::SHIFT;
        }
        if modifiers.alt {
            mods |= Modifiers::ALT;
        }
        Some(Accelerator::new(mods, code))
    }
}
