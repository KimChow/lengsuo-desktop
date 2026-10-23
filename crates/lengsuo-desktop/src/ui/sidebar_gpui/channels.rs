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
    format!(
        "{}：{}（{}）",
        message.sender, message.summary, message.time
    )
}

/// 数据来源标注：C9 契约未就绪 ⇒ 恒为 [`Provenance::Demo`]（⛔ 不许标 Runtime）。
#[must_use]
pub fn provenance() -> Provenance {
    Provenance::Demo
}

// === 接线层：GPUI 元素树（feature = "ui"） ===

#[cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
#[cfg(feature = "ui")]
use gpui_kit::component::Theme;
#[cfg(feature = "ui")]
use gpui_kit::*;

#[cfg(feature = "ui")]
use super::render::render_demo_banner;

/// 频道视图体：演示来源横幅 + 标题 + 消息列表（`div()` 自搭，零新组件）。
///
/// 标题逐字取自描述层的入口标题；消息行 =「发送者 + 时间 / 摘要」两行。
/// ⛔ 只读列表：没有发送入口、没有点击行为（C9 收发语义未就绪）。
#[cfg(feature = "ui")]
pub(super) fn render(
    title: &str,
    messages: &[ChannelMessage],
    provenance: Provenance,
    theme: &Theme,
) -> AnyElement {
    let mut body = div()
        .id("sidebar-channels-body")
        .flex()
        .flex_col()
        .flex_1()
        .overflow_y_scroll();
    // 如实标注：演示数据才展示横幅（当前恒为 Demo；C9 就绪前没有别的来源）。
    if provenance == Provenance::Demo {
        body = body.child(render_demo_banner(theme));
    }
    body = body.child(
        div()
            .px_2()
            .pt_2()
            .pb_1()
            .text_xs()
            .font_bold()
            .text_color(theme.muted_foreground)
            .child(title.to_owned()),
    );
    body.children(messages.iter().map(|message| {
        div()
            .flex()
            .flex_col()
            .px_2()
            .py_1()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_bold()
                            .text_color(theme.sidebar_foreground)
                            .child(message.sender),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(message.time),
                    ),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .truncate()
                    .child(message.summary),
            )
    }))
    .into_any_element()
}

#[cfg(test)]
mod tests {
    //! 频道视图体保真断言（纯函数部分，无窗口可跑）。
    //!
    //! 显式导入、不写 `use super::*`：避免 glob 带入的 GPUI `test` 属性宏
    //! 遮蔽标准库 `#[test]`（同 render_assert_tests 的约定）。

    use crate::state::datasource::Provenance;

    use super::{demo_messages, message_row_text, provenance, ChannelMessage};

    /// ① 演示消息非空，且每条三字段齐全（缺一不可：行渲染依赖全部字段）。
    #[test]
    fn demo_messages_are_nonempty_and_complete() {
        let messages = demo_messages();
        assert!(!messages.is_empty(), "演示消息列表必须非空");
        for message in &messages {
            assert!(!message.sender.is_empty(), "发送者不得为空");
            assert!(!message.summary.is_empty(), "摘要不得为空");
            assert!(!message.time.is_empty(), "时间不得为空");
        }
    }

    /// 整行文本三字段齐全：行语义与行渲染共用同一份数据。
    #[test]
    fn message_row_text_contains_all_fields() {
        let message = ChannelMessage {
            sender: "频道助理",
            summary: "汇总过去 24 小时的项目动态、待跟进事项，并生成一份简报。",
            time: "每日 09:00",
        };
        let text = message_row_text(&message);
        assert!(
            text.contains(message.sender),
            "整行文本必须含发送者：{text}"
        );
        assert!(text.contains(message.summary), "整行文本必须含摘要：{text}");
        assert!(text.contains(message.time), "整行文本必须含时间：{text}");
    }

    /// 如实标注：C9 就绪前来源恒为 Demo（⛔ 不许标 Runtime）。
    #[test]
    fn provenance_is_demo_until_runtime_contract_lands() {
        assert_eq!(provenance(), Provenance::Demo);
    }
}

#[cfg(all(test, feature = "ui"))]
mod render_tests {
    //! 频道视图体元素级断言：整帧画得出来且布局非零。
    //!
    //! 显式导入、不写 `use super::*`（同 render_assert_tests 的约定）；
    //! 元素级断言复用 render_assert_tests 的无窗口绘制骨架。

    use gpui_kit::component::Theme;
    use gpui_kit::{AppContext as _, IntoElement as _};

    use crate::ui::render_assert_tests::{assert_rendered, draw_probe};
    use crate::ui::sidebar::nav::channels_entry;
    use crate::ui::sidebar_gpui::render::HostedElement;

    use super::{demo_messages, provenance, render};

    /// ③ 视图体元素树非空：横幅 + 标题 + 全部消息行整帧渲染，布局非零。
    ///
    /// 有状态滚动容器（`.id`）在绘制期要取「当前视图」，裸画会 panic ⇒
    /// 与 nav.rs 同款，包一层 `HostedElement` 宿主实体再画。
    #[test]
    fn channels_view_body_renders_nonzero() {
        let bounds = draw_probe(|ctx| {
            let body = render(
                channels_entry(2, false).title,
                &demo_messages(),
                provenance(),
                &Theme::default(),
            );
            ctx.new(|_| HostedElement(body)).into_any_element()
        });
        assert_rendered(bounds, "频道消息视图体");
    }
}
