#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use anyhow::{Context, Result};
#[cfg(not(windows))]
use global_hotkey::hotkey::HotKey;
#[cfg(not(windows))]
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
#[cfg(not(windows))]
use marktune::config::parse_hotkey;
use marktune::config::Config;
use marktune::injector::{EnigoInjector, TextInjector};
use resvg::tiny_skia;
#[cfg(not(windows))]
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};

#[cfg(windows)]
mod win_hotkeys;
#[cfg(windows)]
use win_hotkeys::Hotkeys;

const ID_TOGGLE: &str = "toggle";
const ID_OPEN_CONFIG: &str = "open-config";
const ID_QUIT: &str = "quit";
const ID_ABOUT: &str = "about";


// Embed SVG file contents into the binary at compile time
const SVG_ENABLED: &[u8] = include_bytes!("../assets/icon-enabled.svg");
const SVG_DISABLED: &[u8] = include_bytes!("../assets/icon-disabled.svg");

// ---------------------------------------------------------------------------
// Logging (std-only, initialized before any config loading)
// ---------------------------------------------------------------------------

/// Primary log path on macOS: ~/Library/Logs/marktune/marktune.log
fn log_file_path() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            let mut p = PathBuf::from(home);
            p.push("Library");
            p.push("Logs");
            p.push("marktune");
            return Some(p.join("marktune.log"));
        }
    }
    None
}

/// Fallback log path for non-macOS platforms.
#[cfg(not(target_os = "macos"))]
fn log_file_path_fallback() -> Option<PathBuf> {
    // Windows: %LOCALAPPDATA%\marktune\marktune.log
    if let Ok(dir) = std::env::var("LOCALAPPDATA") {
        if !dir.is_empty() {
            return Some(PathBuf::from(dir).join("marktune").join("marktune.log"));
        }
    }
    // Linux/BSD: $XDG_STATE_HOME/marktune/marktune.log (default ~/.local/state)
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            let base = std::env::var("XDG_STATE_HOME")
                .ok()
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(&home)
                        .join(".local")
                        .join("state")
                });
            return Some(base.join("marktune").join("marktune.log"));
        }
    }
    None
}

fn logger() -> &'static Mutex<Option<std::fs::File>> {
    static LOGGER: OnceLock<Mutex<Option<std::fs::File>>> = OnceLock::new();
    LOGGER.get_or_init(|| {
        let path = {
            #[cfg(target_os = "macos")]
            {
                log_file_path()
            }
            #[cfg(not(target_os = "macos"))]
            {
                log_file_path().or_else(log_file_path_fallback)
            }
        };
        let file = path.and_then(|p| open_log_file(&p));
        Mutex::new(file)
    })
}

/// Open (create) the log file, creating parent directories as needed. Appends.
fn open_log_file(path: &Path) -> Option<std::fs::File> {
    if let Some(dir) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(dir) {
            eprintln!("marktune: cannot create log directory {}: {e}", dir.display());
            return None;
        }
    }
    match std::fs::OpenOptions::new().create(true).append(true).open(path) {
        Ok(f) => Some(f),
        Err(e) => {
            eprintln!("marktune: cannot open log file {}: {e}", path.display());
            None
        }
    }
}

/// Explicitly initialize the logger as early as possible.
/// Must be called before any config loading. Never panics.
fn init_logger() {
    let _ = logger();
}

/// Append a timestamped line to the persistent log file. Never panics.
pub fn log_line(msg: &str) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let line = format!("[{now}] {msg}\n");
    if let Ok(mut guard) = logger().lock() {
        if let Some(f) = guard.as_mut() {
            let _ = f.write_all(line.as_bytes());
            let _ = f.flush();
        }
    }
    // In debug builds also print to stderr for `cargo run` visibility.
    #[cfg(debug_assertions)]
    eprint!("{line}");
}

