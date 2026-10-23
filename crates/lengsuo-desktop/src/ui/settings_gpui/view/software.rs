//! 「软件配置」面板的渲染描述（**描述层**：无 GPUI 依赖 ⇒ 无 `ui` feature 也能测）。
//!
//! 文案**逐字**取自仓外文案全表的 `settings.softwareConfig` 命名空间（22 键），
//! 按键层级切区块：`bundledRuntime*`（内置运行时）/ `runtimeTools` + `col*` +
//! `status*` + `enable`/`disable`/`reinstall`/`openInstallDir`（运行时列表）/
//! `desc.*`（随包工具的说明文案）/ `workspaceDeps` + `targetPlatform`
//! （工件空间依赖项）。
//!
//! 硬约定：**只读**、**fail-closed**（服务端未下发运行时清单就显示「未下发」提示，
//! ⛔ 不退化成空表格、⛔ 不伪造版本/状态/目录）、⛔ 不做授权推理、⛔ 不持有凭据。
/// 面板标题（`settings.softwareConfig.title`；与 `PANELS` 清单中的 title 一致）。
pub const TITLE: &str = "软件配置";
/// 「内置运行时」区块标题（`settings.softwareConfig.bundledRuntime`）。
pub const BUNDLED_RUNTIME: &str = "内置运行时";
/// 「内置运行时」区块说明（`settings.softwareConfig.bundledRuntimeDesc`）。
pub const BUNDLED_RUNTIME_DESC: &str = "允许使用随包提供的运行时工具。";
/// 「内置运行时」安装位置说明（`settings.softwareConfig.bundledRuntimePath`）。
pub const BUNDLED_RUNTIME_PATH: &str = "内置运行时安装在 ~/.workbuddy/ 目录下。";
/// 「运行时列表」区块标题（`settings.softwareConfig.runtimeTools`）。
pub const RUNTIME_TOOLS: &str = "运行时列表";
/// 表格列名（`settings.softwareConfig.colTool`）。
pub const COL_TOOL: &str = "工具";
/// 表格列名（`settings.softwareConfig.colVersion`）。
pub const COL_VERSION: &str = "版本";
/// 表格列名（`settings.softwareConfig.colStatus`）。
pub const COL_STATUS: &str = "状态";
/// 表格列名（`settings.softwareConfig.colInstallDir`）。
pub const COL_INSTALL_DIR: &str = "安装目录";
/// 表格列名（`settings.softwareConfig.colDescription`）。
pub const COL_DESCRIPTION: &str = "说明";
/// 状态取值（`settings.softwareConfig.statusEnabled`）。
pub const STATUS_ENABLED: &str = "已启用";
/// 状态取值（`settings.softwareConfig.statusDisabled`）。
pub const STATUS_DISABLED: &str = "已禁用";
/// 行内动作文案（`settings.softwareConfig.enable`；本面板只读，仅展示不可点）。
pub const ACTION_ENABLE: &str = "启用";
/// 行内动作文案（`settings.softwareConfig.disable`；本面板只读，仅展示不可点）。
pub const ACTION_DISABLE: &str = "禁用";
/// 行内动作文案（`settings.softwareConfig.reinstall`；本面板只读，仅展示不可点）。
pub const ACTION_REINSTALL: &str = "重新安装";
/// 行内动作文案（`settings.softwareConfig.openInstallDir`；本面板只读，仅展示不可点）。
pub const ACTION_OPEN_INSTALL_DIR: &str = "打开安装目录";
/// 「工件空间依赖项」区块标题（`settings.softwareConfig.workspaceDeps`）。
pub const WORKSPACE_DEPS: &str = "工件空间依赖项";
/// 「工件空间依赖项」区块说明（`settings.softwareConfig.targetPlatform`）。
pub const TARGET_PLATFORM: &str = "主要面向 Windows。";
/// 服务端尚未下发运行时清单时的提示（fail-closed：不伪造数据、不退化成空表格）。
///
/// ⚠️ 文案全表该命名空间没有「未下发」键 ⇒ 此文案只服务「未下发」分支，
/// ⛔ 不得挪作已下发内容。
pub const NOT_DELIVERED: &str = "服务端尚未下发内容。";
/// 工具说明文案（`settings.softwareConfig.desc.gitbash`）。
pub const DESC_GITBASH: &str = "在 Windows 上提供 Git 和 Bash Shell 的类 Unix 命令行环境。";
/// 工具说明文案（`settings.softwareConfig.desc.nodejs`）。
pub const DESC_NODEJS: &str = "基于 Chrome V8 引擎的 JavaScript 运行时，用于服务端开发。";
/// 工具说明文案（`settings.softwareConfig.desc.python`）。
pub const DESC_PYTHON: &str = "通用编程语言，适用于脚本编写、自动化和数据处理。";
/// 工具说明文案（`settings.softwareConfig.desc.shell`）。
pub const DESC_SHELL: &str = "macOS 上用于命令执行安全增强的随包 Shell 运行时。";

