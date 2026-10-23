//! 频道入口的视图体：消息列表（A15「频道入口」点进去后的界面部分）。
//!
//! C9 运行时契约（频道消息收发）未就绪 ⇒ 本文件**只做视图**：
//! - **纯函数层**（本文件上半部分）：演示条目与整行文本，无 GPUI 依赖，
//!   随 `cargo test` 一起跑。
//! - **接线层**（`feature = "ui"` 门控）：`div()` 自搭元素树，颜色一律取主题
//!   token，文案只有一个来源（演示条目字段 / 描述层入口标题）。
//!
//! ⛔ 不伪造服务端数据：全部条目都是演示数据，来源如实标注
//! [`Provenance::Demo`]（视图体顶部固定展示演示来源横幅）。
//! ⛔ 不实现收发语义：本文件没有任何「发送 / 输入」入口，点击不产生运行时行为。

use crate::state::datasource::Provenance;

/// 一条频道消息的演示条目（演示数据，非服务端数据）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelMessage {
    /// 发送者。
    pub sender: &'static str,
    /// 摘要（单行展示）。
    pub summary: &'static str,
    /// 时间标注（演示数据里是这条摘要的产出周期）。
    pub time: &'static str,
}

/// 演示消息：频道助理定时发到频道的摘要，文案逐字取自文案全表。
///
/// ⛔ 不接真实数据源；条目数与内容仅服务于视图演示。
#[must_use]
pub fn demo_messages() -> Vec<ChannelMessage> {
    vec![
        ChannelMessage {
            sender: "频道助理",
            summary: "汇总过去 24 小时的项目动态、待跟进事项，并生成一份简报。",
            time: "每日 09:00",
        },
        ChannelMessage {
            sender: "频道助理",
            summary: "从 Slack 频道提取今日站会内容，整理成结构化摘要。",
            time: "每日",
        },
        ChannelMessage {
            sender: "频道助理",
            summary: "汇总上周所有未合并 PR 的评审进度与阻塞原因。",
            time: "每周一上午",
        },
        ChannelMessage {
            sender: "频道助理",
            summary: "汇总本周任务、产物、事项，生成周报草稿。",
            time: "每周五",
        },
    ]
}

/// 一条消息的整行文本：`发送者：摘要（时间）`。
///
/// 纯函数（无 GPUI 依赖）：行语义与断言共用同一份格式。
#[must_use]
pub fn message_row_text(message: &ChannelMessage) -> String {
    format!("{}：{}（{}）", message.sender, message.summary, message.time)
}

/// 数据来源标注：C9 契约未就绪 ⇒ 恒为 [`Provenance::Demo`]（⛔ 不许标 Runtime）。
#[must_use]
pub fn provenance() -> Provenance {
    Provenance::Demo
}
