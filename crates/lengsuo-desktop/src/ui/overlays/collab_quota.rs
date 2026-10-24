//! 协作·配额 弹层（波次 9，卡 `ui-overlay-collab-quota`）——纯数据层。
//!
//! 分层约定（同 `background_task`）：文案表 / 状态机 / 分支选择是纯数据层，GPUI 层只做映射。
//! fail-closed：拉取失败按「版本锁死」可见处理，不静默放行、不伪造配额。
//! 文案逐字取自命名空间 `collab.quota`（全表 33 键；本卡落地任务书摘录的 12 键，未摘录键不自编）。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 界面文案（键与中文均逐字摘录自命名空间 `collab.quota`）。
pub mod copy {
    pub const ACTION_DISMISS: (&str, &str) = ("collab.quota.action.dismiss", "我知道了");
    pub const ACTION_EXPLORE_ENTERPRISE: (&str, &str) =
        ("collab.quota.action.exploreEnterprise", "了解企业版");
    pub const ACTION_UPGRADE: (&str, &str) = ("collab.quota.action.upgrade", "去升级");
    pub const EXCLUSIVE_BLOCKED: (&str, &str) = (
        "collab.quota.exclusive.blocked",
        "当前账号版本不支持团队协作功能，请联系企业管理员",
    );
    pub const MEMBER_BANNER_ENTERPRISE_OWNER: (&str, &str) = (
        "collab.quota.member.banner.enterpriseOwner",
        "企业版每个项目最多 {memberLimit} 位成员，移除部分成员后可继续邀请",
    );
    pub const MEMBER_BANNER_EXPERIENCE_OWNER: (&str, &str) = (
        "collab.quota.member.banner.experienceOwner",
        "体验版每个项目最多 {memberLimit} 位成员，移除部分成员后可继续邀请",
    );
    pub const MEMBER_BANNER_NON_OWNER: (&str, &str) = (
        "collab.quota.member.banner.nonOwner",
        "项目成员已达所有者「{ownerName}」账号版本的上限",
    );
    pub const MEMBER_BANNER_PRO_OWNER: (&str, &str) = (
        "collab.quota.member.banner.proOwner",
        "专业版每个项目最多 {memberLimit} 位成员，移除部分成员后可继续邀请",
    );
    pub const MEMBER_BODY_ENTERPRISE_OWNER: (&str, &str) = (
        "collab.quota.member.body.enterpriseOwner",
        "当前项目已达到企业版成员上限（{memberLimit} 人）。请移除不再参与的成员后再邀请。",
    );
    pub const MEMBER_BODY_EXPERIENCE_OWNER: (&str, &str) = (
        "collab.quota.member.body.experienceOwner",
        "当前项目已达到体验版成员上限（{memberLimit} 人）。升级后单个项目最多可容纳 100 人协作，也可以先移除不再参与的成员。",
    );
    pub const MEMBER_BODY_NON_OWNER: (&str, &str) = (
        "collab.quota.member.body.nonOwner",
        "该项目的成员上限由项目所有者「{ownerName}」的账号版本决定。当前{reach}上限，请联系所有者升级或移除成员后再邀请。",
    );
    pub const MEMBER_BODY_PRO_OWNER: (&str, &str) = (
        "collab.quota.member.body.proOwner",
        "当前项目已达到专业版成员上限（{memberLimit} 人）。如需更大团队协作，请升级企业版或联系销售。",
    );
}

/// 视图标题（逐字取自文案全表）。
pub const TITLE: &str = "协作·配额";

/// 账号版本档位：决定成员上限文案分支。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanTier {
    /// 体验版。
    Experience,
    /// 专业版。
    Pro,
    /// 企业版。
    Enterprise,
}

