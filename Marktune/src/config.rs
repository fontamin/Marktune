use anyhow::{Context, Result};
#[cfg(not(windows))]
use anyhow::anyhow;
#[cfg(not(windows))]
use global_hotkey::hotkey::{Code, Modifiers};
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer};
use std::path::Path;
use std::str::FromStr;

fn parse_codepoint(s: &str) -> Option<char> {
    let s = s.trim();
    let hex = s
        .strip_prefix("U+")
        .or_else(|| s.strip_prefix("u+"))
        .unwrap_or(s);
    u32::from_str_radix(hex, 16).ok().and_then(char::from_u32)
}

fn de_char<'de, D>(d: D) -> Result<char, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    parse_codepoint(&s).ok_or_else(|| D::Error::custom(format!("invalide codepoint: {s}")))
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum GroupId {
    Top,
    Bottom,
}

impl Default for GroupId {
    fn default() -> Self {
        GroupId::Top
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Axis {
    X,
    XNeg,
    Y,
    YNeg,
}

impl Default for Axis {
    fn default() -> Self {
        Axis::X
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Binding {
    pub name: String,
    pub hotkey: String,
    #[serde(default)]
    pub group: GroupId,
    #[serde(default)]
    pub axis: Axis,
}

impl Binding {
    pub fn is_reset(&self) -> bool {
        self.name.eq_ignore_ascii_case("reset")
    }

    pub fn text(&self, cfg: &Config) -> String {
        cfg.control_char(self.group, self.axis).to_string()
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct ControlsCfg {
    #[serde(deserialize_with = "de_char")]
    pub x: char,
    #[serde(deserialize_with = "de_char")]
    pub x_neg: char,
    #[serde(deserialize_with = "de_char")]
    pub y: char,
    #[serde(deserialize_with = "de_char")]
    pub y_neg: char,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GroupCfg {
    pub controls: ControlsCfg,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Groups {
    pub top: GroupCfg,
    pub bottom: GroupCfg,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub groups: Groups,
    #[serde(deserialize_with = "de_char")]
    pub cgj: char,
    #[serde(default)]
    pub bindings: Vec<Binding>,
}

impl Config {
    pub fn from_json(s: &str) -> Result<Self> {
        serde_json::from_str(s).context("Error parsing JSON")
    }

    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Configuration file not found: {}", path.display()))?;
        Self::from_json(&content)
    }

    pub fn control_char(&self, group: GroupId, axis: Axis) -> char {
        let g = match group {
            GroupId::Top => &self.groups.top,
            GroupId::Bottom => &self.groups.bottom,
        };
        match axis {
            Axis::X => g.controls.x,
            Axis::XNeg => g.controls.x_neg,
            Axis::Y => g.controls.y,
            Axis::YNeg => g.controls.y_neg,
        }
    }

    pub fn all_controls(&self) -> Vec<char> {
        let mut out = Vec::with_capacity(8);
        for g in [&self.groups.top, &self.groups.bottom] {
            out.push(g.controls.x);
            out.push(g.controls.x_neg);
            out.push(g.controls.y);
            out.push(g.controls.y_neg);
        }
        out
    }
}

impl FromStr for Config {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        Self::from_json(s)
    }
}

// ---------------------------------------------------------------------------
// مک: پارس هات‌کی به نوع‌های global-hotkey (از طریق RegisterHotKey)
// ---------------------------------------------------------------------------

#[cfg(not(windows))]
pub fn parse_hotkey(spec: &str) -> Result<(Modifiers, Code)> {
    let mut mods = Modifiers::empty();
    let mut main: Option<Code> = None;

    for raw in spec.split('+') {
        let part = raw.trim();
        if part.is_empty() {
            continue;
        }
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "alt" | "option" => mods |= Modifiers::ALT,
            "shift" => mods |= Modifiers::SHIFT,
            "cmd" | "super" | "meta" => mods |= Modifiers::SUPER,
            "up" | "arrowup" => set_main(&mut main, Code::ArrowUp, part)?,
            "down" | "arrowdown" => set_main(&mut main, Code::ArrowDown, part)?,
            "left" | "arrowleft" => set_main(&mut main, Code::ArrowLeft, part)?,
            "right" | "arrowright" => set_main(&mut main, Code::ArrowRight, part)?,
            _ => {
                let code = Code::from_str(part)
                    .map_err(|_| anyhow!("Unknown key in shortcut: {part}"))?;
                set_main(&mut main, code, part)?;
            }
        }
    }

    let code = main.ok_or_else(|| anyhow!("The «{spec}» shortcut does not have a primary key"))?;
    Ok((mods, code))
}

#[cfg(not(windows))]
fn set_main(slot: &mut Option<Code>, code: Code, part: &str) -> Result<()> {
    if slot.is_some() {
        return Err(anyhow!(
            "Multiple modifier keys are not supported in a shortcut (second key: {part})"
        ));
    }
    *slot = Some(code);
    Ok(())
}

// ---------------------------------------------------------------------------
// ویندوز: پارس هات‌کی به (ctrl, alt, shift, meta, virtual_key) خام
// ---------------------------------------------------------------------------

#[cfg(windows)]
pub struct WinHotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    pub vk: u32,
}

#[cfg(windows)]
fn vk_from_name(name: &str) -> Option<u32> {
    match name {
        "ArrowUp" | "Up" => Some(0x26),
        "ArrowDown" | "Down" => Some(0x28),
        "ArrowLeft" | "Left" => Some(0x25),
        "ArrowRight" | "Right" => Some(0x27),
        "Space" => Some(0x20),
        "Enter" => Some(0x0D),
        "Tab" => Some(0x09),
        "Escape" => Some(0x1B),
        "Backspace" => Some(0x08),
        "Delete" => Some(0x2E),
        "Home" => Some(0x24),
        "End" => Some(0x23),
        "PageUp" => Some(0x21),
        "PageDown" => Some(0x22),
        _ => {
            if let Some(letter) = name.strip_prefix("Key") {
                if letter.len() == 1 {
                    let ch = letter.chars().next()?;
                    if ch.is_ascii_alphabetic() {
                        return Some(ch.to_ascii_uppercase() as u32);
                    }
                }
                None
            } else if let Some(digit) = name.strip_prefix("Digit") {
                if digit.len() == 1 {
                    let ch = digit.chars().next()?;
                    if ch.is_ascii_digit() {
                        return Some(ch as u32);
                    }
                }
                None
            } else if let Some(fn_num) = name.strip_prefix('F') {
                let n: u32 = fn_num.parse().ok()?;
                if (1..=24).contains(&n) {
                    return Some(0x70 + (n - 1));
                }
                None
            } else {
                None
            }
        }
    }
}

#[cfg(windows)]
pub fn parse_hotkey_win(spec: &str) -> Result<WinHotkey> {
    use anyhow::anyhow;

    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    let mut meta = false;
    let mut main: Option<u32> = None;

    for raw in spec.split('+') {
        let part = raw.trim();
        if part.is_empty() {
            continue;
        }
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => ctrl = true,
            "alt" | "option" => alt = true,
            "shift" => shift = true,
            "cmd" | "super" | "meta" => meta = true,
            _ => {
                if main.is_some() {
                    return Err(anyhow!(
                        "Multiple modifier keys are not supported in a shortcut (second key: {part})"
                    ));
                }
                main = Some(
                    vk_from_name(part).ok_or_else(|| anyhow!("Unknown key in shortcut: {part}"))?,
                );
            }
        }
    }

    let vk = main.ok_or_else(|| anyhow!("The «{spec}» shortcut does not have a primary key"))?;
    Ok(WinHotkey { ctrl, alt, shift, meta, vk })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> Config {
        Config::from_str(
            r#"{
              "groups": {
                "top": {
                  "controls": { "x": "U+06DF", "x_neg": "U+06E0", "y": "U+06EB", "y_neg": "U+06EC" }
                },
                "bottom": {
                  "controls": { "x": "U+08ED", "x_neg": "U+08EE", "y": "U+08EF", "y_neg": "U+08F2" }
                }
              },
              "cgj": "U+034F"
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn config_loads() {
        let c = cfg();
        assert_eq!(c.cgj, '\u{034F}');
    }

    #[cfg(windows)]
    #[test]
    fn win_hotkey_parses_letter() {
        let hk = parse_hotkey_win("Ctrl+Alt+KeyI").unwrap();
        assert!(hk.ctrl && hk.alt && !hk.shift && !hk.meta);
        assert_eq!(hk.vk, 'I' as u32);
    }

    #[cfg(not(windows))]
    #[test]
    fn mac_hotkey_parses() {
        let (_mods, code) = parse_hotkey("Ctrl+Alt+KeyR").unwrap();
        assert_eq!(code, Code::KeyR);
    }
}