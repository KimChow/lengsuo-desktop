//! 组织域与权限可见性视图的 GPUI 接线层：把 [`crate::ui::org_view`] 的描述接到 GPUI 元素树。
//!
//! 分层约定（与 [`crate::ui::env_gpui`] 黄金范例一致）：
//! - **描述层**（`org_view`）：纯数据、无 GPUI 依赖、可在无窗口环境测。
//! - **接线层**（本模块）：只做「描述字段 → 样式」的映射，**不重新实现**业务判断。
//!   渲染分支放在子模块 [`sections`]，本文件只留类型定义与视图主体。
//!
//! 硬约束：界面**不判定权限** —— 只消费服务端下发的可见性标记，点击只上报意图。

#![cfg(feature = "ui")]

use std::rc::Rc;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

use crate::state::org_perm::Capability;
mod sections;

use crate::ui::org_view::OrgViewState;

/// 组织域的三个分区：部门树 / 成员表 / 角色表。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrgSection {
    /// 部门树（层级缩进）。
    Departments,
    /// 成员表。
    Members,
    /// 角色表。
    Roles,
}

impl OrgSection {
    /// tab 顺序。
    pub const ALL: [Self; 3] = [Self::Departments, Self::Members, Self::Roles];

    /// tab 文案。
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Departments => "部门",
            Self::Members => "成员",
            Self::Roles => "角色",
        }
    }
}

/// 界面发出的操作意图。
///
/// **只表达「用户点了哪个按钮」**，不携带也不推导权限：能不能点由服务端标记
/// 决定，意图本身只是把「点了哪颗按钮」告诉上层。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrgAction {
    /// 切换分区。
    SetSection(OrgSection),
    /// 新建部门。
    CreateDepartment,
    /// 邀请成员。
    InviteMember,
    /// 编辑成员（参数为成员 id）。
    EditMember(String),
    /// 移除成员（参数为成员 id）。
    RemoveMember(String),
    /// 新建角色。
    CreateRole,
    /// 分配角色。
    AssignRole,
    /// 编辑角色（参数为角色 id）。
    EditRole(String),
    /// 删除角色（参数为角色 id）。
    DeleteRole(String),
}

/// 操作意图分发器。接线层**不接真实数据源**，由上层注入回调。
pub type OrgActionDispatcher = Rc<dyn Fn(OrgAction)>;

/// 组织域视图的 GPUI 视图。
pub struct OrgViewGpui {
    /// 渲染描述 + 服务端能力快照（只承载，不推理）。
    state: OrgViewState,
    /// 当前分区（tab）。
    section: OrgSection,
    /// 操作意图分发器；缺省时按钮只渲染、不派发。
    dispatch: Option<OrgActionDispatcher>,
}

impl OrgViewGpui {
    /// 用一份渲染描述构造视图。
    #[must_use]
    pub fn new(state: OrgViewState) -> Self {
        Self {
            state,
            section: OrgSection::Departments,
            dispatch: None,
        }
    }

    /// 构造视图并注入操作意图分发器。
    #[must_use]
    pub fn with_dispatcher(state: OrgViewState, dispatch: OrgActionDispatcher) -> Self {
        Self {
            state,
            section: OrgSection::Departments,
            dispatch: Some(dispatch),
        }
    }

    /// 只读访问渲染描述。
    #[must_use]
    pub fn state(&self) -> &OrgViewState {
        &self.state
    }

    /// 可写访问渲染描述（注入服务端数据时用）。
    pub fn state_mut(&mut self) -> &mut OrgViewState {
        &mut self.state
    }

    /// 当前分区。
    #[must_use]
    pub fn section(&self) -> OrgSection {
        self.section
    }

    /// 切换分区。
    pub fn set_section(&mut self, section: OrgSection) {
        self.section = section;
    }
}

/// 分区的主操作：工具栏第一颗按钮对应的服务端能力。
///
/// 这里只是「哪个分区放哪颗按钮」的映射，**不是权限判断**——按钮能不能点
/// 仍然只看服务端下发的 `can_*` 标记。
const fn section_primary_cap(section: OrgSection) -> Capability {
    match section {
        OrgSection::Departments => Capability::ManageDepartment,
        OrgSection::Members => Capability::InviteMember,
        OrgSection::Roles => Capability::CreateRole,
    }
}