/// 一条运行时工具行（表格一行；全部字段由服务端下发，⛔ 界面不编数值/编名字）。
#[derive(Clone, Debug, PartialEq)]
pub struct ToolRow {
    /// 工具名（「工具」列）。
    pub tool: String,
    /// 版本号（「版本」列）。
    pub version: String,
    /// 是否启用（「状态」列 ⇒ [`STATUS_ENABLED`] / [`STATUS_DISABLED`]）。
    pub enabled: bool,
    /// 安装目录（「安装目录」列）。
    pub install_dir: String,
    /// 说明（「说明」列；已知工具可用 [`known_description`] 取随包文案）。
    pub description: String,
}

impl ToolRow {
    /// 「状态」列文案。
    pub fn status_text(&self) -> &'static str {
        status_text(self.enabled)
    }

    /// 开关动作文案：已启用 ⇒ 给「禁用」，已禁用 ⇒ 给「启用」（只读展示）。
    pub fn toggle_label(&self) -> &'static str {
        toggle_label(self.enabled)
    }
}

/// 「状态」列文案（true ⇒ 已启用，false ⇒ 已禁用）。
pub fn status_text(enabled: bool) -> &'static str {
    if enabled {
        STATUS_ENABLED
    } else {
        STATUS_DISABLED
    }
}

/// 开关动作文案（true ⇒ 「禁用」，false ⇒ 「启用」）。
pub fn toggle_label(enabled: bool) -> &'static str {
    if enabled {
        ACTION_DISABLE
    } else {
        ACTION_ENABLE
    }
}

/// 随包内置的工具说明文案（`settings.softwareConfig.desc.*`，按工具 id 取）。
///
/// 未知 id ⇒ `None`（fail-closed：不替服务端编说明）。
pub fn known_description(tool_id: &str) -> Option<&'static str> {
    match tool_id {
        "gitbash" => Some(DESC_GITBASH),
        "nodejs" => Some(DESC_NODEJS),
        "python" => Some(DESC_PYTHON),
        "shell" => Some(DESC_SHELL),
        _ => None,
    }
}

/// 面板主体（描述层纯函数的输出）：未下发 ⇒ 显式「未下发」语义；有内容 ⇒ 行清单。
#[derive(Clone, Debug, PartialEq)]
pub enum Body {
    /// 服务端未下发（或下发为空）运行时清单 ⇒ 表格区只出「未下发」提示。
    NotDelivered,
    /// 服务端下发了清单 ⇒ 渲染表格行。
    Content(Vec<ToolRow>),
}

/// 由服务端快照推导面板主体。
///
/// fail-closed：`None` 与空清单一律算**未下发**（⛔ 不退化成空表格）。
pub fn body(tools: Option<&[ToolRow]>) -> Body {
    match tools {
        Some(rows) if !rows.is_empty() => Body::Content(rows.to_vec()),
        _ => Body::NotDelivered,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_rows() -> Vec<ToolRow> {
        vec![
            ToolRow {
                tool: "Git Bash".into(),
                version: "1.2.3".into(),
                enabled: true,
                install_dir: "C:\\tools\\git".into(),
                description: known_description("gitbash").unwrap().into(),
            },
            ToolRow {
                tool: "Python".into(),
                version: "3.12.0".into(),
                enabled: false,
                install_dir: "C:\\tools\\python".into(),
                description: known_description("python").unwrap().into(),
            },
        ]
    }

    #[test]
    fn undelivered_snapshot_yields_not_delivered() {
        assert_eq!(body(None), Body::NotDelivered);
        // 下发为空清单同样按未下发处理（不退化成空表格）。
        assert_eq!(body(Some(&[])), Body::NotDelivered);
    }

    #[test]
    fn delivered_snapshot_yields_rows() {
        let rows = sample_rows();
        let Body::Content(got) = body(Some(&rows)) else {
            panic!("有内容必须走 Content 分支");
        };
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].tool, "Git Bash");
        // 首行文案：说明列取的是 desc.gitbash 的随包文案（逐字）。
        assert_eq!(got[0].description, DESC_GITBASH);
    }

    #[test]
    fn status_and_toggle_labels_follow_enabled() {
        let rows = sample_rows();
        assert_eq!(rows[0].status_text(), STATUS_ENABLED);
        assert_eq!(rows[0].toggle_label(), ACTION_DISABLE);
        assert_eq!(rows[1].status_text(), STATUS_DISABLED);
        assert_eq!(rows[1].toggle_label(), ACTION_ENABLE);
    }

    #[test]
    fn known_description_only_for_known_ids() {
        for id in ["gitbash", "nodejs", "python", "shell"] {
            assert!(
                known_description(id).is_some(),
                "已知工具 {id} 应有随包说明"
            );
        }
        assert!(known_description("unknown-tool").is_none());
    }

    #[test]
    fn copy_is_verbatim_from_key_table() {
        // 抽查关键文案（逐字，含全角标点）——防手改漂移。
        assert_eq!(
            BUNDLED_RUNTIME_PATH,
            "内置运行时安装在 ~/.workbuddy/ 目录下。"
        );
        assert_eq!(TARGET_PLATFORM, "主要面向 Windows。");
        assert_eq!(
            DESC_NODEJS,
            "基于 Chrome V8 引擎的 JavaScript 运行时，用于服务端开发。"
        );
        assert_eq!(NOT_DELIVERED, "服务端尚未下发内容。");
    }
}
