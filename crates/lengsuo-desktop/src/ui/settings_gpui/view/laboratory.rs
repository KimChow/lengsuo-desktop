//! 「实验室」面板的渲染描述（**描述层**：无 GPUI 依赖 ⇒ 无 `ui` feature 也能测）。
//!
//! 文案**逐字取自仓外文案全表**的 `settings.laboratory` 命名空间（18 键全部入面板）。
//! 该命名空间键层级扁平（无二级段）⇒ 按语义切两区块：「欢迎体验实验性功能」
//! （入口徽标与功能说明）/「频道」（开关状态、关闭确认与异常反馈）。
//!
//! 硬约定：**只读**（⛔ 开关只做展示，不实现开关语义）、**fail-closed**（未下发
//! ⇒ 显式 [`NOT_DELIVERED`] 分支，不伪造数据、不退化成空列表）、⛔ 不做授权推理、
//! ⛔ 不持有凭据。
/// 面板标题（与 `PANELS` 清单中的 title 一致，接线层不另造标题）。
pub const TITLE: &str = "实验室";
/// 服务端尚未下发内容时的提示（「未下发」分支文案，验收口径豁免逐字命中）。
pub const NOT_DELIVERED: &str = "服务端尚未下发内容。";

/// `settings.laboratory.welcome`（区块一标题）。
pub const WELCOME: &str = "欢迎体验实验性功能";
/// `settings.laboratory.invitation`。
pub const INVITATION: &str = "邀你抢先体验 🎉";
/// `settings.laboratory.description1`。
pub const DESCRIPTION1: &str = "和同事、WorkBuddy 一起讨论工作";
/// `settings.laboratory.enter`。
pub const ENTER: &str = "去体验";
/// `settings.laboratory.description2`。
pub const DESCRIPTION2: &str = "@WorkBuddy 整理信息，推进工作。";
/// `settings.laboratory.preview`。
pub const PREVIEW: &str = "查看完整频道产品图";
/// `settings.laboratory.channel`（区块二标题）。
pub const CHANNEL: &str = "频道";
/// `settings.laboratory.enable`。
pub const ENABLE: &str = "开启";
/// `settings.laboratory.enabled`。
pub const ENABLED: &str = "已开启";
/// `settings.laboratory.disable`。
pub const DISABLE: &str = "关闭";
/// `settings.laboratory.disableTitle`。
pub const DISABLE_TITLE: &str = "关闭频道？";
/// `settings.laboratory.disableDescription`。
pub const DISABLE_DESCRIPTION: &str =
    "关闭后频道数据会保留，但不会收取新消息通知，可以随时再次打开。";
/// `settings.laboratory.retry`。
pub const RETRY: &str = "重试";
/// `settings.laboratory.failure`。
pub const FAILURE: &str = "暂时无法确认频道设置，请重试。";
/// `settings.laboratory.cancel`。
pub const CANCEL: &str = "取消";
/// `settings.laboratory.conflict`。
pub const CONFLICT: &str = "频道设置已在其他窗口或设备更新，请确认最新状态。";
/// `settings.laboratory.close`。
pub const CLOSE: &str = "关闭";
/// `settings.laboratory.loading`。
pub const LOADING: &str = "正在加载频道设置…";

/// 一条只读行：左标签、右说明，均逐字取自文案全表（⛔ 不可点、不可增删改）。
pub struct Row {
    /// 左列：短文案（动作/徽标/开关名）。
    pub label: &'static str,
    /// 右列：配套说明文案（无说明时为空串，接线层只渲染左列）。
    pub value: &'static str,
}

/// 一个区块：标题 + 只读行清单（标题同样逐字取自文案全表）。
pub struct Section {
    /// 区块标题。
    pub title: &'static str,
    /// 区块内的只读行。
    pub rows: Vec<Row>,
}

/// 面板主体：未下发 / 已下发两分支（fail-closed）。
pub enum Body {
    /// 未下发：接线层渲染 [`NOT_DELIVERED`] 提示，⛔ 不退化成空列表。
    NotDelivered,
    /// 已下发：按区块组织的只读行清单。
    Delivered(Vec<Section>),
}

/// 组装面板主体。
///
/// `channel_enabled`：`None` ⇒ 服务端未下发（fail-closed）；`Some(enabled)` ⇒
/// 已下发，`enabled` 为频道开关状态（只展示文案，⛔ 不实现开关语义）。
/// 全部行文案均为静态键值，不伪造服务端数据。
pub fn body(channel_enabled: Option<bool>) -> Body {
    match channel_enabled {
        None => Body::NotDelivered,
        Some(enabled) => Body::Delivered(vec![
            Section {
                title: WELCOME,
                rows: vec![
                    Row {
                        label: INVITATION,
                        value: DESCRIPTION1,
                    },
                    Row {
                        label: ENTER,
                        value: DESCRIPTION2,
                    },
                    Row {
                        label: PREVIEW,
                        value: "",
                    },
                ],
            },
            Section {
                title: CHANNEL,
                rows: vec![
                    Row {
                        label: ENABLE,
                        value: if enabled { ENABLED } else { DISABLE },
                    },
                    Row {
                        label: DISABLE_TITLE,
                        value: DISABLE_DESCRIPTION,
                    },
                    Row {
                        label: RETRY,
                        value: FAILURE,
                    },
                    Row {
                        label: CANCEL,
                        value: CONFLICT,
                    },
                    Row {
                        label: CLOSE,
                        value: LOADING,
                    },
                ],
            },
        ]),
    }
}

#[cfg(test)]
mod tests {
    use super::{body, Body};

    #[test]
    fn none_is_not_delivered() {
        assert!(matches!(body(None), Body::NotDelivered));
    }

    #[test]
    fn delivered_has_two_sections_with_verbatim_first_row() {
        let Body::Delivered(sections) = body(Some(true)) else {
            panic!("Some(..) 应进入 Delivered 分支");
        };
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].title, "欢迎体验实验性功能");
        assert_eq!(sections[0].rows[0].label, "邀你抢先体验 🎉");
        assert_eq!(sections[0].rows[0].value, "和同事、WorkBuddy 一起讨论工作");
        assert_eq!(sections[1].title, "频道");
    }

    #[test]
    fn delivered_enabled_switch_text() {
        let Body::Delivered(sections) = body(Some(true)) else {
            panic!("Some(true) 应进入 Delivered 分支");
        };
        assert_eq!(sections[1].rows[0].label, "开启");
        assert_eq!(sections[1].rows[0].value, "已开启");
    }

    #[test]
    fn delivered_disabled_switch_text() {
        let Body::Delivered(sections) = body(Some(false)) else {
            panic!("Some(false) 应进入 Delivered 分支");
        };
        assert_eq!(sections[1].rows[0].value, "关闭");
    }
}
