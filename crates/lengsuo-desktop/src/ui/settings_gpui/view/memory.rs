//! 「记忆」面板的渲染描述（**描述层**：无 GPUI 依赖 ⇒ 无 `ui` feature 也能测）。
//!
//! 文案**逐字**取自文案全表的 `settings.memory` 命名空间（70 键），按键层级切区块：
//! - 顶层功能键（`generateMemory` / `importMemory` / `manageMemory` / `localMemory`）
//!   ⇒ 功能行；文案随客户端下发，不依赖服务端，接线层总是渲染。
//! - `chatMemory*` ⇒ 「来自对话的记忆」区块；更新时间是服务端数据 ⇒ fail-closed 双分支。
//! - `privacy.*` ⇒ 隐私说明，三段逐字拼接，`{name}` 槽位未下发 ⇒ 原样保留。
//! - `modal.*` / `errors.*` / `reset.toast.*` / `time.*` / `updating.*` 属于弹窗与
//!   交互态文案，只读面板不引用。
//!
//! 硬约定：**只读**、**fail-closed**（未下发就显示「未下发」，⛔ 不退化成空列表）、
//! ⛔ 不做授权推理、⛔ 不持有凭据、⛔ 不伪造服务端数据（未下发的槽位不编值、不编名）。

/// 面板标题（与 `PANELS` 清单中的 title 一致，接线层不另造标题）。
pub const TITLE: &str = "记忆";

/// 服务端尚未下发内容时的提示（fail-closed：不伪造数据、不退化成空列表）。
pub const NOT_DELIVERED: &str = "服务端尚未下发内容。";

/// 开场说明（`settings.memory.intro`；`{name}` 为产品名槽位，未下发 ⇒ 原样呈现）。
pub const INTRO: &str = "记忆让 {name} 记住你的偏好和习惯，对话越多，它就越懂你。";

/// `settings.memory.generateMemory`
pub const GENERATE_MEMORY: &str = "对话记忆";
/// `settings.memory.generateMemory.description`
pub const GENERATE_MEMORY_DESC: &str = "记住对话中的重要信息，让后续回答更贴合你。";
/// `settings.memory.importMemory`
pub const IMPORT_MEMORY: &str = "管理记忆";
/// `settings.memory.importMemory.description`
pub const IMPORT_MEMORY_DESC: &str = "每晚自动整理更新，可随时查看和编辑。";
/// `settings.memory.importMemory.button`
pub const IMPORT_MEMORY_BUTTON: &str = "导入";
/// `settings.memory.manageMemory`
pub const MANAGE_MEMORY: &str = "查看和编辑记忆";
/// `settings.memory.manageMemory.description`
pub const MANAGE_MEMORY_DESC: &str = "查看已保存的记忆内容，并可提交修改建议。";
/// `settings.memory.manageAction`
pub const MANAGE_ACTION: &str = "管理";
/// `settings.memory.localMemory`
pub const LOCAL_MEMORY: &str = "本地记忆";
/// `settings.memory.localMemory.description`
pub const LOCAL_MEMORY_DESC: &str = "将日常工作记忆写入到本地文件，跨会话保留上下文。";

/// `settings.memory.chatMemoryTitle`
pub const CHAT_MEMORY_TITLE: &str = "来自对话的记忆";
/// `settings.memory.chatMemoryViewEdit`
pub const CHAT_MEMORY_VIEW_EDIT: &str = "查看和编辑记忆";
/// `settings.memory.chatMemoryUpdated`（`{time}` 为更新时间槽位）
pub const CHAT_MEMORY_UPDATED: &str = "{time}从对话中更新";
/// `settings.memory.chatMemoryReset`
pub const CHAT_MEMORY_RESET: &str = "重置记忆";

/// `settings.memory.privacy.prefix`（尾随空格逐字保留）
pub const PRIVACY_PREFIX: &str = "记忆内容遵循 ";
/// `settings.memory.privacy.linkText`（`{name}` 为产品名槽位）
pub const PRIVACY_LINK: &str = "{name} 隐私政策";
/// `settings.memory.privacy.suffix`
pub const PRIVACY_SUFFIX: &str = "，仅你本人可见";

