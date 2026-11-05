//! Updates from GitHub Releases: looks for a newer published release, downloads the
//! package for this system (verified with the SHA-256 GitHub lists for it), puts it
//! in place of the running app and starts the new version.
//!
//! What gets replaced depends on how the app was installed:
//! - macOS: the whole `.app` bundle, taken from the release's `.dmg`.
//! - Windows: `watershed.exe` (and the files next to it), from the release's `.zip`.
//!   A running `.exe` cannot be overwritten but it can be renamed, so the old one
//!   becomes `watershed.exe.old` and is deleted at the next start.
//! - Linux: the AppImage file (the `APPIMAGE` variable gives its path).
//!
//! The installers on every system are compiled everywhere (they only use `std` and
//! system tools), so a mistake in one shows up on any machine.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use eframe::egui;
use serde::Deserialize;

/// The GitHub repository releases come from.
const REPO: &str = "edarbieto/watershed";

/// The running version.
pub fn current() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).expect("the package version is x.y.z")
}

/// A release version, `major.minor.patch`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(u64, u64, u64);

impl Version {
    /// Reads `1.2.3` or `v1.2.3`. Pre-release versions (`1.2.3-beta`) are not read.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.strip_prefix('v').unwrap_or(text);
        let mut parts = text.split('.').map(|p| p.parse::<u64>().ok());
        let version = Self(parts.next()??, parts.next()??, parts.next()??);
        parts.next().is_none().then_some(version)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

/// A newer release.
#[derive(Clone, Debug)]
pub struct Release {
    pub version: Version,
    /// The release notes, in Markdown.
    pub notes: String,
    /// The release page, to download it by hand.
    pub page: String,
    /// The package for this system, if the release has one with a checksum.
    package: Option<Package>,
}

#[derive(Clone, Debug)]
struct Package {
    url: String,
    size: u64,
    sha256: String,
}

/// The end of the package's file name for this system, as the release workflow
/// names them.
fn package_suffix() -> Option<&'static str> {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("-macos-arm64.dmg")
    } else if cfg!(all(windows, target_arch = "x86_64")) {
        Some("-windows-x86_64.zip")
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("-linux-x86_64.AppImage")
    } else {
        None
    }
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    #[serde(default)]
    body: Option<String>,
    html_url: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
    /// `sha256:<hex>`; GitHub computes it for every uploaded asset.
    #[serde(default)]
    digest: Option<String>,
}

/// Asks GitHub for the latest published release (drafts and pre-releases are not
/// listed). `None` if it is not newer than the running version.
fn check() -> Result<Option<Release>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .build()
        .into();
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let mut response = agent
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .header(
            "User-Agent",
            concat!("Watershed/", env!("CARGO_PKG_VERSION")),
        )
        .call()
        .map_err(|e| match e {
            ureq::Error::StatusCode(404) => "No release is published yet.".to_owned(),
            e => format!("Could not reach GitHub: {e}"),
        })?;
    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("Could not reach GitHub: {e}"))?;
    let release: GitHubRelease =
        serde_json::from_str(&text).map_err(|e| format!("Unexpected answer from GitHub: {e}"))?;
    Ok(newer_release(release, current()))
}

fn newer_release(release: GitHubRelease, current: Version) -> Option<Release> {
    let version = Version::parse(&release.tag_name)?;
    if version <= current {
        return None;
    }
    let package = package_suffix().and_then(|suffix| {
        let asset = release.assets.iter().find(|a| a.name.ends_with(suffix))?;
        let sha256 = asset.digest.as_deref()?.strip_prefix("sha256:")?;
        Some(Package {
            url: asset.browser_download_url.clone(),
            size: asset.size,
            sha256: sha256.to_ascii_lowercase(),
        })
    });
    Some(Release {
        version,
        notes: release.body.unwrap_or_default().trim().to_owned(),
        page: release.html_url,
        package,
    })
}

