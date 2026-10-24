//! 协作·配额 弹层（波次 9，卡 `ui-overlay-collab-quota`）——纯数据层。
//!
//! 分层约定（同 `background_task`）：文案表 / 状态机 / 分支选择是纯数据层，GPUI 层只做映射。
//! fail-closed：拉取失败按「版本锁死」可见处理，不静默放行、不伪造配额。
//! 文案逐字取自命名空间 `collab.quota`（全表 33 键；本卡落地任务书摘录的 12 键，未摘录键不自编）。

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