/// 正常态内容：版本独占锁死，或成员达限。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CollabQuotaContent {
    /// 当前账号版本不支持团队协作。
    ExclusiveBlocked,
    /// 成员已达上限：按所有者档位与「查看者是否所有者」选文案分支。
    MemberLimit {
        /// 所有者账号档位。
        tier: PlanTier,
        /// 当前查看者是否项目所有者。
        is_owner: bool,
        /// 该档位的成员上限。
        member_limit: u32,
        /// 项目所有者名字（nonOwner 文案用）。
        owner_name: String,
        /// nonOwner 正文里 `{reach}` 的实参（全表未含该词，由调用方传入）。
        reach: String,
    },
}

impl CollabQuotaContent {
    /// 横幅文案（已填占位符）；仅成员达限有横幅，版本锁死无横幅。
    pub fn banner_text(&self) -> Option<String> {
        match self {
            CollabQuotaContent::ExclusiveBlocked => None,
            CollabQuotaContent::MemberLimit {
                tier,
                is_owner,
                member_limit,
                owner_name,
                ..
            } => {
                let (_, text) = banner_copy(*tier, *is_owner);
                Some(fill_quotas(text, *member_limit, owner_name, ""))
            }
        }
    }

    /// 正文文案（已填占位符）；版本锁死用独占文案，达限按分支模板填充。
    pub fn body_text(&self) -> String {
        match self {
            CollabQuotaContent::ExclusiveBlocked => copy::EXCLUSIVE_BLOCKED.1.to_owned(),
            CollabQuotaContent::MemberLimit {
                tier,
                is_owner,
                member_limit,
                owner_name,
                reach,
            } => {
                let (_, text) = body_copy(*tier, *is_owner);
                fill_quotas(text, *member_limit, owner_name, reach)
            }
        }
    }

    /// 动作序列（键，文案）：锁死 ⇒ 我知道了 + 了解企业版；所有者达限 ⇒ 去升级 + 我知道了；非所有者达限 ⇒ 我知道了。
    pub fn actions(&self) -> &'static [(&'static str, &'static str)] {
        match self {
            CollabQuotaContent::ExclusiveBlocked => {
                &[copy::ACTION_DISMISS, copy::ACTION_EXPLORE_ENTERPRISE]
            }
            CollabQuotaContent::MemberLimit { is_owner: true, .. } => {
                &[copy::ACTION_UPGRADE, copy::ACTION_DISMISS]
            }
            CollabQuotaContent::MemberLimit { .. } => &[copy::ACTION_DISMISS],
        }
    }
}

/// 浮层视图态：加载中（默认，未拉取过 ≠ 空态）/ 失败（fail-closed）/ 空态 / 正常。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum CollabQuotaState {
    /// 配额未拉取过或正在加载（三态契约默认，≠ 空态）。
    #[default]
    Loading,
    /// 拉取失败：按版本锁死可见处理（不静默放行）。
    Failed,
    /// 已拉取且无配额信息：不弹层。
    Empty,
    /// 已拉取且有限额内容。
    Ready(CollabQuotaContent),
}

/// 横幅分支：非所有者固定 nonOwner；所有者按档位。
fn banner_copy(tier: PlanTier, is_owner: bool) -> (&'static str, &'static str) {
    if !is_owner {
        return copy::MEMBER_BANNER_NON_OWNER;
    }
    match tier {
        PlanTier::Experience => copy::MEMBER_BANNER_EXPERIENCE_OWNER,
        PlanTier::Pro => copy::MEMBER_BANNER_PRO_OWNER,
        PlanTier::Enterprise => copy::MEMBER_BANNER_ENTERPRISE_OWNER,
    }
}

/// 正文分支：与横幅同构。
fn body_copy(tier: PlanTier, is_owner: bool) -> (&'static str, &'static str) {
    if !is_owner {
        return copy::MEMBER_BODY_NON_OWNER;
    }
    match tier {
        PlanTier::Experience => copy::MEMBER_BODY_EXPERIENCE_OWNER,
        PlanTier::Pro => copy::MEMBER_BODY_PRO_OWNER,
        PlanTier::Enterprise => copy::MEMBER_BODY_ENTERPRISE_OWNER,
    }
}