/// How the running app was installed, which is what an update replaces.
#[derive(Clone, Debug)]
pub enum Installation {
    /// The `.app` bundle.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    MacApp(PathBuf),
    /// `watershed.exe`.
    #[cfg_attr(not(windows), allow(dead_code))]
    WindowsExe(PathBuf),
    /// The `.AppImage` file.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    AppImage(PathBuf),
}

/// Finds the installed app, or says why this copy cannot update itself.
pub fn installation() -> Result<Installation, &'static str> {
    let exe = std::env::current_exe()
        .and_then(|exe| exe.canonicalize())
        .map_err(|_| "Cannot tell where the app is installed.")?;
    if in_source_tree(&exe) {
        return Err("This is a development build: update it from the source code.");
    }
    #[cfg(target_os = "macos")]
    {
        // <bundle>.app/Contents/MacOS/watershed
        let bundle = exe
            .ancestors()
            .nth(3)
            .filter(|b| b.extension().is_some_and(|e| e == "app"));
        match bundle {
            Some(bundle) => Ok(Installation::MacApp(bundle.to_owned())),
            None => Err("Only the packaged app (Watershed.app) can update itself."),
        }
    }
    #[cfg(windows)]
    {
        Ok(Installation::WindowsExe(exe))
    }
    #[cfg(target_os = "linux")]
    {
        match std::env::var_os("APPIMAGE") {
            Some(path) => Ok(Installation::AppImage(path.into())),
            None => Err("Only the AppImage can update itself."),
        }
    }
    #[cfg(not(any(target_os = "macos", windows, target_os = "linux")))]
    {
        Err("Updates are not supported on this system.")
    }
}

/// Whether `exe` is a build inside a Cargo project (`<project>/target/...`).
fn in_source_tree(exe: &Path) -> bool {
    exe.ancestors().any(|dir| {
        dir.file_name().is_some_and(|n| n == "target")
            && dir.parent().is_some_and(|p| p.join("Cargo.toml").is_file())
    })
}

/// Deletes what an earlier update left behind. Called at start.
pub fn clean_up() {
    let _ = std::fs::remove_dir_all(work_dir());
    match installation() {
        Ok(Installation::MacApp(bundle)) => {
            for name in [OLD_BUNDLE, NEW_BUNDLE] {
                if let Some(path) = bundle.parent().map(|p| p.join(name)) {
                    let _ = std::fs::remove_dir_all(path);
                }
            }
        }
        Ok(Installation::WindowsExe(exe)) => {
            let _ = std::fs::remove_file(old_exe(&exe));
        }
        Ok(Installation::AppImage(path)) => {
            let _ = std::fs::remove_file(new_appimage(&path));
        }
        Err(_) => {}
    }
}

/// Where packages are downloaded and unpacked.
fn work_dir() -> PathBuf {
    std::env::temp_dir().join("watershed-update")
}

/// Downloads `package` and puts it in place of `installation`.
fn install(
    package: &Package,
    installation: &Installation,
    progress: &dyn Fn(u64, u64),
    cancel: &dyn Fn() -> bool,
) -> Result<(), String> {
    let download = |dest: &Path| {
        if let Some(dir) = dest.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        watershed_separation::download(
            std::slice::from_ref(&package.url),
            dest,
            watershed_separation::Expected {
                size: package.size,
                sha256: &package.sha256,
            },
            progress,
            cancel,
        )
        .map_err(|e| e.to_string())
    };
    match installation {
        Installation::MacApp(bundle) => {
            let dmg = work_dir().join("Watershed.dmg");
            download(&dmg)?;
            install_mac(&dmg, bundle)
        }
        Installation::WindowsExe(exe) => {
            let zip = work_dir().join("Watershed.zip");
            download(&zip)?;
            install_windows(&zip, exe)
        }
        Installation::AppImage(path) => {
            // Downloaded next to the old one, so the final rename stays on one disk.
            let new = new_appimage(path);
            download(&new)?;
            install_appimage(&new, path)
        }
    }
}

