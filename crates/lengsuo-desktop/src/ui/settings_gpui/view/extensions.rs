//! 「扩展」面板的渲染描述（**描述层**：无 GPUI 依赖 ⇒ 无 `ui` feature 也能测）。
//!
//! 文案逐字取自文案全表 `settings.extensions` 命名空间（共 5 键）：
//! `title`=智能体、`action.enable`=开启、`action.disable`=禁用、
//! `action.cancelDisable`=取消禁用、`tag.disabled`=禁用中。
//! 该面板列出**智能体**（agent 应用）条目，每条目带启停状态与对应动作文案。
//!
//! 硬约定：**只读**（动作文案按状态展示为静态文本，⛔ 不渲染成可点按钮）、
//! **fail-closed**（未下发就显示「未下发」提示，⛔ 不退化成空列表、⛔ 不编名字/数值）、
//! ⛔ 不做授权推理、⛔ 不持有凭据。
/// 面板标题（与 `PANELS` 清单中的 title 一致，接线层不另造标题）。
pub const TITLE: &str = "扩展";
/// 服务端尚未下发内容时的提示（fail-closed：不伪造数据、不退化成空列表）。
///
/// 仅出现在**未下发分支**；服务端下发条目数据后本提示不再渲染。
pub const NOT_DELIVERED: &str = "服务端尚未下发内容。";
/// 条目类型名（文案全表 `settings.extensions.title`）。
pub const ITEM_TITLE: &str = "智能体";
/// 动作：开启（文案全表 `settings.extensions.action.enable`）。
pub const ACTION_ENABLE: &str = "开启";
/// 动作：禁用（文案全表 `settings.extensions.action.disable`）。
pub const ACTION_DISABLE: &str = "禁用";
/// 动作：取消禁用（文案全表 `settings.extensions.action.cancelDisable`）。
pub const ACTION_CANCEL_DISABLE: &str = "取消禁用";
/// 状态标签：禁用中（文案全表 `settings.extensions.tag.disabled`）。
pub const TAG_DISABLED: &str = "禁用中";

/// 一条「扩展」面板的只读行：条目名 + 状态标签 + 当前可用动作文案。
///
/// `name` 来自服务端下发数据（⛔ 描述层/接线层都不编造）；状态由数据决定动作文案。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// 服务端下发的条目名（不虚构；未下发 ⇒ 走 `Body::NotDelivered`）。
    pub name: String,
    /// 是否处于「禁用中」状态。
    pub disabled: bool,
}

impl Row {
    /// 当前状态标签：禁用中条目带标签，开启条目不带（照抄参考：仅禁用态显示标签）。
    pub fn tag(&self) -> Option<&'static str> {
        if self.disabled {
            Some(TAG_DISABLED)
        } else {
            None
        }
    }

    /// 当前可用动作文案：禁用中 ⇒「取消禁用」，开启 ⇒「禁用」，未开启 ⇒「开启」。
    pub fn action(&self) -> &'static str {
        if self.disabled {
            ACTION_CANCEL_DISABLE
        } else {
            ACTION_DISABLE
        }
    }
}

/// 面板主体：未下发（fail-closed 显示提示）或条目行清单。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    /// 服务端尚未下发条目：面板只显示提示，⛔ 不渲染空列表。
    NotDelivered,
    /// 已下发的条目行清单（空清单视作未下发处理，fail-closed）。
    Delivered(Vec<Row>),
}

/// 面板主体描述纯函数。
///
/// - `None` / `Some(空)` ⇒ `NotDelivered`（fail-closed：未下发必须显式显示，
///   ⛔ 不退化成空列表；空下发与未下发对用户不可区分，一律按未下发提示）。
/// - `Some(rows)` ⇒ `Delivered(rows)`。
pub fn body(delivered: Option<Vec<Row>>) -> Body {
    match delivered {
        Some(rows) if !rows.is_empty() => Body::Delivered(rows),
        _ => Body::NotDelivered,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 未下发（None）与空下发（Some(vec![])）都必须走 NotDelivered（fail-closed，
    /// 不退化成空列表）。
    #[test]
    fn body_none_and_empty_is_not_delivered() {
        assert_eq!(body(None), Body::NotDelivered);
        assert_eq!(body(Some(Vec::new())), Body::NotDelivered);
    }

    /// 有内容 ⇒ 行清单：行数一致、首行文案（动作/标签）来自文案全表常量。
    #[test]
    fn body_delivered_rows() {
        let rows = vec![
            Row {
                name: "agent-01".to_owned(),
                disabled: false,
            },
            Row {
                name: "agent-02".to_owned(),
                disabled: true,
            },
        ];
        let Body::Delivered(rendered) = body(Some(rows)) else {
            panic!("有内容必须返回 Delivered");
        };
        assert_eq!(rendered.len(), 2);
        assert_eq!(rendered[0].name, "agent-01");
        assert_eq!(rendered[0].action(), ACTION_DISABLE);
        assert_eq!(rendered[0].tag(), None);
        assert_eq!(rendered[1].action(), ACTION_CANCEL_DISABLE);
        assert_eq!(rendered[1].tag(), Some(TAG_DISABLED));
    }

    /// 文案常量逐字取自文案全表（`grep -F` 逐字命中口径的守护断言）。
    #[test]
    fn copy_verbatim_from_catalog() {
        assert_eq!(ITEM_TITLE, "智能体");
        assert_eq!(ACTION_ENABLE, "开启");
        assert_eq!(ACTION_DISABLE, "禁用");
        assert_eq!(ACTION_CANCEL_DISABLE, "取消禁用");
        assert_eq!(TAG_DISABLED, "禁用中");
        assert_eq!(TITLE, "扩展");
    }
}
