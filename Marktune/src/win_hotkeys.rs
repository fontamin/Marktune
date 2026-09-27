//! گرفتن هات‌کی سراسری روی ویندوز با یک WH_KEYBOARD_LL hook.
//!
//! `global-hotkey` (که از RegisterHotKey استفاده می‌کند) فقط جلوی رسیدن
//! کلید به صف پیام معمولی Win32 (WM_KEYDOWN) پنجرهٔ فوکوس‌شده را می‌گیرد؛
//! جلوی Raw Input (WM_INPUT) را نمی‌گیرد. Illustrator ظاهراً کیبورد را از
//! همان مسیر Raw Input می‌خواند، برای همین کلید فیزیکی (پیکان یا حرف)
//! مستقل از global-hotkey به سند نشت می‌کرد.
//!
//! یک low-level keyboard hook زودتر از این مسیر می‌نشیند؛ اگر از hook
//! procedure مقدار غیرصفر برگردانده شود، ویندوز رویداد را قبل از رسیدن به
//! Raw Input یا هر مصرف‌کنندهٔ دیگری حذف می‌کند — همان تکنیکی که AutoHotkey
//! هم استفاده می‌کند.

use anyhow::{anyhow, Context, Result};
use marktune::config::{parse_hotkey_win, Config};
use std::collections::HashSet;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::{Mutex, OnceLock};
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
    UnhookWindowsHookEx, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL, WM_KEYDOWN,
    WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

const VK_LCONTROL: u32 = 0xA2;
const VK_RCONTROL: u32 = 0xA3;
const VK_CONTROL: u32 = 0x11;
const VK_LMENU: u32 = 0xA4; // Alt
const VK_RMENU: u32 = 0xA5;
const VK_MENU: u32 = 0x12;
const VK_LSHIFT: u32 = 0xA0;
const VK_RSHIFT: u32 = 0xA1;
const VK_SHIFT: u32 = 0x10;
const VK_LWIN: u32 = 0x5B;
const VK_RWIN: u32 = 0x5C;

#[derive(Debug, Clone)]
struct WinBinding {
    ctrl: bool,
    alt: bool,
    shift: bool,
    meta: bool,
    vk: u32,
    /// None برای reset (فقط سرکوب می‌شود، چیزی درج نمی‌شود) یا وقتی متن خالیست.
    text: Option<String>,
}

static BINDINGS: OnceLock<Mutex<Vec<WinBinding>>> = OnceLock::new();
static EVENT_TX: OnceLock<Mutex<Sender<String>>> = OnceLock::new();
static DOWN_KEYS: OnceLock<Mutex<HashSet<u32>>> = OnceLock::new();
static SUPPRESSED: OnceLock<Mutex<HashSet<u32>>> = OnceLock::new();

fn bindings() -> &'static Mutex<Vec<WinBinding>> {
    BINDINGS.get_or_init(|| Mutex::new(Vec::new()))
}
fn down_keys() -> &'static Mutex<HashSet<u32>> {
    DOWN_KEYS.get_or_init(|| Mutex::new(HashSet::new()))
}
fn suppressed() -> &'static Mutex<HashSet<u32>> {
    SUPPRESSED.get_or_init(|| Mutex::new(HashSet::new()))
}

