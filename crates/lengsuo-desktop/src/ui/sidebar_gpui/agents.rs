//! 智能体管理入口的视图体：智能体列表（A15「智能体管理」点进去后的界面部分）。
//!
//! C9 运行时契约（智能体 CRUD / 多代理角色模型）未就绪 ⇒ 本文件**只做视图**：
//! - **纯函数层**（本文件上半部分）：演示条目与状态文案，无 GPUI 依赖，
//!   随 `cargo test` 一起跑。
//! - **接线层**（`feature = "ui"` 门控）：`div()` 自搭元素树，颜色一律取主题
//!   token，文案只有一个来源（演示条目字段 / 描述层入口标题）。
//!
//! ⛔ 不伪造服务端数据：全部条目都是演示数据，来源如实标注
//! [`Provenance::Demo`]（视图体顶部固定展示演示来源横幅）。
//! ⛔ 不实现 CRUD 语义：本文件没有任何「创建 / 删除」入口，点击不产生运行时行为。

use crate::state::datasource::Provenance;

/// 一个智能体的演示条目（演示数据，非服务端数据）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentEntry {
    /// 名称。
    pub name: &'static str,
    /// 角色文案（多代理角色模型未定 ⇒ 只展示一行角色，不做角色判定）。
    pub role: &'static str,
    /// 是否在线。
    pub online: bool,
}

/// 演示智能体：文案逐字取自文案全表；条目数与入口计数角标一致（demo_state 的 3）。
///
/// ⛔ 不接真实数据源；条目数与内容仅服务于视图演示。
#[must_use]
pub fn demo_agents() -> Vec<AgentEntry> {
    vec![
        AgentEntry {
            name: "每日站会摘要整理",
            role: "频道助理",
            online: true,
        },
        AgentEntry {
            name: "每周五生成项目周报",
            role: "频道助理",
            online: false,
        },
        AgentEntry {
            name: "竞品官网定时巡检",
            role: "频道助理",
            online: true,
        },
    ]
}

/// 在线状态文案：双态必须可分。
///
/// 文案逐字取自文案全表的助理状态词（在线 / 空闲；表内无独立「离线」键）。
#[must_use]
pub fn agent_status_label(online: bool) -> &'static str {
    if online { "在线" } else { "空闲" }
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

/// 智能体视图体：演示来源横幅 + 标题 + 智能体列表（`div()` 自搭，零新组件）。
///
/// 行 =「名称 + 角色 / 在线状态胶囊」；状态文案只来自 [`agent_status_label`]。
/// ⛔ 只读列表：没有创建 / 删除入口（C9 CRUD 语义未就绪）。
#[cfg(feature = "ui")]
pub(super) fn render(
    title: &str,
    agents: &[AgentEntry],
    provenance: Provenance,
    theme: &Theme,
) -> AnyElement {
    let mut body = div()
        .id("sidebar-agents-body")
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
    body.children(agents.iter().map(|agent| {
        // 在线是行动信号 ⇒ 强调色描边；空闲弱一档（同会话区「置顶 / 归档」胶囊）。
        let status_color = if agent.online {
            theme.accent
        } else {
            theme.muted_foreground
        };
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .px_2()
            .py_1()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.sidebar_foreground)
                            .child(agent.name),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(agent.role),
                    ),
            )
            .child(
                div()
                    .rounded_full()
                    .px_1()
                    .text_xs()
                    .text_color(status_color)
                    .border_1()
                    .border_color(status_color)
                    .child(agent_status_label(agent.online)),
            )
    }))
    .into_any_element()
}
