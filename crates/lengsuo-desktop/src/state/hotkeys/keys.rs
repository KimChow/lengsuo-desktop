//! 按键原语：修饰键、主键、组合键，以及解析错误。
//!
//! 由 `state/hotkeys.rs` 拆分而来（`e47-hotkey-split`），逻辑与原文件逐字一致。

use std::fmt;

use thiserror::Error;

/// 修饰键集合。四个修饰键互相独立，匹配时要求**完全相等**。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool,
}

impl Modifiers {
    pub const NONE: Self = Self {
        ctrl: false,
        shift: false,
        alt: false,
        meta: false,
    };

    pub const CTRL: Self = Self {
        ctrl: true,
        shift: false,
        alt: false,
        meta: false,
    };

    pub const CTRL_SHIFT: Self = Self {
        ctrl: true,
        shift: true,
        alt: false,
        meta: false,
    };
}

/// 主键。字符键统一存小写，保证 `Ctrl+K` 与 `Ctrl+k` 匹配同一组合。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Key {
    Character(char),
    Escape,
    Enter,
    Tab,
    Space,
    Backspace,
    Delete,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Function(u8),
}

impl Key {
    /// 解析单个按键名。未知名称返回 `None`，不猜测。
    fn from_token(token: &str) -> Option<Self> {
        let lower = token.to_ascii_lowercase();
        match lower.as_str() {
            "escape" | "esc" => return Some(Self::Escape),
            "enter" | "return" => return Some(Self::Enter),
            "tab" => return Some(Self::Tab),
            "space" | "spacebar" => return Some(Self::Space),
            "backspace" => return Some(Self::Backspace),
            "delete" | "del" => return Some(Self::Delete),
            "up" | "arrowup" => return Some(Self::Up),
            "down" | "arrowdown" => return Some(Self::Down),
            "left" | "arrowleft" => return Some(Self::Left),
            "right" | "arrowright" => return Some(Self::Right),
            "home" => return Some(Self::Home),
            "end" => return Some(Self::End),
            "pageup" | "pgup" => return Some(Self::PageUp),
            "pagedown" | "pgdn" => return Some(Self::PageDown),
            _ => {}
        }
        if let Some(number) = lower
            .strip_prefix('f')
            .and_then(|rest| rest.parse::<u8>().ok())
        {
            if (1..=24).contains(&number) {
                return Some(Self::Function(number));
            }
        }
        let mut chars = token.chars();
        let c = chars.next()?;
        if chars.next().is_some() {
            return None;
        }
        Some(Self::Character(if c.is_ascii_alphabetic() {
            c.to_ascii_lowercase()
        } else {
            c
        }))
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Character(c) if c.is_ascii_alphabetic() => {
                write!(f, "{}", c.to_ascii_uppercase())
            }
            Self::Character(c) => write!(f, "{c}"),
            Self::Escape => f.write_str("Escape"),
            Self::Enter => f.write_str("Enter"),
            Self::Tab => f.write_str("Tab"),
            Self::Space => f.write_str("Space"),
            Self::Backspace => f.write_str("Backspace"),
            Self::Delete => f.write_str("Delete"),
            Self::Up => f.write_str("Up"),
            Self::Down => f.write_str("Down"),
            Self::Left => f.write_str("Left"),
            Self::Right => f.write_str("Right"),
            Self::Home => f.write_str("Home"),
            Self::End => f.write_str("End"),
            Self::PageUp => f.write_str("PageUp"),
            Self::PageDown => f.write_str("PageDown"),
            Self::Function(number) => write!(f, "F{number}"),
        }
    }
}

/// 一个快捷键组合。宿主把平台键盘事件转换成同形状的值后传给
/// [`HotkeyRegistry::command_for`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Hotkey {
    pub modifiers: Modifiers,
    pub key: Key,
}

impl Hotkey {
    #[must_use]
    pub const fn new(modifiers: Modifiers, key: Key) -> Self {
        Self { modifiers, key }
    }

    #[must_use]
    pub const fn plain(key: Key) -> Self {
        Self::new(Modifiers::NONE, key)
    }

    /// 解析 `Ctrl+Shift+P` 这类配置文本。
    ///
    /// 大小写、修饰键顺序与空白都不敏感；只接受**一个**主键。
    pub fn parse(spec: &str) -> Result<Self, HotkeyParseError> {
        let spec = spec.trim();
        if spec.is_empty() {
            return Err(HotkeyParseError::Empty);
        }

        let mut modifiers = Modifiers::NONE;
        let mut seen = [false; 4];
        let mut key = None;

        for raw in spec.split('+') {
            let token = raw.trim();
            if token.is_empty() {
                return Err(HotkeyParseError::Malformed(spec.to_owned()));
            }
            if let Some(slot) = modifier_slot(token) {
                if seen[slot] {
                    return Err(HotkeyParseError::DuplicateModifier(token.to_owned()));
                }
                seen[slot] = true;
                match slot {
                    0 => modifiers.ctrl = true,
                    1 => modifiers.shift = true,
                    2 => modifiers.alt = true,
                    _ => modifiers.meta = true,
                }
                continue;
            }
            if key.is_some() {
                return Err(HotkeyParseError::MultipleKeys(token.to_owned()));
            }
            key = Some(
                Key::from_token(token)
                    .ok_or_else(|| HotkeyParseError::UnknownKey(token.to_owned()))?,
            );
        }

        Ok(Self {
            modifiers,
            key: key.ok_or(HotkeyParseError::MissingKey)?,
        })
    }
}

impl fmt::Display for Hotkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut empty = true;
        for (enabled, name) in [
            (self.modifiers.ctrl, "Ctrl"),
            (self.modifiers.shift, "Shift"),
            (self.modifiers.alt, "Alt"),
            (self.modifiers.meta, "Meta"),
        ] {
            if enabled {
                if !empty {
                    f.write_str("+")?;
                }
                f.write_str(name)?;
                empty = false;
            }
        }
        if !empty {
            f.write_str("+")?;
        }
        write!(f, "{}", self.key)
    }
}

/// 修饰键在 `seen` 数组中的槽位；不是修饰键返回 `None`。
fn modifier_slot(token: &str) -> Option<usize> {
    match token.to_ascii_lowercase().as_str() {
        "ctrl" | "control" | "ctl" => Some(0),
        "shift" => Some(1),
        "alt" | "option" | "opt" => Some(2),
        "meta" | "cmd" | "command" | "win" | "super" => Some(3),
        _ => None,
    }
}

/// 快捷键解析错误。
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum HotkeyParseError {
    #[error("快捷键不能为空")]
    Empty,
    #[error("快捷键格式不合法：{0}")]
    Malformed(String),
    #[error("未知按键：{0}")]
    UnknownKey(String),
    #[error("快捷键缺少主键（只有修饰键）")]
    MissingKey,
    #[error("快捷键只能有一个主键：{0}")]
    MultipleKeys(String),
    #[error("修饰键重复：{0}")]
    DuplicateModifier(String),
}