fn any_down(keys: &HashSet<u32>, codes: &[u32]) -> bool {
    codes.iter().any(|c| keys.contains(c))
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 {
        return CallNextHookEx(None, code, wparam, lparam);
    }

    let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);

    // رویدادهایی که خودمان (تزریق متن) تولید کرده‌ایم را دست نزن؛ وگرنه
    // حلقهٔ بازخورد و رفتار غیرقابل‌پیش‌بینی ایجاد می‌شود.
    if kb.flags.0 & LLKHF_INJECTED.0 != 0 {
        return CallNextHookEx(None, code, wparam, lparam);
    }

    let vk = kb.vkCode;
    let msg = wparam.0 as u32;
    let is_down_msg = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
    let is_up_msg = msg == WM_KEYUP || msg == WM_SYSKEYUP;

    if !is_down_msg && !is_up_msg {
        return CallNextHookEx(None, code, wparam, lparam);
    }

    if is_up_msg {
        down_keys().lock().unwrap().remove(&vk);
        if suppressed().lock().unwrap().remove(&vk) {
            return LRESULT(1);
        }
        return CallNextHookEx(None, code, wparam, lparam);
    }

    // is_down_msg
    let (was_down, snapshot) = {
        let mut dk = down_keys().lock().unwrap();
        let was_down = !dk.insert(vk);
        (was_down, dk.clone())
    };

    if was_down {
        // auto-repeat؛ اگر بار اول سرکوب شده بود همچنان سرکوب کن
        if suppressed().lock().unwrap().contains(&vk) {
            return LRESULT(1);
        }
        return CallNextHookEx(None, code, wparam, lparam);
    }

    let ctrl = any_down(&snapshot, &[VK_CONTROL, VK_LCONTROL, VK_RCONTROL]);
    let alt = any_down(&snapshot, &[VK_MENU, VK_LMENU, VK_RMENU]);
    let shift = any_down(&snapshot, &[VK_SHIFT, VK_LSHIFT, VK_RSHIFT]);
    let meta = any_down(&snapshot, &[VK_LWIN, VK_RWIN]);

    let hit = {
        let binds = bindings().lock().unwrap();
        binds
            .iter()
            .find(|b| {
                b.ctrl == ctrl && b.alt == alt && b.shift == shift && b.meta == meta && b.vk == vk
            })
            .cloned()
    };

    if let Some(b) = hit {
        suppressed().lock().unwrap().insert(vk);
        if let Some(text) = b.text {
            if let Some(tx) = EVENT_TX.get() {
                let _ = tx.lock().unwrap().send(text);
            }
        }
        return LRESULT(1);
    }

    CallNextHookEx(None, code, wparam, lparam)
}

fn build_bindings(cfg: &Config) -> Result<Vec<WinBinding>> {
    let mut out = Vec::new();
    for b in &cfg.bindings {
        if b.is_reset() {
            continue;
        }
        let hk = parse_hotkey_win(&b.hotkey)
            .with_context(|| format!("Invalid shortcut: {}", b.hotkey))?;
        let text = b.text(cfg);
        if text.is_empty() {
            continue;
        }
        out.push(WinBinding {
            ctrl: hk.ctrl,
            alt: hk.alt,
            shift: hk.shift,
            meta: hk.meta,
            vk: hk.vk,
            text: Some(text),
        });
    }
    Ok(out)
}

fn install_hook_and_pump() {
    unsafe {
        let hmodule = match GetModuleHandleW(None) {
            Ok(h) => h,
            Err(e) => {
                crate::log_line(&format!("GetModuleHandleW failed: {e}"));
                return;
            }
        };
        let hinstance: windows::Win32::Foundation::HINSTANCE = hmodule.into();
        let hook = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), Some(&hinstance), 0) {
            Ok(h) => h,
            Err(e) => {
                crate::log_line(&format!("SetWindowsHookExW failed: {e}"));
                return;
            }
        };

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        let _ = UnhookWindowsHookEx(hook);
    }
}

pub struct Hotkeys {
    rx: Receiver<String>,
}

impl Hotkeys {
    pub fn new(cfg: &Config) -> Result<Self> {
        let (tx, rx) = channel();
        EVENT_TX
            .set(Mutex::new(tx))
            .map_err(|_| anyhow!("Keyboard hook already initialized"))?;

        let built = build_bindings(cfg)?;
        *bindings().lock().unwrap() = built;

        std::thread::spawn(install_hook_and_pump);

        Ok(Self { rx })
    }

    pub fn register(&mut self, cfg: &Config) -> Result<()> {
        let built = build_bindings(cfg)?;
        *bindings().lock().unwrap() = built;
        Ok(())
    }

    pub fn unregister_all(&mut self) -> Result<()> {
        bindings().lock().unwrap().clear();
        suppressed().lock().unwrap().clear();
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        bindings().lock().unwrap().iter().all(|b| b.text.is_none())
    }

    pub fn drain_events(&self) -> Vec<String> {
        let mut out = Vec::new();
        loop {
            match self.rx.try_recv() {
                Ok(text) => out.push(text),
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }
        out
    }
}