const OLD_BUNDLE: &str = ".Watershed-old.app";
const NEW_BUNDLE: &str = ".Watershed-new.app";

/// Copies the `.app` out of `dmg` next to `bundle`, then swaps the two. The running
/// app keeps working from files already open; it is replaced when it restarts.
fn install_mac(dmg: &Path, bundle: &Path) -> Result<(), String> {
    let dir = bundle.parent().ok_or("The app has no parent folder.")?;
    let mount = work_dir().join("mount");
    std::fs::create_dir_all(&mount).map_err(|e| format!("{}: {e}", mount.display()))?;
    run(Command::new("hdiutil")
        .args([
            "attach",
            "-nobrowse",
            "-readonly",
            "-noautoopen",
            "-mountpoint",
        ])
        .arg(&mount)
        .arg(dmg))?;
    let new = dir.join(NEW_BUNDLE);
    let _ = std::fs::remove_dir_all(&new);
    let copied = run(Command::new("ditto")
        .arg(mount.join("Watershed.app"))
        .arg(&new));
    let _ = Command::new("hdiutil")
        .args(["detach", "-quiet"])
        .arg(&mount)
        .status();
    copied.map_err(|e| not_writable(dir, e))?;
    // Our own download is not quarantined, but a copy from a quarantined disk image
    // could be: Gatekeeper would then block the new version.
    let _ = Command::new("xattr")
        .args(["-dr", "com.apple.quarantine"])
        .arg(&new)
        .status();
    swap(bundle, &new, &dir.join(OLD_BUNDLE))?;
    let _ = std::fs::remove_dir_all(dir.join(OLD_BUNDLE));
    Ok(())
}

/// Unpacks `zip` and replaces `exe` and the files next to it.
fn install_windows(zip: &Path, exe: &Path) -> Result<(), String> {
    let dir = exe.parent().ok_or("The app has no parent folder.")?;
    let unpacked = work_dir().join("unpacked");
    let _ = std::fs::remove_dir_all(&unpacked);
    std::fs::create_dir_all(&unpacked).map_err(|e| format!("{}: {e}", unpacked.display()))?;
    // Windows 10 and 11 ship bsdtar, which reads zip files. The full path avoids
    // another `tar` on the PATH (Git's cannot read them).
    let tar = std::env::var_os("SystemRoot")
        .map(|root| PathBuf::from(root).join("System32").join("tar.exe"))
        .unwrap_or_else(|| "tar".into());
    run(Command::new(tar)
        .arg("-xf")
        .arg(zip)
        .arg("-C")
        .arg(&unpacked))?;
    let package = unpacked.join("Watershed");
    let new_exe = package.join("watershed.exe");
    if !new_exe.is_file() {
        return Err("The downloaded package has no watershed.exe.".into());
    }
    let old = old_exe(exe);
    let _ = std::fs::remove_file(&old);
    std::fs::rename(exe, &old).map_err(|e| not_writable(dir, e.to_string()))?;
    if let Err(e) = std::fs::copy(&new_exe, exe) {
        let _ = std::fs::rename(&old, exe);
        return Err(not_writable(dir, e.to_string()));
    }
    // The README and the licenses: not needed to run, so failures are ignored.
    if let Ok(entries) = std::fs::read_dir(&package) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name != "watershed.exe" {
                let _ = std::fs::copy(entry.path(), dir.join(name));
            }
        }
    }
    Ok(())
}

/// Makes the downloaded AppImage `new` executable and moves it over `path`. The
/// running one keeps its open file until it exits.
fn install_appimage(new: &Path, path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(new, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("{}: {e}", new.display()))?;
    }
    std::fs::rename(new, path).map_err(|e| {
        let dir = path.parent().unwrap_or(path);
        not_writable(dir, e.to_string())
    })
}

