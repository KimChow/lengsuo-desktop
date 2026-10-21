//! 主题偏好文件的读写（**本仓唯一做 IO 的地方**）。
//!
//! ⛔ 读不到 / 读坏了**都不该让程序起不来**：读失败 ⇒ 回落「跟随系统」；
//! 写失败 ⇒ 只记日志，不影响界面。

use std::path::PathBuf;

use crate::state::prefs::{decode_theme, encode_theme};
use crate::ui::settings_gpui::view::ThemeChoice;

/// 偏好文件所在目录（XDG 约定；Windows 用 `%APPDATA%`）。
#[cfg(windows)]
#[must_use]
pub fn prefs_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|base| PathBuf::from(base).join("lengsuo-desktop"))
}

/// 偏好文件所在目录（XDG 约定；Windows 用 `%APPDATA%`）。
#[cfg(not(windows))]
#[must_use]
pub fn prefs_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("lengsuo-desktop"))
}

/// 偏好文件的完整路径。
#[must_use]
pub fn prefs_path() -> Option<PathBuf> {
    Some(prefs_dir()?.join("theme.json"))
}

/// 读主题偏好。
///
/// **读不到 / 读坏了 ⇒ 回落 [`ThemeChoice::System`]**（跟随系统），
/// 这是**保守**回落：不会让用户在暗色系统上被强制亮色。
/// 第二个返回值是人读的说明（供日志/诊断），成功时是 `None`。
#[must_use]
pub fn read_theme() -> (ThemeChoice, Option<String>) {
    let Some(path) = prefs_path() else {
        return (ThemeChoice::System, Some("定位不到偏好目录".to_owned()));
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        // 文件不存在 = 首次启动 = 正常路径，不算错误。
        return (ThemeChoice::System, None);
    };
    match decode_theme(&text) {
        Ok(choice) => (choice, None),
        Err(reason) => (ThemeChoice::System, Some(reason)),
    }
}

/// 写主题偏好。
///
/// **写失败只返回错误，不 panic**（调用方决定怎么提示）。
pub fn write_theme(choice: ThemeChoice) -> Result<(), String> {
    let Some(path) = prefs_path() else {
        return Err("定位不到偏好目录".to_owned());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建偏好目录失败：{e}"))?;
    }
    std::fs::write(&path, encode_theme(choice)).map_err(|e| format!("写偏好文件失败：{e}"))
}
