//! 「订阅用量」面板的渲染描述（**描述层**：无 GPUI 依赖 ⇒ 无 `ui` feature 也能测）。
//!
//! ⚠️ 本文件当前是**波次 1 占位描述**：只给面板标题与「未下发」提示。
//! 由 `ui-settings-subscription` 卡按 `docs/ui-workplan-v0.2.md` §2 补成完整描述层
//! （文案**逐字取自仓外文案全表**的 `settings.subscription` 命名空间，⛔ 不许自己编中文）。
//!
//! 硬约定：**只读**、**fail-closed**（未下发就显示「未下发」，⛔ 不退化成空列表）、
//! ⛔ 不做授权推理、⛔ 不持有凭据。
/// 面板标题（与 `PANELS` 清单中的 title 一致，接线层不另造标题）。
pub const TITLE: &str = "订阅用量";
/// 服务端尚未下发内容时的提示（fail-closed：不伪造数据、不退化成空列表）。
///
/// ⚠️ 占位文案：`ui-settings-subscription` 卡必须用文案全表里的真实文案替换它。
pub const NOT_DELIVERED: &str = "服务端尚未下发内容。";