/// 把模板里的 `{memberLimit}`/`{ownerName}`/`{reach}` 占位替换为实参（模板逐字，其余字符不动）。
fn fill_quotas(template: &str, member_limit: u32, owner_name: &str, reach: &str) -> String {
    template
        .replace("{memberLimit}", &member_limit.to_string())
        .replace("{ownerName}", owner_name)
        .replace("{reach}", reach)
}

// ── GPUI 层 ─────────────────────────────────────────────────────────

/// 弹层入口；无数据源 ⇒ 默认 Loading 态（三态契约：未拉取过 ≠ 空态），不伪造配额。
pub fn main_area(theme: &Theme) -> Div {
    render(theme, &CollabQuotaState::default())
}

/// 状态分支渲染：Loading ⇒ 蒙层 + 无文案加载占位；Failed ⇒ 版本锁死弹层（fail-closed 可见，
/// 不静默放行）；Empty ⇒ 不弹层；Ready ⇒ 按内容弹层。颜色一律取 theme token。
fn render(theme: &Theme, state: &CollabQuotaState) -> Div {
    match state {
        CollabQuotaState::Loading => modal_frame(theme, loading_chip(theme)),
        CollabQuotaState::Failed => content_overlay(theme, &CollabQuotaContent::ExclusiveBlocked),
        CollabQuotaState::Empty => div(),
        CollabQuotaState::Ready(content) => content_overlay(theme, content),
    }
}

/// 全屏蒙层 + 居中内容；蒙层透明度只作用于遮罩本身（内容是独立子节点）。
fn modal_frame(theme: &Theme, content: Div) -> Div {
    div()
        .absolute()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .absolute()
                .size_full()
                .bg(theme.background)
                .opacity(0.72),
        )
        .child(content)
}

/// 加载占位：全表未含加载文案键，用主题色小圆片占位（不伪造文案）。
fn loading_chip(theme: &Theme) -> Div {
    div()
        .w(px(16.))
        .h(px(16.))
        .rounded_md()
        .bg(theme.muted_foreground)
}

/// 弹层卡片：横幅（仅成员达限）+ 正文 + 动作行；文案全部来自 copy 表。
fn content_overlay(theme: &Theme, content: &CollabQuotaContent) -> Div {
    let mut card = div()
        .flex()
        .flex_col()
        .gap_3()
        .p_5()
        .w(px(420.))
        .bg(theme.background)
        .border_1()
        .border_color(theme.border)
        .rounded_md();
    if let Some(banner) = content.banner_text() {
        card = card.child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(SharedString::from(banner)),
        );
    }
    card = card.child(
        div()
            .text_sm()
            .text_color(theme.foreground)
            .child(SharedString::from(content.body_text())),
    );
    card = card.child(action_row(theme, content));
    modal_frame(theme, card)
}

/// 动作行：非「我知道了」动作按主色描边（同 `delete` 弹层的描边 chip 习惯），其余次级描边；
/// 触达高度 48px（无障碍下限）。点击接线随数据源/派发卡落地，本层只做展示映射。
fn action_row(theme: &Theme, content: &CollabQuotaContent) -> Div {
    let mut row = div().flex().flex_row().justify_end().gap_2();
    for (key, text) in content.actions() {
        let primary = *key != copy::ACTION_DISMISS.0;
        let (border, label) = if primary {
            (theme.accent, theme.accent)
        } else {
            (theme.border, theme.muted_foreground)
        };
        row = row.child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .h(px(48.))
                .min_w(px(96.))
                .px_4()
                .rounded_md()
                .border_1()
                .border_color(border)
                .text_sm()
                .text_color(label)
                .child(*text),
        );
    }
    row
}

/// 纯数据层断言 + 分支渲染不 panic。
///
/// ⛔ 不写 `use super::*`：父模块的 gpui 通配导入会带进 GPUI 的 `test` 属性宏并遮蔽
/// 标准库 `#[test]`；这里显式列出用到的符号。
#[cfg(test)]
mod tests {
    use super::copy;
    use super::{
        fill_quotas, main_area, render, CollabQuotaContent, CollabQuotaState, PlanTier, TITLE,
    };
    use gpui_kit::component::Theme;

