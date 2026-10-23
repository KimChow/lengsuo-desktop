//! 「模型」面板的渲染描述（**描述层**：无 GPUI 依赖 ⇒ 无 `ui` feature 也能编译与测试）。
//!
//! 文案**逐字取自文案全表**的 `settings.models` 命名空间（171 键），按键层级切区块：
//! [`SAVED_TITLE`]（已保存模型）/ `fields.*`（高级配置字段）/ `capability.*`（能力开关）/
//! `localModels.*`（本地模型）。动作流文案（`actions.*` / `modal.*` / `toast.*` /
//! `validation.*` / `policy.*` / `overwrite.*` / `delete.*` …）属于增删改与测连接交互，
//! 本面板**只读** ⇒ 不进描述层。
//!
//! 硬约定：
//! - **只读**：只描述「显示什么」，不描述任何编辑动作。
//! - **fail-closed**：未下发 ⇒ [`SavedModels::NotDelivered`]，空清单 ⇒ [`SavedModels::Empty`]；
//!   **未下发 ≠ 空集**，两者不互相冒充。
//! - ⛔ 不编造服务端数据：未下发的字段不进清单；已下发条目原样透传。

/// 面板标题（与 `PANELS` 清单中的 title 一致；由窗口顶栏渲染，面板体不重复出标题）。
pub const TITLE: &str = "模型";

/// 面板顶部说明（`settings.models.subtitle`）。
pub const SUBTITLE: &str = "管理写入到 ~/.codebuddy/models.json 的本地自定义模型配置。";

/// 「已保存模型」区块标题（`settings.models.savedTitle`）。
pub const SAVED_TITLE: &str = "已保存模型";

/// 未下发提示（fail-closed 兜底）：接线层还没有模型配置下发源时显示。
///
/// 该文案不在 `settings.models` 命名空间里（未下发分支的兜底提示，验收口径豁免）。
pub const NOT_DELIVERED: &str = "服务端尚未下发内容。";

/// 空态标题（`settings.models.empty.title`）：已下发但还没有任何自定义模型。
pub const EMPTY_TITLE: &str = "还没有配置自定义模型";

/// 空态说明（`settings.models.empty.description`）。
pub const EMPTY_DESCRIPTION: &str =
    "添加后会自动写入本地 models.json，并出现在聊天模型下拉的\"自定义模型\"分组中。";

/// 「高级配置」区块标题（`settings.models.fields.advanced`）。
pub const FIELDS_ADVANCED: &str = "高级配置";

/// 「本地模型」区块标题（`settings.models.localModels.title`）。
pub const LOCAL_MODELS_TITLE: &str = "本地模型";

/// 「本地模型」区块说明（`settings.models.localModels.description`）。
pub const LOCAL_MODELS_DESCRIPTION: &str = "下载并管理本地运行的 AI 模型。";

/// 只读行：标签 + 说明，均逐字取自文案全表（不是下发数据）。
pub struct Row {
    /// 行标签（字段名 / 能力开关名）。
    pub label: &'static str,
    /// 行说明（该标签对应的 `*Description` 文案）。
    pub value: String,
}

/// 面板区块：标题（参考 UI 里没有独立标题的区块用 `None`，不编标题）+ 说明行 + 只读行。
pub struct Section {
    /// 区块标题；`None` = 参考 UI 该处无标题。
    pub title: Option<&'static str>,
    /// 区块说明（无则空串）。
    pub intro: &'static str,
    /// 区块内的只读行。
    pub rows: Vec<Row>,
}

/// 「已保存模型」区块的三态（**未下发 ≠ 空集**，默认态不得显示成空态）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SavedModels {
    /// 未下发：显示 [`NOT_DELIVERED`]。
    NotDelivered,
    /// 已下发但未配置任何模型：显示 [`EMPTY_TITLE`] + [`EMPTY_DESCRIPTION`]。
    Empty,
    /// 已下发且有条目：模型参数值原样透传（不加动作、不加工）。
    Entries(Vec<String>),
}

/// 判定「已保存模型」区块该显示什么（纯函数，接线层只做映射）。
///
/// - `None`：未下发（≠ 空清单）⇒ [`SavedModels::NotDelivered`]；
/// - `Some(&[])`：已下发但为空 ⇒ [`SavedModels::Empty`]；
/// - `Some(names)`：已下发的模型参数值 ⇒ 原样透传。
#[must_use]
pub fn body(saved: Option<&[String]>) -> SavedModels {
    match saved {
        None => SavedModels::NotDelivered,
        Some([]) => SavedModels::Empty,
        Some(names) => SavedModels::Entries(names.to_vec()),
    }
}

