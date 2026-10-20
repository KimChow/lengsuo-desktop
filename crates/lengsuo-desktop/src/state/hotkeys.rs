//! 快捷键注册表：组合键 → 命令 id 的解析、匹配与冲突检测。
//!
//! 三条硬约定：
//! - **按 id 派发**：映射的值是 [`crate::theme::palette`] 的命令 id
//!   （或本模块声明的界面级命令 id），绝不按标题派发。
//! - **冲突必须报错**：同一组合键映射到两个不同命令时返回
//!   [`HotkeyError::Conflict`]，不静默取第一个。
//! - **覆盖后重新校验**：用户改键经 [`HotkeyRegistry::apply_overrides`]
//!   **原子**应用；整批覆盖完成后重新做一次冲突检测，有冲突则整批不生效。
//!
//! 本模块只做注册表与匹配逻辑；真实全局快捷键注册是打包时宿主的职责。

use std::fmt;

use thiserror::Error;

use crate::theme::palette::COMMANDS;

/// 界面级命令 id（不属于可搜索命令表，由宿主 / 视图直接处理）。
pub mod command_ids {
    /// 打开命令面板（`Ctrl+K` / `Ctrl+Shift+P`）。
    pub const PALETTE_OPEN: &str = "palette.open";
    /// 保存文件（`Ctrl+S`）。
    pub const FILE_SAVE: &str = "file.save";
    /// 关闭浮层（`Escape`）。
    pub const OVERLAY_CLOSE: &str = "overlay.close";
}

/// 全部界面级命令 id，用于校验覆盖项。
pub const APP_COMMAND_IDS: &[&str] = &[
    command_ids::PALETTE_OPEN,
    command_ids::FILE_SAVE,
    command_ids::OVERLAY_CLOSE,
];

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

/// 一条绑定：一个组合键 → 一个命令 id。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Binding {
    pub hotkey: Hotkey,
    pub command_id: &'static str,
}

/// 内置主窗快捷键（与产品逆向文档一致）。
///
/// `Ctrl+K` 与 `Ctrl+Shift+P` 指向**同一个**命令，因此不构成冲突。
pub const DEFAULT_BINDINGS: &[Binding] = &[
    Binding {
        hotkey: Hotkey::new(Modifiers::CTRL, Key::Character('k')),
        command_id: command_ids::PALETTE_OPEN,
    },
    Binding {
        hotkey: Hotkey::new(Modifiers::CTRL_SHIFT, Key::Character('p')),
        command_id: command_ids::PALETTE_OPEN,
    },
    Binding {
        hotkey: Hotkey::new(Modifiers::CTRL, Key::Character('s')),
        command_id: command_ids::FILE_SAVE,
    },
    Binding {
        hotkey: Hotkey::plain(Key::Escape),
        command_id: command_ids::OVERLAY_CLOSE,
    },
];

/// 把命令 id 解析为已知的 `'static` 值（同时完成合法性校验）。
///
/// 内置命令来自 [`COMMANDS`]；界面级动作来自 [`APP_COMMAND_IDS`]。
#[must_use]
pub fn resolve_command_id(id: &str) -> Option<&'static str> {
    COMMANDS
        .iter()
        .map(|command| command.id)
        .find(|known| *known == id)
        .or_else(|| APP_COMMAND_IDS.iter().copied().find(|known| *known == id))
}

/// 用户改键。按**命令**覆盖：同一命令的旧组合键会被释放。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotkeyOverride {
    pub command_id: String,
    pub hotkey: Hotkey,
}

impl HotkeyOverride {
    /// 从配置文本构造（命令 id + 快捷键文本）。
    pub fn parse(command_id: impl Into<String>, spec: &str) -> Result<Self, HotkeyError> {
        Ok(Self {
            command_id: command_id.into(),
            hotkey: Hotkey::parse(spec)?,
        })
    }
}

/// 注册表错误。
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum HotkeyError {
    #[error(transparent)]
    Parse(#[from] HotkeyParseError),
    #[error("未知命令 id：{0}")]
    UnknownCommand(String),
    #[error("快捷键冲突：{hotkey} 同时绑定到 {first} 与 {second}")]
    Conflict {
        hotkey: Hotkey,
        first: &'static str,
        second: &'static str,
    },
}

/// 快捷键注册表。构造与修改都保证**不存在冲突**，因此匹配结果唯一。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotkeyRegistry {
    bindings: Vec<Binding>,
}

