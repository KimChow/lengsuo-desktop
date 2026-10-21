//! 偏好文件的读写（**本仓唯一做 IO 的地方**）。
//!
//! ⛔ 读不到 / 读坏了**都不该让程序起不来**：读失败 ⇒ 回落默认值；
//! 写失败 ⇒ 只记日志，不影响界面。
//!
//! 两份偏好各自一个文件，互不干扰：
//! - `theme.json`：`{"theme":"dark"}`（主题，e21 建立）
//! - `layout.json`：三区版式（宽度 / 折叠态，e22 建立）
//!
//! ⛔ 路径**不硬编码**：按 XDG 约定（Windows 用 `%APPDATA%`）定位目录。

use std::path::PathBuf;

use crate::state::layout_prefs::{decode_layout, encode_layout, LayoutPrefs};
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

/// 版式偏好文件的完整路径（与主题偏好**同目录、不同文件**）。
///
/// ⛔ 不复用 `theme.json`：那个文件的格式是 `{"theme":"dark"}`，
/// 塞进版式键会破坏既有格式与向后兼容。
#[must_use]
pub fn layout_prefs_path() -> Option<PathBuf> {
    Some(prefs_dir()?.join("layout.json"))
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

/// 读版式偏好。
///
/// **读不到 / 读坏了 ⇒ 回落默认值**（默认边栏 260px、右栏 320px、右栏展开），
/// 这是**保守**回落：不会让用户拿到一个宽度为 0 的窄条面板。
/// 第二个返回值是人读的说明（供日志 / 诊断），成功时是 `None`。
///
/// 三层结构（与 [`read_theme`] 同款）：
/// 1. 定位不到目录 ⇒ `(默认值, Some("定位不到偏好目录"))`
/// 2. 文件不存在 ⇒ `(默认值, None)`（首次启动 = 正常路径，不算错误）
/// 3. 解析失败 ⇒ `(默认值, Some(原因))`
#[must_use]
pub fn read_layout() -> (LayoutPrefs, Option<String>) {
    let Some(path) = layout_prefs_path() else {
        return (LayoutPrefs::default(), Some("定位不到偏好目录".to_owned()));
    };
    read_layout_from(&path)
}

/// 读版式偏好的**路径版**（测试用：可指定任意文件，不必碰真实配置目录）。
#[must_use]
fn read_layout_from(path: &std::path::Path) -> (LayoutPrefs, Option<String>) {
    let Ok(text) = std::fs::read_to_string(path) else {
        // 文件不存在 = 首次启动 = 正常路径，不算错误。
        return (LayoutPrefs::default(), None);
    };
    match decode_layout(&text) {
        Ok(prefs) => (prefs, None),
        Err(reason) => (LayoutPrefs::default(), Some(reason)),
    }
}

/// 写版式偏好。
///
/// **写失败只返回错误，不 panic**（调用方决定怎么提示；
/// 接线层的约定是「只记日志，不影响交互」）。
pub fn write_layout(prefs: LayoutPrefs) -> Result<(), String> {
    let Some(path) = layout_prefs_path() else {
        return Err("定位不到偏好目录".to_owned());
    };
    write_layout_to(&path, prefs)
}

/// 写版式偏好的**路径版**（测试用：可指定任意文件，不必碰真实配置目录）。
fn write_layout_to(path: &std::path::Path, prefs: LayoutPrefs) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建偏好目录失败：{e}"))?;
    }
    std::fs::write(path, encode_layout(prefs)).map_err(|e| format!("写偏好文件失败：{e}"))
}

#[cfg(test)]
mod tests {
    // 显式导入（不用 `use super::*`），理由见速查表 §12.3。
    use std::path::PathBuf;

    use super::{layout_prefs_path, read_layout_from, write_layout_to};
    use crate::state::layout_prefs::LayoutPrefs;

    /// 一个**临时**偏好目录（测试不碰真实配置目录）。
    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lengsuo-e22-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("临时目录应可创建");
        dir
    }

    #[test]
    fn layout_prefs_lives_in_its_own_file_next_to_the_theme_file() {
        let Some(layout) = layout_prefs_path() else {
            // 定位不到偏好目录的环境（无 HOME / XDG_CONFIG_HOME）：跳过。
            return;
        };
        let name = layout
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        assert_eq!(name, "layout.json", "版式偏好必须是独立文件");
        let Some(theme) = super::prefs_path() else {
            return;
        };
        assert_ne!(layout, theme, "版式偏好不得写进 theme.json");
    }

    #[test]
    fn a_missing_file_is_not_an_error() {
        let dir = temp_dir("missing");
        let path = dir.join("layout.json");
        let (prefs, note) = read_layout_from(&path);
        assert_eq!(prefs, LayoutPrefs::default(), "首次启动回落默认值");
        assert!(note.is_none(), "文件不存在不算错误：{note:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_broken_file_falls_back_with_a_reason() {
        let dir = temp_dir("broken");
        let path = dir.join("layout.json");
        std::fs::write(&path, "{ this is not json").expect("应可写测试文件");
        let (prefs, note) = read_layout_from(&path);
        assert_eq!(prefs, LayoutPrefs::default(), "读坏了回落默认值");
        assert!(note.is_some(), "读坏了必须给出原因，不能静默吞错");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn writing_then_reading_round_trips() {
        let dir = temp_dir("roundtrip");
        let path = dir.join("nested").join("layout.json");
        let prefs = LayoutPrefs {
            sidebar_width: 288.0,
            right_width: 352.0,
            right_collapsed: true,
        };
        write_layout_to(&path, prefs).expect("应能写入");
        let (read, note) = read_layout_from(&path);
        assert_eq!(read, prefs, "写入的内容必须能读回来");
        assert!(note.is_none(), "读回来不应报错：{note:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
