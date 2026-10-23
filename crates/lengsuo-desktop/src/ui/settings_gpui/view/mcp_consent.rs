//! 「MCP 授权同意」面板的渲染描述（**描述层**：无 GPUI 依赖 ⇒ 无 `ui` feature 也能测）。
//!
//! 文案逐字取自文案全表的 `poi.mcp.consent` 命名空间（实测 4 键：`title` /
//! `description` / `allow` / `deny`）。参考组件是**卡片**（ConsentCard）而非表格，
//! 主体形态因此是「标题 / 描述 / 允许 / 拒绝」，不是行清单。
//!
//! 硬约定：**只读**、**fail-closed**（未下发就显示「未下发」，⛔ 不退化成空列表）、
//! ⛔ 不做授权判定、⛔ 不持有凭据、⛔ 不伪造服务端数据。
/// 面板标题（与 `PANELS` 清单中的 title 一致，接线层不另造标题）。
pub const TITLE: &str = "MCP 授权同意";

/// 服务端尚未下发内容时的提示（fail-closed：不伪造数据、不退化成空列表）。
///
/// ⚠️ `poi.mcp.consent` 实测 4 键均无「未下发」语义 ⇒ 此文案**只**出现在
/// 「未下发」分支（验收口径对该分支豁免逐字命中）。
pub const NOT_DELIVERED: &str = "服务端尚未下发内容。";

/// 卡片标题模板（`poi.mcp.consent.title`，逐字；含 `{mcpServerName}` 占位符）。
pub const CARD_TITLE_TEMPLATE: &str = "是否允许向「{mcpServerName}」提供您的位置信息？";

/// 卡片描述（`poi.mcp.consent.description`，逐字）。
pub const CARD_DESCRIPTION: &str = "该服务需要您的位置信息来提供本地化服务推荐";

/// 「允许」动作文案（`poi.mcp.consent.allow`，逐字）。
pub const ALLOW: &str = "允许";

/// 「拒绝」动作文案（`poi.mcp.consent.deny`，逐字）。
pub const DENY: &str = "拒绝";

/// 卡片标题里的 MCP 服务名占位符。
const SERVER_NAME_PLACEHOLDER: &str = "{mcpServerName}";

/// 服务端下发的一条 MCP 授权同意请求（只读快照）。
///
/// 当前无宿主数据源 ⇒ 接线层只会传 `None`（如实显示「未下发」）；
/// 留给将来宿主接入时填充，⛔ 界面侧不伪造。
pub struct ConsentRequest {
    /// MCP 服务名（填入卡片标题的 `{mcpServerName}` 占位符）。
    pub mcp_server_name: String,
}

/// 面板主体：未下发（fail-closed）或已下发的同意卡片。
pub enum Body {
    /// 未下发 ⇒ 显示 [`NOT_DELIVERED`]，⛔ 不退化成空列表。
    NotDelivered,
    /// 已下发 ⇒ 渲染 ConsentCard（标题 / 描述 / 允许 / 拒绝）。
    Card(Card),
}

/// 已下发的同意卡片（字段文案逐字取自 `poi.mcp.consent`）。
pub struct Card {
    /// 已填充服务名的卡片标题。
    pub title: String,
    /// 卡片描述。
    pub description: &'static str,
    /// 「允许」动作文案。
    pub allow: &'static str,
    /// 「拒绝」动作文案。
    pub deny: &'static str,
}

/// 用户点「允许 / 拒绝」时上报的意图（界面只上报，⛔ 不做授权判定）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Intent {
    /// 允许。
    Allow,
    /// 拒绝。
    Deny,
}

/// 把卡片标题的 `{mcpServerName}` 占位符填成实际服务名。
fn fill_server_name(server_name: &str) -> String {
    CARD_TITLE_TEMPLATE.replace(SERVER_NAME_PLACEHOLDER, server_name)
}

/// 面板主体纯函数：未下发（`None`）⇒ [`Body::NotDelivered`]；有内容 ⇒ [`Body::Card`]。
///
/// fail-closed：`None` 显式落在 `NotDelivered`，永不退化成空卡片。
#[must_use]
pub fn body(request: Option<&ConsentRequest>) -> Body {
    match request {
        None => Body::NotDelivered,
        Some(request) => Body::Card(Card {
            title: fill_server_name(&request.mcp_server_name),
            description: CARD_DESCRIPTION,
            allow: ALLOW,
            deny: DENY,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        body, Body, ConsentRequest, ALLOW, CARD_DESCRIPTION, CARD_TITLE_TEMPLATE, DENY,
        NOT_DELIVERED, SERVER_NAME_PLACEHOLDER, TITLE,
    };

    /// 未下发 ⇒ NOT_DELIVERED 语义（fail-closed，不退化为空）。
    #[test]
    fn body_without_request_is_not_delivered() {
        assert!(matches!(body(None), Body::NotDelivered));
    }

    /// 有内容 ⇒ 卡片齐备：标题占位符被服务名替换，四路文案逐字带出。
    #[test]
    fn body_with_request_fills_card() {
        let request = ConsentRequest {
            mcp_server_name: "demo-server".to_string(),
        };
        let Body::Card(card) = body(Some(&request)) else {
            panic!("有内容必须是 Body::Card");
        };
        assert!(card.title.starts_with("是否允许向「"));
        assert!(card.title.contains("demo-server"));
        assert!(card.title.ends_with("」提供您的位置信息？"));
        assert!(!card.title.contains(SERVER_NAME_PLACEHOLDER));
        assert_eq!(card.description, CARD_DESCRIPTION);
        assert_eq!(card.allow, ALLOW);
        assert_eq!(card.deny, DENY);
    }

    /// 文案逐字钉死 `poi.mcp.consent` 的 4 键（改一个字就红）。
    #[test]
    fn copy_is_verbatim_from_poi_mcp_consent() {
        assert_eq!(
            CARD_TITLE_TEMPLATE,
            "是否允许向「{mcpServerName}」提供您的位置信息？"
        );
        assert_eq!(CARD_DESCRIPTION, "该服务需要您的位置信息来提供本地化服务推荐");
        assert_eq!(ALLOW, "允许");
        assert_eq!(DENY, "拒绝");
    }

    /// 面板标题与「未下发」提示必须可见（fail-closed 的最低要求）。
    #[test]
    fn title_and_not_delivered_are_visible() {
        assert_eq!(TITLE, "MCP 授权同意");
        assert!(!NOT_DELIVERED.trim().is_empty());
    }
}