/// Moves `current` to `old` and `new` to `current`, putting `current` back if the
/// second step fails.
fn swap(current: &Path, new: &Path, old: &Path) -> Result<(), String> {
    let _ = std::fs::remove_dir_all(old);
    let dir = current.parent().unwrap_or(current);
    std::fs::rename(current, old).map_err(|e| not_writable(dir, e.to_string()))?;
    if let Err(e) = std::fs::rename(new, current) {
        let _ = std::fs::rename(old, current);
        return Err(not_writable(dir, e.to_string()));
    }
    Ok(())
}

fn old_exe(exe: &Path) -> PathBuf {
    exe.with_extension("exe.old")
}

fn new_appimage(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_owned();
    name.push(".new");
    path.with_file_name(name)
}

fn not_writable(dir: &Path, error: String) -> String {
    format!(
        "Cannot write to {}: {error}. Download the new version from the release page.",
        dir.display()
    )
}

/// Runs a system tool, turning a failure into a message.
fn run(command: &mut Command) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // No console window flashing up.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let name = command.get_program().to_string_lossy().into_owned();
    let output = command
        .output()
        .map_err(|e| format!("Cannot run {name}: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{name} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// Starts the installed app again. Called on exit, after the session is saved.
pub fn relaunch(installation: &Installation) {
    let result = match installation {
        // -n: a new instance, since this one has not quit yet.
        Installation::MacApp(bundle) => Command::new("open").arg("-n").arg(bundle).spawn(),
        Installation::WindowsExe(path) | Installation::AppImage(path) => Command::new(path).spawn(),
    };
    if let Err(e) = result {
        log::error!("cannot start the updated app: {e}");
    }
}

/// A background task's answer.
struct Task<T>(Receiver<T>);

impl<T: Send + 'static> Task<T> {
    fn spawn(ctx: &egui::Context, work: impl FnOnce() -> T + Send + 'static) -> Self {
        let (send, receive) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = send.send(work());
            ctx.request_repaint();
        });
        Self(receive)
    }

    /// `Some` once, when the task is done (or if it panicked).
    fn poll(&self) -> Option<Option<T>> {
        match self.0.try_recv() {
            Ok(value) => Some(Some(value)),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(None),
        }
    }
}

/// Where the updater is.
enum State {
    Idle,
    Checking {
        /// Started by the app, not the user: "up to date" and errors are not shown.
        quiet: bool,
        task: Task<Result<Option<Release>, String>>,
    },
    UpToDate,
    Available(Release),
    Installing {
        release: Release,
        /// Bytes downloaded and the total.
        progress: Arc<Mutex<(u64, u64)>>,
        cancel: Arc<AtomicBool>,
        task: Task<Result<(), String>>,
    },
    /// Installed; it runs after a restart.
    Installed(Version),
    Failed(String),
}

/// What the user asked for in [`Updater::ui`].
#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Restart,
}

/// The updater: its state and its part of the settings window.
pub struct Updater {
    state: State,
    installation: Result<Installation, &'static str>,
}

impl Updater {
    pub fn new() -> Self {
        Self {
            state: State::Idle,
            installation: installation(),
        }
    }

    /// Looks for a newer release. `quiet` hides "up to date" and failures.
    pub fn check(&mut self, quiet: bool, ctx: &egui::Context) {
        if matches!(
            self.state,
            State::Checking { .. } | State::Installing { .. }
        ) {
            return;
        }
        self.state = State::Checking {
            quiet,
            task: Task::spawn(ctx, check),
        };
    }

    /// A newer release found, until it is installed.
    pub fn available(&self) -> Option<Version> {
        match &self.state {
            State::Available(release) | State::Installing { release, .. } => Some(release.version),
            _ => None,
        }
    }

    pub fn installation(&self) -> Option<&Installation> {
        self.installation.as_ref().ok()
    }