/// Show a native macOS alert via /usr/bin/osascript (no extra dependencies).
/// On other platforms this is a no-op. Never panics; failures are ignored
/// (but logged in debug builds).
fn show_alert(title: &str, message: &str) {
    #[cfg(target_os = "macos")]
    {
        let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
        let script = format!(
            "display dialog \"{}\" with title \"{}\" buttons {{\"OK\"}} default button \"OK\" with icon caution",
            esc(message),
            esc(title)
        );
        if let Err(e) = std::process::Command::new("/usr/bin/osascript")
            .arg("-e")
            .arg(&script)
            .output()
        {
            log_line(&format!("Could not show alert via osascript: {e}"));
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (title, message);
    }
}

// ---------------------------------------------------------------------------
// Icons
// ---------------------------------------------------------------------------

// Render SVG to an RGBA pixel array with specified dimensions
fn render_svg_to_icon(svg_data: &[u8], size: u32) -> Result<Icon> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(svg_data, &opt).context("Failed to parse SVG file")?;

    let mut pixmap =
        tiny_skia::Pixmap::new(size, size).context("Failed to allocate memory for icon image")?;

    let sx = size as f32 / tree.size().width();
    let sy = size as f32 / tree.size().height();
    let transform = tiny_skia::Transform::from_scale(sx, sy);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Convert RGBA pixels to Icon struct
    Icon::from_rgba(pixmap.take(), size, size).context("Failed to create icon from rendered image")
}

// Select and build the appropriate icon based on state
fn tray_icon(enabled: bool) -> Icon {
    let svg_bytes = if enabled { SVG_ENABLED } else { SVG_DISABLED };
    render_svg_to_icon(svg_bytes, 32).unwrap_or_else(|e| {
        log_line(&format!("Error rendering SVG icon: {e:#}; using fallback icon"));
        fallback_icon(enabled)
    })
}

// Fallback icon in case of errors in the SVG file
fn fallback_icon(enabled: bool) -> Icon {
    const S: u32 = 32;
    let mut rgba = vec![0u8; (S * S * 4) as usize];
    let (r, g, b) = if enabled { (46, 204, 64) } else { (192, 86, 76) };
    for chunk in rgba.chunks_exact_mut(4) {
        chunk[0] = r;
        chunk[1] = g;
        chunk[2] = b;
        chunk[3] = 255;
    }
    Icon::from_rgba(rgba, S, S).expect("Fallback icon is always valid")
}

// ---------------------------------------------------------------------------
// Hotkey management (macOS / non-Windows): global-hotkey + RegisterHotKey.
// روی ویندوز از ماژول win_hotkeys (WH_KEYBOARD_LL hook) استفاده می‌شود،
// چون RegisterHotKey جلوی Raw Input را نمی‌گیرد (رجوع کن به win_hotkeys.rs).
// ---------------------------------------------------------------------------

#[cfg(not(windows))]
struct Hotkeys {
    manager: GlobalHotKeyManager,
    registered: Vec<HotKey>,
    map: HashMap<u32, String>,
}

#[cfg(not(windows))]
impl Hotkeys {
    fn new(cfg: &Config) -> Result<Self> {
        let manager =
            GlobalHotKeyManager::new().context("Failed to initialize GlobalHotKeyManager")?;
        let mut hk = Self {
            manager,
            registered: Vec::new(),
            map: HashMap::new(),
        };
        hk.register(cfg)?;
        Ok(hk)
    }

    fn register(&mut self, cfg: &Config) -> Result<()> {
        let mut new_registered = Vec::new();
        let mut new_map = HashMap::new();

        for b in &cfg.bindings {
            if b.is_reset() {
                continue;
            }
            let (mods, code) = parse_hotkey(&b.hotkey)
                .with_context(|| format!("Invalid shortcut: {}", b.hotkey))?;
            let hotkey = HotKey::new(Some(mods), code);

            if let Err(e) = self.manager.register(hotkey) {
                let _ = self.manager.unregister_all(&new_registered);
                return Err(e).with_context(|| format!("Failed to register: {}", b.hotkey));
            }

            let text = b.text(cfg);
            if !text.is_empty() {
                new_map.insert(hotkey.id(), text);
            }
            new_registered.push(hotkey);
        }

        self.registered = new_registered;
        self.map = new_map;
        Ok(())
    }

