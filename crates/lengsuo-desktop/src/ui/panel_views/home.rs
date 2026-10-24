//! 主区默认页「首页」（波次 9，卡 `ui-rail-home`，差异 D3/D7）。
//!
//! 参照形态（`ui-parity-ref-snapshot.md` §三 D3/D7）：
//! 大标题（`home.header.title`）+ 副标题按场景切换（`home.header.subtitle*`）+
//! 场景徽章行 + 4 条推荐问句 + 底部 AI 免责声明（`home.aiDisclaimer`）。
//! 硬约束：文案逐字取自命名空间 `home.*`；无任务/空间数据时如实「未下发」（fail-closed）；
//! 颜色一律主题 token；点击只上报意图，不判权限。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 标题（逐字：`home.header.title`）。
pub const TITLE: &str = "WorkBuddy, 我帮你";
/// 副标题·职场（逐字：`home.header.subtitle`）。
pub const SUBTITLE: &str = "你的职场超能力";
/// 副标题·开发（逐字：`home.header.subtitle.coding`）。
pub const SUBTITLE_CODING: &str = "你的开发超能力";
/// 副标题·设计（逐字：`home.header.subtitle.design`）。
pub const SUBTITLE_DESIGN: &str = "你的设计超能力";
/// AI 免责声明（逐字：`home.aiDisclaimer`）。
pub const AI_DISCLAIMER: &str = "内容由 AI 生成，请核实重要信息";
/// 推荐问句区标题（逐字：`home.relatedPlaybooks.heading`）。
pub const PLAYBOOKS_HEADING: &str = "不知道做什么，试试最佳实践案例";
/// 换一批（逐字：`home.relatedPlaybooks.refresh`）。
pub const REFRESH: &str = "换一批";
/// 查看更多（逐字：`home.relatedPlaybooks.viewMore`）。
pub const VIEW_MORE: &str = "查看更多";
/// 数据未下发说明（fail-closed）。
pub const NOT_DELIVERED: &str = "未下发";

/// 场景（三枚徽章；标题行随场景切副标题）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scene {
    /// 职场。
    Working,
    /// 开发。
    Coding,
    /// 设计。
    Design,
}

impl Scene {
    /// 副标题（逐字）。
    pub fn subtitle(self) -> &'static str {
        match self {
            Scene::Working => SUBTITLE,
            Scene::Coding => SUBTITLE_CODING,
            Scene::Design => SUBTITLE_DESIGN,
        }
    }
}

/// 推荐问句（demo 数据；真实来源等 C1~C10，届时只换源不改视图）。
pub const SUGGESTIONS: [&str; 4] = [
    "帮我写一份周报",
    "总结这个网页的要点",
    "把这段文字翻译成英文",
    "为新产品起一个名字",
];

/// 渲染首页主区（视图是状态的纯函数；`delivered=None` ⇒ 未下发 fail-closed）。
pub fn main_area(theme: &Theme, scene: Scene, delivered: Option<&[&str]>) -> Div {
    let mut area = div().flex().flex_col().items_center().gap_4();
    let _ = theme;
    match delivered {
        None => {
            area = area
                .child(TITLE)
                .child(scene.subtitle())
                .child(NOT_DELIVERED)
                .child(AI_DISCLAIMER);
        }
        Some(items) => {
            area = area.child(TITLE).child(scene.subtitle());
            for s in items {
                area = area.child(SharedString::from(format!("{s} ↗")));
            }
            area = area
                .child(PLAYBOOKS_HEADING)
                .child(REFRESH)
                .child(VIEW_MORE);
            area = area.child(AI_DISCLAIMER);
        }
    }
    area
}

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块 `use gpui_kit::*` 的 `test` 属性宏会遮蔽 `#[test]`。
    use super::{main_area, Scene, AI_DISCLAIMER, SUBTITLE, SUBTITLE_CODING, SUGGESTIONS, TITLE};

    #[test]
    fn title_verbatim() {
        assert_eq!(TITLE, "WorkBuddy, 我帮你");
        assert_eq!(SUBTITLE, "你的职场超能力");
        assert_eq!(SUBTITLE_CODING, "你的开发超能力");
    }

    #[test]
    fn scene_subtitle_mapping() {
        assert_eq!(Scene::Working.subtitle(), SUBTITLE);
        assert_eq!(Scene::Coding.subtitle(), SUBTITLE_CODING);
    }

    #[test]
    fn suggestions_count() {
        assert_eq!(SUGGESTIONS.len(), 4);
    }

    #[test]
    fn disclaimer_verbatim() {
        assert_eq!(AI_DISCLAIMER, "内容由 AI 生成，请核实重要信息");
    }

    #[test]
    fn not_delivered_is_fail_closed() {
        // None ⇒ 仍渲染标题（不空白），并带「未下发」语义分支（构建不 panic 即可）。
        let theme = gpui_kit::component::Theme::default();
        let _ = main_area(&theme, Scene::Working, None);
        let _ = main_area(&theme, Scene::Design, Some(&SUGGESTIONS));
    }
}
