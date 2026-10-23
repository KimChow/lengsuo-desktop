//! 绑定表与注册表：内置快捷键、冲突检测、用户覆盖的原子应用。
//!
//! 由 `state/hotkeys.rs` 拆分而来（`e47-hotkey-split`），逻辑与原文件逐字一致。

use thiserror::Error;

use super::command_ids;
use super::keys::{Hotkey, HotkeyParseError, Key, Modifiers};
use super::APP_COMMAND_IDS;
use crate::theme::palette::COMMANDS;

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