/// 静态目录区块（不依赖下发数据，文案逐字取自文案全表）：
/// 高级配置字段说明 → 能力开关说明 → 本地模型。
#[must_use]
pub fn catalog() -> Vec<Section> {
    vec![
        Section {
            title: Some(FIELDS_ADVANCED),
            intro: "",
            rows: vec![
                field_row(
                    "最大输入 Token",
                    "单次请求可接受的最大上下文长度。留空时跟随提供商默认值。",
                ),
                field_row(
                    "最大输出 Token",
                    "单次回复可生成的最大 Token 数。留空时跟随提供商默认值。",
                ),
                field_row(
                    "默认思考强度",
                    "开启思考模式但未手动选择强度时使用的默认档位。留空时由请求层选择默认值。",
                ),
                field_row(
                    "支持的思考强度",
                    "模型选择器里展示的思考强度档位。留空表示只支持思考开关，不展示强度选择。",
                ),
            ],
        },
        Section {
            // 参考网格里能力开关直接跟在高级配置之后、没有独立标题 ⇒ 不编。
            title: None,
            intro: "",
            rows: vec![
                field_row(
                    "思考模式",
                    "将该模型标记为思考模型，客户端会据此开启相关能力和交互。",
                ),
                field_row(
                    "仅思考模式",
                    "模型只能以思考模式运行，模型选择器不会展示关闭思考的入口。",
                ),
                field_row(
                    "工具调用",
                    "允许模型调用智能体运行时暴露出来的工具和函数。",
                ),
                field_row(
                    "图片输入",
                    "允许在聊天时向该模型发送图片附件。",
                ),
                field_row(
                    "允许关闭思考",
                    "允许用户关闭思考模式。对于无法关闭思考的模型或接口，请取消勾选。",
                ),
                field_row(
                    "自定义协议",
                    "开启后将直接使用填写的接口地址，不再自动补全 /chat/completions 路径。适用于代理层或服务端已自行处理协议的场景。",
                ),
            ],
        },
        Section {
            title: Some(LOCAL_MODELS_TITLE),
            intro: LOCAL_MODELS_DESCRIPTION,
            // 本地模型清单/下载状态属下发数据：未下发不编。
            rows: Vec::new(),
        },
    ]
}

/// 组一行「标签 + 说明」（说明文案逐字取自文案全表）。
fn field_row(label: &'static str, description: &str) -> Row {
    Row {
        label,
        value: description.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    // 显式导入符号（不用 `use super::*`）：与仓内约定保持一致。
    use super::{body, catalog, SavedModels, FIELDS_ADVANCED, LOCAL_MODELS_TITLE};

    #[test]
    fn undelivered_is_explicit_not_empty() {
        // fail-closed：未下发必须是显式状态，⛔ 不冒充空清单。
        assert_eq!(body(None), SavedModels::NotDelivered);
        assert_ne!(body(None), SavedModels::Empty);
    }

    #[test]
    fn delivered_empty_is_the_empty_state_not_undelivered() {
        // 三态契约：空清单是已下发的空态，与未下发不同。
        assert_eq!(body(Some(&[])), SavedModels::Empty);
        assert_ne!(body(Some(&[])), body(None));
    }

    #[test]
    fn delivered_entries_pass_through_count_and_first_row() {
        let names = vec!["gpt-4o".to_owned(), "deepseek-chat".to_owned()];
        let SavedModels::Entries(rows) = body(Some(&names)) else {
            panic!("有内容时必须返回行清单");
        };
        assert_eq!(rows.len(), 2, "行数与下发条目一致");
        assert_eq!(rows[0], "gpt-4o", "首行文案 = 首个下发条目，原样透传");
    }

    #[test]
    fn catalog_cuts_sections_by_key_hierarchy() {
        let sections = catalog();
        assert_eq!(sections.len(), 3, "高级配置 / 能力开关 / 本地模型");

        let advanced = &sections[0];
        assert_eq!(advanced.title, Some(FIELDS_ADVANCED));
        assert_eq!(advanced.rows.len(), 4);
        assert_eq!(advanced.rows[0].label, "最大输入 Token");
        assert_eq!(
            advanced.rows[0].value,
            "单次请求可接受的最大上下文长度。留空时跟随提供商默认值。"
        );

        // 能力开关跟在高级配置之后、无独立标题（参考形态），6 个开关逐字取表。
        let capability = &sections[1];
        assert_eq!(capability.title, None);
        assert_eq!(capability.rows.len(), 6);
        assert_eq!(capability.rows[0].label, "思考模式");

        let local = &sections[2];
        assert_eq!(local.title, Some(LOCAL_MODELS_TITLE));
        assert_eq!(local.intro, "下载并管理本地运行的 AI 模型。");
        assert!(local.rows.is_empty(), "未下发的本地模型清单不编造");
    }
}