    fn unregister_all(&mut self) -> Result<()> {
        if self.registered.is_empty() {
            return Ok(());
        }
        self.manager
            .unregister_all(&self.registered)
            .context("Failed to unregister shortcuts")?;
        self.registered.clear();
        self.map.clear();
        Ok(())
    }

    fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    fn drain_events(&self) -> Vec<String> {
        let mut out = Vec::new();
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.state != HotKeyState::Pressed {
                continue;
            }
            if let Some(text) = self.map.get(&event.id) {
                out.push(text.clone());
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

struct App {
    hotkeys: Hotkeys,
    cfg: Config,
    cfg_path: PathBuf,
    cgj: char,
    injector: EnigoInjector,
    tray: Option<TrayIcon>,
    toggle_item: CheckMenuItem,
    enabled: bool,
}

impl App {
    fn weave_cgj(&self, s: &str) -> String {
        let mut out = String::with_capacity(s.len() * 2);
        for ch in s.chars() {
            out.push(self.cgj);
            out.push(ch);
        }
        out
    }

    fn handle(&mut self, text: &str) {
        let payload = self.weave_cgj(text);
        if let Err(e) = self.injector.insert(&payload) {
            log_line(&format!("Error injecting text: {e}"));
        }
    }

    fn set_enabled(&mut self, enabled: bool) {
        if self.enabled == enabled {
            return;
        }
        if enabled {
            if let Err(e) = self.hotkeys.register(&self.cfg) {
                log_line(&format!("Activation failed: {e:#}"));
                return;
            }
            log_line("Shortcuts enabled.");
        } else {
            if let Err(e) = self.hotkeys.unregister_all() {
                log_line(&format!("Error unregistering shortcuts: {e:#}"));
                return;
            }
            log_line("Shortcuts disabled.");
        }
        self.enabled = enabled;
        self.toggle_item.set_checked(enabled);
        if let Some(t) = &mut self.tray {
            let icon = tray_icon(enabled);
            if let Err(e) = t.set_icon(Some(icon)) {
                log_line(&format!("Failed to update icon: {e}"));
            }
            let tip = if enabled {
                "Marktune – Shortcuts enabled"
            } else {
                "Marktune – Shortcuts disabled"
            };
            let _ = t.set_tooltip(Some(tip));
        }
    }

fn reveal_config_file(&self) {
    let path = &self.cfg_path;

    #[cfg(target_os = "macos")]
    {
        let res = std::process::Command::new("open").arg("-R").arg(path).spawn();
        if let Err(e) = res {
            log_line(&format!("Failed to reveal file in Finder: {e}"));
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let arg = format!("/select,\"{}\"", path.display());
        let res = std::process::Command::new("explorer").raw_arg(&arg).spawn();
        if let Err(e) = res {
            log_line(&format!("Failed to reveal file in Explorer: {e}"));
        }
    }

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
        let res = std::process::Command::new("xdg-open").arg(dir).spawn();
        if let Err(e) = res {
            log_line(&format!("Failed to open config directory: {e}"));
        }
    }
}

}

impl ApplicationHandler<MenuEvent> for App {
    fn resumed(&mut self, _el: &ActiveEventLoop) {
        if self.tray.is_some() {
            return;
        }

        let icon = tray_icon(self.enabled);
        let menu = Menu::new();
        let _ = menu.append(&self.toggle_item);
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&MenuItem::with_id(
            ID_OPEN_CONFIG,
            "Edit Shortcut Config…",
            true,
            None,
        ));
        let _ = menu.append(&MenuItem::with_id(
            ID_ABOUT,
            "About Marktune…",
            true,
            None,
        ));
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&MenuItem::with_id(ID_QUIT, "Quit", true, None));

        match TrayIconBuilder::new()
            .with_id("marktune-tray")
            .with_icon(icon)
            .with_tooltip("Marktune – Shortcuts enabled")
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(true)
            .build()
        {
            Ok(tray) => {
                self.tray = Some(tray);
                log_line("Tray icon created.");
            }
            Err(e) => {
                log_line(&format!("Failed to build Tray icon: {e}"));
                log_line("The program will continue without a Tray icon.");
            }
        }
    }

    fn window_event(
        &mut self,
        _el: &ActiveEventLoop,
        _id: winit::window::WindowId,
        _ev: WindowEvent,
    ) {
    }

    fn user_event(&mut self, el: &ActiveEventLoop, ev: MenuEvent) {
        match ev.id().as_ref() {
            ID_TOGGLE => self.set_enabled(!self.enabled),
            ID_OPEN_CONFIG => self.reveal_config_file(),
            ID_ABOUT => show_about_dialog(),
            ID_QUIT => el.exit(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _el: &ActiveEventLoop) {
        while TrayIconEvent::receiver().try_recv().is_ok() {}

        if self.enabled {
            for text in self.hotkeys.drain_events() {
                self.handle(&text);
            }
        } else {
            // حتی وقتی غیرفعاله، صف رو خالی کن تا انباشته نشه.
            let _ = self.hotkeys.drain_events();
        }

        // NOTE: The winit event loop with ControlFlow::WaitUntil is what keeps
        // the tray (and the whole process) alive when launched from Finder.
        // Do not replace this with ControlFlow::Wait/Exit.
        _el.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(50),
        ));
    }
}

// ---------------------------------------------------------------------------
// Config path resolution (independent of the current working directory)
// ---------------------------------------------------------------------------

/// macOS .app layout:
///   Foo.app/Contents/MacOS/marktune -> Foo.app/Contents/Resources/config.json
fn bundle_resource_config(exe: &Path) -> Option<PathBuf> {
    let macos_dir = exe.parent()?;
    let contents = macos_dir.parent()?;
    let candidate = contents.join("Resources").join("config.json");
    candidate.exists().then_some(candidate)
}

/// Resolve config.json in a fixed priority order:
///   1) explicit first CLI argument (if present)
///   2) macOS bundle Contents/Resources/config.json (derived from current_exe)
///   3) config.json beside the executable
///   4) CARGO_MANIFEST_DIR/config.json — development builds only
///   5) cwd/config.json — last resort
fn config_path() -> PathBuf {
    // 1) Explicit CLI path always wins.
    if let Some(arg) = std::env::args().nth(1) {
        log_line(&format!("Config from CLI argument: {arg}"));
        return PathBuf::from(arg);
    }

    if let Ok(exe) = std::env::current_exe() {
        // 2) macOS app bundle: Contents/Resources/config.json
        if let Some(p) = bundle_resource_config(&exe) {
            log_line(&format!("Config from app bundle: {}", p.display()));
            return p;
        }
        // 3) config.json beside the executable.
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("config.json");
            if candidate.exists() {
                log_line(&format!("Config beside executable: {}", candidate.display()));
                return candidate;
            }
        }
    }