    /// Collects the answers of background tasks.
    pub fn poll(&mut self) {
        let next = match &self.state {
            State::Checking { quiet, task } => task.poll().map(|answer| {
                match answer.unwrap_or_else(|| Err("The update check failed.".into())) {
                    Ok(Some(release)) => State::Available(release),
                    Ok(None) if *quiet => State::Idle,
                    Ok(None) => State::UpToDate,
                    Err(_) if *quiet => State::Idle,
                    Err(e) => State::Failed(e),
                }
            }),
            State::Installing {
                release,
                cancel,
                task,
                ..
            } => task.poll().map(|answer| {
                match answer.unwrap_or_else(|| Err("The update failed.".into())) {
                    Ok(()) => State::Installed(release.version),
                    Err(_) if cancel.load(Ordering::Relaxed) => State::Available(release.clone()),
                    Err(e) => State::Failed(e),
                }
            }),
            _ => None,
        };
        if let Some(next) = next {
            self.state = next;
        }
    }

    fn install(&mut self, ctx: &egui::Context) {
        let (State::Available(release), Ok(installation)) = (&self.state, &self.installation)
        else {
            return;
        };
        let Some(package) = release.package.clone() else {
            return;
        };
        let installation = installation.clone();
        let progress = Arc::new(Mutex::new((0, package.size)));
        let cancel = Arc::new(AtomicBool::new(false));
        let task = {
            let progress = progress.clone();
            let cancel = cancel.clone();
            let ctx = ctx.clone();
            Task::spawn(&ctx.clone(), move || {
                install(
                    &package,
                    &installation,
                    &|done, total| {
                        *progress.lock().unwrap() = (done, total);
                        ctx.request_repaint_after(Duration::from_millis(100));
                    },
                    &|| cancel.load(Ordering::Relaxed),
                )
            })
        };
        self.state = State::Installing {
            release: release.clone(),
            progress,
            cancel,
            task,
        };
    }

    /// The updates section of the settings window. `can_restart` is false while
    /// work that a restart would lose is running.
    pub fn ui(&mut self, ui: &mut egui::Ui, can_restart: bool) -> Option<Action> {
        let mut action = None;
        let mut check = false;
        let mut install = false;
        ui.horizontal(|ui| {
            ui.label(format!("Version {}", current()));
            let busy = matches!(
                self.state,
                State::Checking { .. } | State::Installing { .. }
            );
            check = ui
                .add_enabled(!busy, egui::Button::new("Check for updates").small())
                .clicked();
        });
        match &self.state {
            State::Idle => {}
            State::Checking { .. } => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Checking…");
                });
            }
            State::UpToDate => {
                ui.label("Watershed is up to date.");
            }
            State::Available(release) => {
                ui.label(
                    egui::RichText::new(format!("Version {} is available.", release.version))
                        .strong(),
                );
                match (&release.package, &self.installation) {
                    (Some(_), Ok(_)) => {
                        install = ui.button("Download and install").clicked();
                    }
                    (None, _) => {
                        ui.hyperlink_to("Download it from the release page", &release.page);
                    }
                    (_, Err(why)) => {
                        ui.label(egui::RichText::new(*why).weak().small());
                        ui.hyperlink_to("Download it from the release page", &release.page);
                    }
                }
                notes(ui, release);
            }
            State::Installing {
                release,
                progress,
                cancel,
                ..
            } => {
                let (done, total) = *progress.lock().unwrap();
                let fraction = if total > 0 {
                    done as f32 / total as f32
                } else {
                    0.0
                };
                ui.horizontal(|ui| {
                    let text = if done >= total && total > 0 {
                        "Installing…".to_owned()
                    } else {
                        format!(
                            "Downloading {}: {:.0} / {:.0} MB",
                            release.version,
                            done as f64 / 1e6,
                            total as f64 / 1e6
                        )
                    };
                    ui.add(
                        egui::ProgressBar::new(fraction)
                            .desired_width(260.0)
                            .text(text),
                    );
                    if cancel.load(Ordering::Relaxed) {
                        ui.label(egui::RichText::new("Cancelling…").weak());
                    } else if ui.small_button("Cancel").clicked() {
                        cancel.store(true, Ordering::Relaxed);
                    }
                });
            }
            State::Installed(version) => {
                ui.label(format!(
                    "Version {version} is installed. Restart to use it."
                ));
                let restart = ui
                    .add_enabled(can_restart, egui::Button::new("Restart now"))
                    .on_disabled_hover_text("Wait for the separation to finish, or cancel it.");
                if restart.clicked() {
                    action = Some(Action::Restart);
                }
            }
            State::Failed(error) => {
                ui.label(egui::RichText::new(error).color(ui.visuals().error_fg_color));
            }
        }
        if check {
            self.check(false, ui.ctx());
        }
        if install {
            self.install(ui.ctx());
        }
        action
    }
}