impl HotkeyRegistry {
    /// 使用内置快捷键表；若内置表自相冲突则返回错误，不静默取第一个。
    pub fn with_defaults() -> Result<Self, HotkeyError> {
        Self::from_bindings(DEFAULT_BINDINGS.iter().copied())
    }

    /// 从任意绑定表构造；重复项去重，未知命令与冲突都报错。
    pub fn from_bindings(bindings: impl IntoIterator<Item = Binding>) -> Result<Self, HotkeyError> {
        let mut unique: Vec<Binding> = Vec::new();
        for binding in bindings {
            let command_id = resolve_command_id(binding.command_id)
                .ok_or_else(|| HotkeyError::UnknownCommand(binding.command_id.to_owned()))?;
            let binding = Binding {
                command_id,
                ..binding
            };
            if !unique.contains(&binding) {
                unique.push(binding);
            }
        }
        if let Some(conflict) = find_conflict(&unique) {
            return Err(conflict);
        }
        Ok(Self { bindings: unique })
    }

    /// 当前全部绑定（按添加顺序）。
    #[must_use]
    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }

    /// 给定一次按键事件，返回要派发的命令 id。
    ///
    /// 注册表保证无冲突，所以至多命中一条；未命中返回 `None`。
    #[must_use]
    pub fn command_for(&self, event: Hotkey) -> Option<&'static str> {
        self.bindings
            .iter()
            .find(|binding| binding.hotkey == event)
            .map(|binding| binding.command_id)
    }

    /// 新增一条绑定。组合键已被其他命令占用时返回 [`HotkeyError::Conflict`]，
    /// 且不改变注册表。
    pub fn bind(&mut self, hotkey: Hotkey, command_id: &str) -> Result<(), HotkeyError> {
        let command_id = resolve_command_id(command_id)
            .ok_or_else(|| HotkeyError::UnknownCommand(command_id.to_owned()))?;
        if let Some(existing) = self
            .bindings
            .iter()
            .find(|binding| binding.hotkey == hotkey)
        {
            if existing.command_id == command_id {
                return Ok(());
            }
            return Err(HotkeyError::Conflict {
                hotkey,
                first: existing.command_id,
                second: command_id,
            });
        }
        self.bindings.push(Binding { hotkey, command_id });
        Ok(())
    }

    /// 按命令改键（单条覆盖）。
    pub fn rebind(&mut self, command_id: &str, hotkey: Hotkey) -> Result<(), HotkeyError> {
        self.apply_overrides(&[HotkeyOverride {
            command_id: command_id.to_owned(),
            hotkey,
        }])
    }

    /// 原子应用一批用户覆盖：全部生效后统一做冲突检测。
    ///
    /// 任一条未知命令或覆盖后出现冲突，整批都不生效；用户改出来的冲突
    /// 同样会被发现（不会因为「是用户配置」就放行）。
    pub fn apply_overrides(&mut self, overrides: &[HotkeyOverride]) -> Result<(), HotkeyError> {
        let mut next = self.bindings.clone();
        for item in overrides {
            let command_id = resolve_command_id(&item.command_id)
                .ok_or_else(|| HotkeyError::UnknownCommand(item.command_id.clone()))?;
            // 覆盖是「改这个命令的键」：先释放它的旧键，再挂新键。
            next.retain(|binding| binding.command_id != command_id);
            next.push(Binding {
                hotkey: item.hotkey,
                command_id,
            });
        }
        if let Some(conflict) = find_conflict(&next) {
            return Err(conflict);
        }
        self.bindings = next;
        Ok(())
    }
}

