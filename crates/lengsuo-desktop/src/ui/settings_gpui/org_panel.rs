//! 「账号」面板：展示组织域成员清单（**只读**）。
//!
//! ⛔ 本面板**不提供增删改**：成员管理属于业务语义，不在界面接线层。
//! ⛔ 本面板**不做授权推理**：可见性完全来自服务端下发的列表。
//!
//! 硬约束：
//! - **只读**：行不可点击、不接增删改入口。
//! - **fail-closed**：加载失败必须显示错误，不得退化成空列表。
//! - **「真实为空」与「加载失败」用不同文案**（描述层的 [`AccountPanelBody`]）。
//! - C8（预备）：另设**可注入**的快照来源（[`AccountPanelModel::from_account`]）；
//!   契约未就绪 ⇒ 占位出口保持成员清单现状 + 来源如实标注
//!   （⛔ 不许把 Demo 标成 Runtime）。
#![cfg(feature = "ui")]

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::state::datasource::{LoadState, Provenance};
use crate::state::org::OrgListState;
use crate::ui::settings_gpui::view::{account_body, AccountPanelBody, AccountRow};

use super::window::SettingsGpui;

/// 账号面板模型：占位来源（成员清单）或注入的 C8 快照。
pub(super) enum AccountPanelModel {
    /// 占位来源（契约未就绪的现状）：组织成员只读清单。
    Members(AccountPanelBody),
    /// C8 消费侧快照：正文（描述层 [`AccountPanelBody::from_account`] 的产物）
    /// + 来源如实标注。
    Account(AccountPanelBody, Provenance),
}

impl AccountPanelModel {
    /// **注入入口**：C8 消费侧快照（`LoadState` 三态直通，fail-closed）+ 来源标注。
    ///
    /// 本卡只落入口不改 `window.rs`（接线由后续卡完成），故暂无仓内调用方 —— ⛔ 不许删。
    #[allow(dead_code)]
    pub(super) fn from_account<T: Into<AccountPanelBody>>(
        state: LoadState<T>,
        provenance: Provenance,
    ) -> Self {
        Self::Account(
            match state {
                LoadState::Loading => AccountPanelBody::Loading,
                LoadState::Failed(reason) => AccountPanelBody::Failed(reason),
                LoadState::Ready(snapshot) => snapshot.into(),
            },
            provenance,
        )
    }

    /// 占位出口的来源：组织成员清单（保持现状展示）。
    fn from_members(state: &OrgListState) -> Self {
        Self::Members(account_body(state))
    }

    /// 正文（两种来源共用描述层的 [`AccountPanelBody`]）。
    fn body(&self) -> &AccountPanelBody {
        match self {
            Self::Members(body) | Self::Account(body, _) => body,
        }
    }

    /// 引言行（来源如实标注）。
    fn intro(&self) -> &'static str {
        match self {
            Self::Members(_) => MEMBERS_INTRO,
            Self::Account(_, Provenance::Demo) => ACCOUNT_DEMO_INTRO,
            Self::Account(_, Provenance::Runtime) => ACCOUNT_RUNTIME_INTRO,
        }
    }

    /// 加载态文案（成员 / 账号两来源各用各的）。
    fn loading_text(&self) -> &'static str {
        match self {
            Self::Members(_) => MEMBERS_LOADING,
            Self::Account(..) => ACCOUNT_LOADING,
        }
    }

    /// 失败标题（成员 / 账号两来源各用各的）。
    fn failed_title(&self) -> &'static str {
        match self {
            Self::Members(_) => MEMBERS_FAILED_TITLE,
            Self::Account(..) => ACCOUNT_FAILED_TITLE,
        }
    }
}

/// 渲染账号面板：只读成员清单（薄封装 —— 占位来源进模型后走统一渲染）。
pub(super) fn render_org_panel(
    state: &OrgListState,
    theme: &Theme,
    _cx: &mut Context<SettingsGpui>,
) -> AnyElement {
    render_model(&AccountPanelModel::from_members(state), theme)
}

/// 模型渲染（区块结构与既有面板一致：引言行 + 三态正文；⛔ 不改区块结构）。
fn render_model(model: &AccountPanelModel, theme: &Theme) -> AnyElement {
    let mut column = div().flex().flex_col().gap_2();

    column = column.child(
        div()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(model.intro()),
    );

    match model.body() {
        AccountPanelBody::Loading => {
            column = column.child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(model.loading_text()),
            );
        }
        AccountPanelBody::Failed(message) => {
            // fail-closed：失败必须显示错误，**不得**退化成空列表。
            column = column.child(render_banner(model.failed_title(), message, theme));
        }
        AccountPanelBody::Empty => {
            column = column.child(render_empty(MEMBERS_EMPTY, theme));
        }
        AccountPanelBody::Content(rows) => {
            for row in rows {
                column = column.child(render_row(row, theme));
            }
        }
        AccountPanelBody::AccountContent(lines) => {
            for line in lines {
                column = column.child(render_line(&line.primary, &line.secondary, theme));
            }
        }
    }

    column.into_any_element()
}

// ---------------------------------------------------------------------------
// 文案：成员来源为既有文案（逐字保持，⛔ 不改）；账号来源标签逐字取自
// 文案全表（account.* 命名空间），来源标注为契约术语。
// ---------------------------------------------------------------------------

/// 成员清单引言（既有文案）。
const MEMBERS_INTRO: &str = "以下成员由服务端下发，仅供查看；界面不提供增删改。";
/// 成员加载中文案（既有文案）。
const MEMBERS_LOADING: &str = "正在加载成员…";
/// 成员失败标题（既有文案）。
const MEMBERS_FAILED_TITLE: &str = "成员加载失败";
/// 成员空态文案（既有文案）。
const MEMBERS_EMPTY: &str = "暂无成员";
/// 账号加载中文案（account.credits.fetchLoading）。
const ACCOUNT_LOADING: &str = "获取中";
/// 账号失败标题（account.credits.fetchFailed）。
const ACCOUNT_FAILED_TITLE: &str = "获取失败";
/// 快照来源 = 演示数据的引言（如实标注，⛔ 不许冒充已接通）。
const ACCOUNT_DEMO_INTRO: &str = "以下为演示数据：账号 / 订阅用量 / 能力位契约未接通。";
/// 快照来源 = 运行时数据的引言。
const ACCOUNT_RUNTIME_INTRO: &str = "以下账号数据由服务端下发，仅供查看。";

/// 一行成员：展示名 + id（薄封装到通用行区块）。
fn render_row(row: &AccountRow, theme: &Theme) -> Div {
    render_line(&row.name, &row.id, theme)
}

/// 通用行区块：主文案（加粗）+ 次文案（弱化）—— 成员行与账号快照行共用。
fn render_line(primary: &str, secondary: &str, theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(primary.to_owned()),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(secondary.to_owned()),
        )
}

/// 空态：一条弱化文案，不带「一切正常」的误导。
fn render_empty(text: &str, theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .p_4()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(text.to_owned()),
        )
}

/// 失败横幅：标题 + 错误原因（fail-closed，不让用户把错误当成空数据）。
fn render_banner(title: &str, message: &str, theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .p_4()
        .rounded_md()
        .border_1()
        .border_color(theme.danger)
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.danger)
                .child(title.to_owned()),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(message.to_owned()),
        )
}