/// The release notes, folded.
fn notes(ui: &mut egui::Ui, release: &Release) {
    if release.notes.is_empty() {
        return;
    }
    egui::CollapsingHeader::new("What's new")
        .id_salt("update notes")
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new(&release.notes).small())
                });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert_eq!(Version::parse("v0.1.1"), Some(Version(0, 1, 1)));
        assert_eq!(Version::parse("1.20.3"), Some(Version(1, 20, 3)));
        assert_eq!(Version::parse("v0.2.0-beta"), None);
        assert_eq!(Version::parse("0.2"), None);
        assert_eq!(Version::parse("0.2.0.1"), None);
        assert!(Version(0, 10, 0) > Version(0, 9, 9));
        assert!(Version(1, 0, 0) > Version(0, 99, 99));
        assert_eq!(Version(0, 1, 1).to_string(), "0.1.1");
    }

    fn github_release(tag: &str) -> GitHubRelease {
        let asset = |suffix: &str| GitHubAsset {
            name: format!("Watershed-0.2.0{suffix}"),
            browser_download_url: format!("https://example.com/Watershed-0.2.0{suffix}"),
            size: 42,
            digest: Some("sha256:ABC123".into()),
        };
        GitHubRelease {
            tag_name: tag.into(),
            body: Some("  Notes\n".into()),
            html_url: "https://example.com/release".into(),
            assets: vec![
                asset("-linux-x86_64.AppImage"),
                asset("-macos-arm64.dmg"),
                asset("-windows-x86_64.zip"),
            ],
        }
    }

    #[test]
    fn only_newer_releases() {
        let current = Version(0, 1, 1);
        assert!(newer_release(github_release("v0.1.1"), current).is_none());
        assert!(newer_release(github_release("v0.1.0"), current).is_none());
        assert!(newer_release(github_release("nightly"), current).is_none());
        let release = newer_release(github_release("v0.2.0"), current).unwrap();
        assert_eq!(release.version, Version(0, 2, 0));
        assert_eq!(release.notes, "Notes");
        if let Some(suffix) = package_suffix() {
            let package = release.package.unwrap();
            assert!(package.url.ends_with(suffix));
            assert_eq!(package.sha256, "abc123");
            assert_eq!(package.size, 42);
        }
    }

    #[test]
    fn no_package_without_a_checksum() {
        let mut release = github_release("v0.2.0");
        for asset in &mut release.assets {
            asset.digest = None;
        }
        let release = newer_release(release, Version(0, 1, 1)).unwrap();
        assert!(release.package.is_none());
    }

    #[test]
    fn source_tree_builds_are_found() {
        let project = std::env::current_dir().unwrap();
        let project = project
            .ancestors()
            .find(|d| d.join("Cargo.lock").is_file())
            .unwrap();
        assert!(in_source_tree(&project.join("target/release/watershed")));
        assert!(!in_source_tree(Path::new(
            "/Applications/Watershed.app/Contents/MacOS/watershed"
        )));
    }
}