/// 找出第一处冲突：同一组合键绑定了两个不同命令。
fn find_conflict(bindings: &[Binding]) -> Option<HotkeyError> {
    for (index, left) in bindings.iter().enumerate() {
        for right in &bindings[index + 1..] {
            if left.hotkey == right.hotkey && left.command_id != right.command_id {
                return Some(HotkeyError::Conflict {
                    hotkey: left.hotkey,
                    first: left.command_id,
                    second: right.command_id,
                });
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hotkey(spec: &str) -> Hotkey {
        Hotkey::parse(spec).expect("测试快捷键应可解析")
    }

    #[test]
    fn parses_documented_shortcuts() {
        assert_eq!(
            hotkey("Ctrl+K"),
            Hotkey::new(Modifiers::CTRL, Key::Character('k'))
        );
        assert_eq!(
            hotkey("Ctrl+Shift+P"),
            Hotkey::new(Modifiers::CTRL_SHIFT, Key::Character('p'))
        );
        assert_eq!(hotkey("Escape"), Hotkey::plain(Key::Escape));
    }

    #[test]
    fn parsing_ignores_case_order_and_spaces() {
        assert_eq!(hotkey("shift + ctrl + p"), hotkey("Ctrl+Shift+P"));
        assert_eq!(hotkey("CTRL+k"), hotkey("ctrl+K"));
        assert_eq!(hotkey("esc"), hotkey("Escape"));
    }

    #[test]
    fn invalid_specs_are_rejected() {
        assert_eq!(Hotkey::parse(""), Err(HotkeyParseError::Empty));
        assert!(matches!(
            Hotkey::parse("Ctrl"),
            Err(HotkeyParseError::MissingKey)
        ));
        assert!(matches!(
            Hotkey::parse("Ctrl+K+J"),
            Err(HotkeyParseError::MultipleKeys(_))
        ));
        assert!(matches!(
            Hotkey::parse("Hyper+K"),
            Err(HotkeyParseError::UnknownKey(_))
        ));
        assert!(matches!(
            Hotkey::parse("Ctrl+Ctrl+K"),
            Err(HotkeyParseError::DuplicateModifier(_))
        ));
        assert!(matches!(
            Hotkey::parse("Ctrl+"),
            Err(HotkeyParseError::Malformed(_))
        ));
    }

    #[test]
    fn display_is_canonical() {
        assert_eq!(hotkey("shift+ctrl+p").to_string(), "Ctrl+Shift+P");
        assert_eq!(hotkey("esc").to_string(), "Escape");
        assert_eq!(hotkey("F5").to_string(), "F5");
    }

    #[test]
    fn defaults_match_documented_shortcuts() {
        let registry = HotkeyRegistry::with_defaults().expect("内置快捷键表必须无冲突");
        assert_eq!(
            registry.command_for(hotkey("Ctrl+K")),
            Some(command_ids::PALETTE_OPEN)
        );
        assert_eq!(
            registry.command_for(hotkey("Ctrl+Shift+P")),
            Some(command_ids::PALETTE_OPEN)
        );
        assert_eq!(
            registry.command_for(hotkey("Ctrl+S")),
            Some(command_ids::FILE_SAVE)
        );
        assert_eq!(
            registry.command_for(hotkey("Escape")),
            Some(command_ids::OVERLAY_CLOSE)
        );
    }

    #[test]
    fn matching_requires_exact_modifiers() {
        let registry = HotkeyRegistry::with_defaults().unwrap();
        assert_eq!(registry.command_for(hotkey("Ctrl+Shift+K")), None);
        assert_eq!(registry.command_for(hotkey("K")), None);
        assert_eq!(registry.command_for(hotkey("Alt+K")), None);
    }

    #[test]
    fn two_keys_for_one_command_are_not_a_conflict() {
        let registry = HotkeyRegistry::with_defaults().unwrap();
        let count = registry
            .bindings()
            .iter()
            .filter(|binding| binding.command_id == command_ids::PALETTE_OPEN)
            .count();
        assert_eq!(count, 2, "一个命令可以有两个键，不算冲突");
    }

    #[test]
    fn duplicate_hotkey_errors_instead_of_taking_first() {
        let result = HotkeyRegistry::from_bindings([
            Binding {
                hotkey: hotkey("Ctrl+K"),
                command_id: "session.new",
            },
            Binding {
                hotkey: hotkey("Ctrl+K"),
                command_id: "session.list",
            },
        ]);
        match result {
            Err(HotkeyError::Conflict {
                hotkey,
                first,
                second,
            }) => {
                assert_eq!(hotkey.to_string(), "Ctrl+K");
                assert_eq!((first, second), ("session.new", "session.list"));
            }
            other => panic!("必须报冲突，不得静默取第一个：{other:?}"),
        }
    }

    #[test]
    fn bind_rejects_conflict_and_leaves_registry_untouched() {
        let mut registry = HotkeyRegistry::with_defaults().unwrap();
        let error = registry.bind(hotkey("Ctrl+S"), "session.new").unwrap_err();
        assert!(matches!(error, HotkeyError::Conflict { .. }));
        assert_eq!(
            registry.command_for(hotkey("Ctrl+S")),
            Some(command_ids::FILE_SAVE),
            "冲突失败后原绑定必须保持"
        );
    }

    #[test]
    fn dispatch_uses_id_not_title() {
        let mut registry = HotkeyRegistry::with_defaults().unwrap();
        registry.bind(hotkey("Ctrl+N"), "session.new").unwrap();
        let id = registry.command_for(hotkey("Ctrl+N")).unwrap();
        assert_eq!(id, "session.new");
        let command = COMMANDS.iter().find(|command| command.id == id).unwrap();
        assert_ne!(id, command.title, "派发依据是 id，不是标题");
    }

    #[test]
    fn user_override_conflict_is_detected() {
        let mut registry = HotkeyRegistry::with_defaults().unwrap();
        let overrides = [HotkeyOverride::parse("session.new", "Ctrl+K").unwrap()];
        let error = registry.apply_overrides(&overrides).unwrap_err();
        assert!(matches!(error, HotkeyError::Conflict { .. }));
        assert_eq!(
            registry.command_for(hotkey("Ctrl+K")),
            Some(command_ids::PALETTE_OPEN),
            "覆盖失败时注册表不得改变"
        );
        assert_eq!(registry.command_for(hotkey("Ctrl+N")), None);
    }

    #[test]
    fn successful_override_is_applied_and_revalidated() {
        let mut registry = HotkeyRegistry::with_defaults().unwrap();
        registry
            .apply_overrides(&[HotkeyOverride::parse("session.new", "Ctrl+N").unwrap()])
            .unwrap();
        assert_eq!(registry.command_for(hotkey("Ctrl+N")), Some("session.new"));
        // 再改成已占用的键：仍然必须报错。
        let error = registry
            .apply_overrides(&[HotkeyOverride::parse("session.new", "Ctrl+S").unwrap()])
            .unwrap_err();
        assert!(matches!(error, HotkeyError::Conflict { .. }));
    }

    #[test]
    fn override_releases_the_old_key() {
        let mut registry = HotkeyRegistry::with_defaults().unwrap();
        registry
            .rebind("file.save", hotkey("Ctrl+Shift+S"))
            .unwrap();
        assert_eq!(
            registry.command_for(hotkey("Ctrl+Shift+S")),
            Some(command_ids::FILE_SAVE)
        );
        assert_eq!(
            registry.command_for(hotkey("Ctrl+S")),
            None,
            "改键后旧键必须释放"
        );
    }

    #[test]
    fn unknown_command_is_rejected() {
        let mut registry = HotkeyRegistry::with_defaults().unwrap();
        assert!(matches!(
            registry.apply_overrides(&[HotkeyOverride::parse("nope.cmd", "Ctrl+N").unwrap()]),
            Err(HotkeyError::UnknownCommand(_))
        ));
        assert!(matches!(
            registry.bind(hotkey("Ctrl+N"), "nope.cmd"),
            Err(HotkeyError::UnknownCommand(_))
        ));
    }

    #[test]
    fn override_batch_is_atomic() {
        let mut registry = HotkeyRegistry::with_defaults().unwrap();
        let before = registry.bindings().to_vec();
        let overrides = [
            HotkeyOverride::parse("session.new", "Ctrl+N").unwrap(),
            HotkeyOverride::parse("session.list", "Ctrl+K").unwrap(),
        ];
        let error = registry.apply_overrides(&overrides).unwrap_err();
        assert_eq!(registry.bindings(), before, "整批覆盖失败时不得部分生效");
        let text = error.to_string();
        assert!(text.contains("session.list") && text.contains("palette.open"));
        assert!(text.contains("Ctrl+K"));
    }
}