impl Render for OrgViewGpui {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let section = self.section;
        let state = &self.state;

        div()
            .flex()
            .flex_col()
            .h_full()
            .w_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(sections::render_tabs(section, &theme, cx))
            .child(sections::render_toolbar(section, state, &theme, cx))
            .child(
                div()
                    .id("org-content")
                    .flex_1()
                    .overflow_y_scroll()
                    .child(sections::render_content(state, section, &theme, cx)),
            )
    }
}

#[cfg(test)]
mod tests {
    // 显式导入（不用 `use super::*`）：父模块的 `use gpui_kit::*` 会把 GPUI 的
    // `test` 属性宏带进来并遮蔽标准库的 `#[test]`，导致测试无法编译。
    use super::sections::display_label;
    use super::{section_primary_cap, OrgAction, OrgSection};
    use crate::state::org_perm::Capability;

    #[test]
    fn three_sections_have_distinct_labels() {
        let labels: Vec<&str> = OrgSection::ALL.iter().map(|s| s.label()).collect();
        assert_eq!(labels.len(), 3);
        assert_eq!(labels, vec!["部门", "成员", "角色"]);
    }

    #[test]
    fn each_section_maps_to_its_own_primary_capability() {
        let caps = OrgSection::ALL.map(section_primary_cap);
        assert_eq!(caps[0], Capability::ManageDepartment);
        assert_eq!(caps[1], Capability::InviteMember);
        assert_eq!(caps[2], Capability::CreateRole);
    }

    #[test]
    fn missing_label_falls_back_to_placeholder_and_id() {
        assert_eq!(display_label(Some("研发"), "d1", "（未命名部门）"), "研发");
        assert_eq!(
            display_label(None, "d1", "（未命名部门）"),
            "（未命名部门） d1"
        );
        assert_eq!(
            display_label(Some("   "), "", "（未命名部门）"),
            "（未命名部门）"
        );
    }

    #[test]
    fn actions_carry_no_permission_information() {
        // 意图只描述「点了哪颗按钮」，不含任何权限判断。
        let action = OrgAction::DeleteRole("r-1".to_owned());
        assert_eq!(action, OrgAction::DeleteRole("r-1".to_owned()));
        assert_ne!(action, OrgAction::DeleteRole("r-2".to_owned()));
    }

    #[test]
    fn with_dispatcher_sets_dispatch_field() {
        use super::{OrgActionDispatcher, OrgViewGpui};
        use crate::state::org_perm::CapabilityFlags;
        use crate::ui::org_view::OrgViewState;
        use std::rc::Rc;

        let state = OrgViewState::new(CapabilityFlags::default());
        let dispatch: OrgActionDispatcher = Rc::new(|_| {});
        let view = OrgViewGpui::with_dispatcher(state, dispatch);
        assert!(
            view.dispatch.is_some(),
            "with_dispatcher 必须把 dispatch 设为 Some"
        );

        let state2 = OrgViewState::new(CapabilityFlags::default());
        let view2 = OrgViewGpui::new(state2);
        assert!(view2.dispatch.is_none(), "new 必须把 dispatch 设为 None");
    }

    #[test]
    fn dispatcher_closure_can_be_invoked() {
        use super::{OrgAction, OrgActionDispatcher, OrgViewGpui};
        use crate::state::org_perm::CapabilityFlags;
        use crate::ui::org_view::OrgViewState;
        use std::cell::RefCell;
        use std::rc::Rc;

        let calls: Rc<RefCell<Vec<OrgAction>>> = Rc::new(RefCell::new(Vec::new()));
        let calls_clone = calls.clone();
        let dispatch: OrgActionDispatcher = Rc::new(move |action| {
            calls_clone.borrow_mut().push(action);
        });
        let state = OrgViewState::new(CapabilityFlags::default());
        let _view = OrgViewGpui::with_dispatcher(state, dispatch.clone());
        dispatch(OrgAction::InviteMember);
        assert_eq!(calls.borrow().len(), 1);
        assert_eq!(calls.borrow()[0], OrgAction::InviteMember);
    }
}