    fn member_limit(tier: PlanTier, is_owner: bool, reach: &str) -> CollabQuotaContent {
        CollabQuotaContent::MemberLimit {
            tier,
            is_owner,
            member_limit: 5,
            owner_name: "张三".to_owned(),
            reach: reach.to_owned(),
        }
    }

    #[test]
    fn default_state_is_loading_not_empty() {
        assert_eq!(CollabQuotaState::default(), CollabQuotaState::Loading);
        assert_ne!(
            CollabQuotaState::default(),
            CollabQuotaState::Empty,
            "未拉取过 ≠ 空态（三态契约）"
        );
    }

    #[test]
    fn copy_keys_verbatim() {
        assert_eq!(
            copy::ACTION_DISMISS,
            ("collab.quota.action.dismiss", "我知道了")
        );
        assert_eq!(
            copy::ACTION_EXPLORE_ENTERPRISE,
            ("collab.quota.action.exploreEnterprise", "了解企业版")
        );
        assert_eq!(
            copy::ACTION_UPGRADE,
            ("collab.quota.action.upgrade", "去升级")
        );
        assert_eq!(
            copy::EXCLUSIVE_BLOCKED,
            (
                "collab.quota.exclusive.blocked",
                "当前账号版本不支持团队协作功能，请联系企业管理员"
            )
        );
        assert_eq!(
            copy::MEMBER_BANNER_ENTERPRISE_OWNER,
            (
                "collab.quota.member.banner.enterpriseOwner",
                "企业版每个项目最多 {memberLimit} 位成员，移除部分成员后可继续邀请"
            )
        );
        assert_eq!(
            copy::MEMBER_BANNER_EXPERIENCE_OWNER,
            (
                "collab.quota.member.banner.experienceOwner",
                "体验版每个项目最多 {memberLimit} 位成员，移除部分成员后可继续邀请"
            )
        );
        assert_eq!(
            copy::MEMBER_BANNER_NON_OWNER,
            (
                "collab.quota.member.banner.nonOwner",
                "项目成员已达所有者「{ownerName}」账号版本的上限"
            )
        );
        assert_eq!(
            copy::MEMBER_BANNER_PRO_OWNER,
            (
                "collab.quota.member.banner.proOwner",
                "专业版每个项目最多 {memberLimit} 位成员，移除部分成员后可继续邀请"
            )
        );
        assert_eq!(
            copy::MEMBER_BODY_ENTERPRISE_OWNER,
            (
                "collab.quota.member.body.enterpriseOwner",
                "当前项目已达到企业版成员上限（{memberLimit} 人）。请移除不再参与的成员后再邀请。"
            )
        );
        assert_eq!(
            copy::MEMBER_BODY_EXPERIENCE_OWNER,
            (
                "collab.quota.member.body.experienceOwner",
                "当前项目已达到体验版成员上限（{memberLimit} 人）。升级后单个项目最多可容纳 100 人协作，也可以先移除不再参与的成员。"
            )
        );
        assert_eq!(
            copy::MEMBER_BODY_NON_OWNER,
            (
                "collab.quota.member.body.nonOwner",
                "该项目的成员上限由项目所有者「{ownerName}」的账号版本决定。当前{reach}上限，请联系所有者升级或移除成员后再邀请。"
            )
        );
        assert_eq!(
            copy::MEMBER_BODY_PRO_OWNER,
            (
                "collab.quota.member.body.proOwner",
                "当前项目已达到专业版成员上限（{memberLimit} 人）。如需更大团队协作，请升级企业版或联系销售。"
            )
        );
        assert!(!TITLE.is_empty());
    }