/// 一条只读行：`label` 为功能名，`value` 为说明文案或服务端数据，
/// `action` 为参考面板按钮文字的只读镜像（无点击行为）。
pub struct Row {
    pub label: &'static str,
    pub value: String,
    pub action: Option<&'static str>,
}

/// 「来自对话的记忆」区块主体（依赖服务端数据 ⇒ fail-closed 双分支）。
pub enum Body {
    /// 更新时间未下发 ⇒ 明示「未下发」，⛔ 不编时间、不退化成空清单。
    NotDelivered,
    /// 有内容 ⇒ 行清单。
    Rows(Vec<Row>),
}

/// 顶层功能行（文案随客户端下发，不依赖服务端，接线层总是渲染）。
pub fn feature_rows() -> Vec<Row> {
    vec![
        Row {
            label: GENERATE_MEMORY,
            value: GENERATE_MEMORY_DESC.to_string(),
            action: None,
        },
        Row {
            label: IMPORT_MEMORY,
            value: IMPORT_MEMORY_DESC.to_string(),
            action: Some(IMPORT_MEMORY_BUTTON),
        },
        Row {
            label: MANAGE_MEMORY,
            value: MANAGE_MEMORY_DESC.to_string(),
            action: Some(MANAGE_ACTION),
        },
        Row {
            label: LOCAL_MEMORY,
            value: LOCAL_MEMORY_DESC.to_string(),
            action: None,
        },
    ]
}

/// 生成「来自对话的记忆」区块主体。
///
/// `updated` 是 `chatMemoryUpdated` 的 `{time}` 槽位（`settings.memory.time.*`
/// 的格式化结果，如「3 天前」）；未下发（`None`）⇒ `Body::NotDelivered`（fail-closed）。
pub fn body(updated: Option<&str>) -> Body {
    match updated {
        None => Body::NotDelivered,
        Some(t) => Body::Rows(vec![Row {
            label: CHAT_MEMORY_VIEW_EDIT,
            value: CHAT_MEMORY_UPDATED.replace("{time}", t),
            action: Some(CHAT_MEMORY_RESET),
        }]),
    }
}

/// 隐私说明：`privacy.prefix` + `linkText` + `suffix` 三段逐字拼接。
pub fn privacy_line() -> String {
    format!("{PRIVACY_PREFIX}{PRIVACY_LINK}{PRIVACY_SUFFIX}")
}

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`（会把 GPUI 的 test 宏带进来遮蔽 `#[test]`）；显式列出符号。
    use super::{body, feature_rows, privacy_line, Body};

    #[test]
    fn undelivered_is_explicit_not_empty() {
        // 未下发 ⇒ 明示「未下发」语义，⛔ 不退化成空行清单
        match body(None) {
            Body::NotDelivered => {}
            Body::Rows(rows) => panic!("未下发必须是 NotDelivered，却拿到 {} 行", rows.len()),
        }
    }

    #[test]
    fn delivered_rows_carry_verbatim_copy() {
        match body(Some("3 天前")) {
            Body::NotDelivered => panic!("有内容必须是行清单"),
            Body::Rows(rows) => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].label, "查看和编辑记忆");
                assert_eq!(rows[0].value, "3 天前从对话中更新");
                assert_eq!(rows[0].action, Some("重置记忆"));
            }
        }
    }

    #[test]
    fn feature_rows_cover_top_level_keys() {
        let rows = feature_rows();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].label, "对话记忆");
        assert_eq!(rows[0].value, "记住对话中的重要信息，让后续回答更贴合你。");
        assert_eq!(rows[1].action, Some("导入"));
        assert_eq!(rows[2].action, Some("管理"));
        assert_eq!(rows[3].label, "本地记忆");
    }

    #[test]
    fn privacy_line_joins_three_verbatim_segments() {
        assert_eq!(privacy_line(), "记忆内容遵循 {name} 隐私政策，仅你本人可见");
    }
}