    // 4) Development fallback: config.json in the project root relative to
    //    CARGO_MANIFEST_DIR (compile-time), so `cargo run` keeps working.
    //    This is NOT cwd-dependent, so Finder launches stay reliable.
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config.json");
    if dev.exists() {
        log_line(&format!("Config from development tree: {}", dev.display()));
        return dev;
    }

    // 5) Last resort: current working directory.
    let cwd = PathBuf::from("config.json");
    log_line(&format!(
        "Falling back to cwd config: {} (resolved: {})",
        cwd.display(),
        std::env::current_dir()
            .map(|d| d.join(&cwd).display().to_string())
            .unwrap_or_else(|_| cwd.display().to_string())
    ));
    cwd
}

fn get_about_text() -> String {
    let candidate_paths = [
        std::env::current_exe().ok().and_then(|exe| {
            exe.parent()?
                .parent()?
                .join("Resources")
                .join("About.txt")
                .into()
        }),
        std::env::current_exe().ok().and_then(|exe| {
            exe.parent().map(|p| p.join("About.txt"))
        }),
        Some(std::path::PathBuf::from("About.txt")),
    ];

    for path in candidate_paths.into_iter().flatten() {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                return content;
            }
        }
    }

    "Marktune\nVersion 0.2.0\n\nNo About.txt file found.".to_string()
}