    #[test]
    fn banner_branches_fill_limit_and_owner_name() {
        assert_eq!(
            member_limit(PlanTier::Experience, true, "")
                .banner_text()
                .as_deref(),
            Some("体验版每个项目最多 5 位成员，移除部分成员后可继续邀请")
        );
        assert_eq!(
            member_limit(PlanTier::Pro, true, "")
                .banner_text()
                .as_deref(),
            Some("专业版每个项目最多 5 位成员，移除部分成员后可继续邀请")
        );
        assert_eq!(
            member_limit(PlanTier::Enterprise, true, "")
                .banner_text()
                .as_deref(),
            Some("企业版每个项目最多 5 位成员，移除部分成员后可继续邀请")
        );
        assert_eq!(
            member_limit(PlanTier::Pro, false, "")
                .banner_text()
                .as_deref(),
            Some("项目成员已达所有者「张三」账号版本的上限")
        );
        assert_eq!(CollabQuotaContent::ExclusiveBlocked.banner_text(), None);
    }

    #[test]
    fn body_branches_fill_limit_and_reach() {
        assert_eq!(
            member_limit(PlanTier::Experience, true, "").body_text(),
            "当前项目已达到体验版成员上限（5 人）。升级后单个项目最多可容纳 100 人协作，也可以先移除不再参与的成员。"
        );
        assert_eq!(
            member_limit(PlanTier::Pro, true, "").body_text(),
            "当前项目已达到专业版成员上限（5 人）。如需更大团队协作，请升级企业版或联系销售。"
        );
        assert_eq!(
            member_limit(PlanTier::Enterprise, true, "").body_text(),
            "当前项目已达到企业版成员上限（5 人）。请移除不再参与的成员后再邀请。"
        );
        assert_eq!(
            member_limit(PlanTier::Pro, false, "已达").body_text(),
            "该项目的成员上限由项目所有者「张三」的账号版本决定。当前已达上限，请联系所有者升级或移除成员后再邀请。"
        );
    }

    #[test]
    fn exclusive_blocked_body_is_verbatim() {
        assert_eq!(
            CollabQuotaContent::ExclusiveBlocked.body_text(),
            "当前账号版本不支持团队协作功能，请联系企业管理员"
        );
    }

    #[test]
    fn actions_by_branch() {
        let blocked: &[(&str, &str)] = &[
            ("collab.quota.action.dismiss", "我知道了"),
            ("collab.quota.action.exploreEnterprise", "了解企业版"),
        ];
        assert_eq!(CollabQuotaContent::ExclusiveBlocked.actions(), blocked);
        let owner: &[(&str, &str)] = &[
            ("collab.quota.action.upgrade", "去升级"),
            ("collab.quota.action.dismiss", "我知道了"),
        ];
        assert_eq!(
            member_limit(PlanTier::Experience, true, "").actions(),
            owner
        );
        let non_owner: &[(&str, &str)] = &[("collab.quota.action.dismiss", "我知道了")];
        assert_eq!(
            member_limit(PlanTier::Enterprise, false, "").actions(),
            non_owner
        );
    }

    #[test]
    fn fill_only_replaces_present_placeholders() {
        assert_eq!(
            fill_quotas("最多 {memberLimit} 位", 3, "甲", "已达"),
            "最多 3 位"
        );
        assert_eq!(
            fill_quotas("所有者「{ownerName}」", 9, "李四", ""),
            "所有者「李四」"
        );
    }

    #[test]
    fn main_area_renders_without_panic() {
        let _ = main_area(&Theme::default());
    }

    #[test]
    fn every_state_branch_renders_without_panic() {
        let theme = Theme::default();
        let states = vec![
            CollabQuotaState::Loading,
            CollabQuotaState::Failed,
            CollabQuotaState::Empty,
            CollabQuotaState::Ready(CollabQuotaContent::ExclusiveBlocked),
            CollabQuotaState::Ready(member_limit(PlanTier::Experience, true, "")),
            CollabQuotaState::Ready(member_limit(PlanTier::Pro, false, "已达")),
        ];
        for state in &states {
            let _ = render(&theme, state);
        }
    }
}