fn show_about_dialog() {
    let about_text = get_about_text();
    std::thread::spawn(move || {
        rfd::MessageDialog::new()
            .set_title("About Marktune")
            .set_description(&about_text)
            .set_buttons(rfd::MessageButtons::Ok)
            .set_level(rfd::MessageLevel::Info)
            .show();
    });
}


// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    // Initialize the diagnostic log before anything else (especially config).
    init_logger();
    log_line(&format!(
        "marktune starting (pid: {}, args: {:?})",
        std::process::id(),
        std::env::args().collect::<Vec<_>>()
    ));
    if let Ok(exe) = std::env::current_exe() {
        log_line(&format!("current_exe: {}", exe.display()));
    }
    if let Ok(cwd) = std::env::current_dir() {
        log_line(&format!("cwd: {}", cwd.display()));
    }

    let exit_code = run();
    match &exit_code {
        Err(e) => {
            // Log the full error chain ({:#} renders "ctx: cause: root cause").
            log_line(&format!("FATAL: {e:#}"));
            show_alert(
                "Marktune",
                &format!("Startup failed:\n{e:#}\n\nLog: ~/Library/Logs/marktune/marktune.log"),
            );
        }
        Ok(()) => log_line("Exiting normally."),
    }
    std::process::exit(exit_code.is_err() as i32);
}

fn run() -> Result<()> {
    let cfg_path = config_path();
    log_line(&format!("Using config: {}", cfg_path.display()));

    let cfg = Config::load(&cfg_path)
        .with_context(|| format!("Error loading {}", cfg_path.display()))?;
    log_line(&format!(
        "Config loaded ({} binding(s), cgj: U+{:04X}).",
        cfg.bindings.len(),
        cfg.cgj as u32
    ));

    let hotkeys = Hotkeys::new(&cfg).map_err(|e| {
        log_line(&format!("Hotkey manager init failed: {e:#}"));
        e
    })?;
    log_line(&format!(
        "Hotkey manager ready ({} binding(s)).",
        cfg.bindings.len()
    ));
    if hotkeys.is_empty() {
        log_line("Warning: No keys were registered. Check the keys in config.json");
    }

    let cgj = cfg.cgj;
    let injector = EnigoInjector::new().map_err(|e| {
        log_line(&format!("EnigoInjector failed: {e:#}"));
        e
    })?;
    log_line("Injector ready.");

    let event_loop = EventLoop::<MenuEvent>::with_user_event()
        .build()
        .map_err(|e| {
            log_line(&format!("The EventLoop failed: {e}"));
            anyhow::anyhow!("{e}")
        })?;

    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |e| {
        let _ = proxy.send_event(e);
    }));

    // Explicitly set ID_TOGGLE for CheckMenuItem
    let toggle_item = CheckMenuItem::with_id(ID_TOGGLE, "Shortcuts enabled", true, true, None);

    let mut app = App {
        hotkeys,
        cfg,
        cfg_path,
        cgj,
        injector,
        tray: None,
        toggle_item,
        enabled: true,
    };

    log_line("Entering event loop…");
    let result = event_loop
        .run_app(&mut app)
        .map_err(|e| anyhow::anyhow!("{e}"));
    let _ = app.hotkeys.unregister_all();
    result
}

