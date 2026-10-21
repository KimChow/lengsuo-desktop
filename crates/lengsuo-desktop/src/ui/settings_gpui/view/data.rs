//! 数据管理面板的**纯逻辑**判定（描述层，无 GPUI 依赖，可无窗口测）。
//!
//! ⛔ 只做「分组视图状态 → 面板该显示什么」的映射，不实现任何业务语义：
//! 环境数据的增删改属于业务层，不在界面接线层（见 `data_panel.rs`）。
//!
//! 分层约定同 [`super::permissions`]：接线层只映射这里的判定结果。

use crate::state::env_group::{EmptyReason, EnvGroupState, GroupSummary};
use crate::state::env_list::EnvStatus;

/// 未命名分组的占位文案（`name` 缺失或空白时用，**不拿 id 冒充名字**）。
pub const GROUP_NAME_PLACEHOLDER: &str = "未命名分组";

/// 面板顶部要显示的汇总。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataSummary {
    /// 最近一次注入的原始条目数（含解析失败）。
    pub total: usize,
    /// 解析失败的条目数。
    pub invalid: usize,
    /// 分组数（不含未分组）。
    pub group_count: usize,
    /// 未分组条目数。
    pub ungrouped: usize,
}

/// 面板主体该显示什么。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataPanelBody {
    /// 正在加载。
    Loading,
    /// 加载失败（fail-closed：必须显示错误，不得退化成空列表）。
    Failed(String),
    /// 空态：原始为空或筛选后为空，两者文案不同（见 [`empty_state_text`]）。
    Empty(EmptyReason),
    /// 有可展示内容：顶部汇总 + 分组清单 + 未分组行。
    Content(DataSummary),
}

/// 算面板顶部的汇总（只读 [`EnvGroupState`] 的公开快照方法，不新增推理）。
#[must_use]
pub fn data_summary(state: &EnvGroupState) -> DataSummary {
    DataSummary {
        total: state.raw_count(),
        invalid: state.invalid_count(),
        group_count: state.groups().len(),
        ungrouped: state.ungrouped_count(),
    }
}

/// 分组在界面上显示的名字（`name` 缺失或空白时给明确占位，不拿 id 冒充）。
#[must_use]
pub fn group_display_name(group: &GroupSummary) -> String {
    match group.name.as_deref().map(str::trim) {
        Some(name) if !name.is_empty() => name.to_owned(),
        _ => GROUP_NAME_PLACEHOLDER.to_owned(),
    }
}

/// 空态文案：`NoData` 与 `FilteredOut` 必须用**不同**文案（铁律 4）。
#[must_use]
pub const fn empty_state_text(reason: EmptyReason) -> &'static str {
    match reason {
        EmptyReason::NoData => "还没有任何环境数据",
        EmptyReason::FilteredOut => "当前筛选下没有可展示的环境",
    }
}

/// 面板主体该展示什么（fail-closed：加载失败优先于空态判定）。
#[must_use]
pub fn data_panel_body(state: &EnvGroupState) -> DataPanelBody {
    match state.status() {
        EnvStatus::Loading => DataPanelBody::Loading,
        EnvStatus::Failed => {
            DataPanelBody::Failed(state.last_error().unwrap_or("环境数据加载失败").to_owned())
        }
        EnvStatus::Ready | EnvStatus::Degraded => {
            if let Some(reason) = state.empty_reason() {
                DataPanelBody::Empty(reason)
            } else {
                DataPanelBody::Content(data_summary(state))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // 显式导入符号（不用 `use super::*`）：与仓内约定保持一致。
    use super::{
        data_panel_body, data_summary, empty_state_text, group_display_name, DataPanelBody,
        DataSummary, GROUP_NAME_PLACEHOLDER,
    };
    use crate::state::env_group::{EmptyReason, EnvGroupState, GroupSummary};
    use crate::state::env_list::EnvStatus;
    use serde_json::json;

    #[test]
    fn empty_state_summary_is_all_zeroes() {
        let s = EnvGroupState::new();
        assert_eq!(
            data_summary(&s),
            DataSummary {
                total: 0,
                invalid: 0,
                group_count: 0,
                ungrouped: 0,
            }
        );
    }

    #[test]
    fn summary_counts_groups_and_ungrouped() {
        let mut s = EnvGroupState::new();
        s.apply_entries(vec![
            json!({"id": "e1", "group_id": "g1", "group_name": "华东组"}),
            json!({"id": "e2", "group_id": "g1"}),
            json!({"id": "e3", "group_id": "g2", "group_name": "华南组"}),
            json!({"id": "e4"}),
            json!({"id": "e5"}),
            json!({"name": "坏条目", "group_id": "g2"}),
        ]);
        let summary = data_summary(&s);
        assert_eq!(summary.total, 6);
        assert_eq!(summary.invalid, 1);
        assert_eq!(summary.group_count, 2);
        assert_eq!(summary.ungrouped, 2);
    }

    #[test]
    fn group_display_name_uses_name_or_placeholder() {
        let named = GroupSummary {
            id: "g1".into(),
            name: Some("生产环境".into()),
            env_count: 1,
            invalid_count: 0,
        };
        assert_eq!(group_display_name(&named), "生产环境");

        let unnamed = GroupSummary {
            id: "g7".into(),
            name: None,
            env_count: 0,
            invalid_count: 1,
        };
        assert_eq!(group_display_name(&unnamed), GROUP_NAME_PLACEHOLDER);
        assert_ne!(group_display_name(&unnamed), "g7", "缺名字时不得拿 id 冒充");

        let blank = GroupSummary {
            id: "g8".into(),
            name: Some("   ".into()),
            env_count: 0,
            invalid_count: 0,
        };
        assert_eq!(
            group_display_name(&blank),
            GROUP_NAME_PLACEHOLDER,
            "空白名字同样要用占位，不是原始空白串"
        );
    }

    #[test]
    fn empty_reasons_have_distinct_text() {
        assert_eq!(empty_state_text(EmptyReason::NoData), "还没有任何环境数据");
        assert_eq!(
            empty_state_text(EmptyReason::FilteredOut),
            "当前筛选下没有可展示的环境"
        );
        assert_ne!(
            empty_state_text(EmptyReason::NoData),
            empty_state_text(EmptyReason::FilteredOut),
            "「原始为空」与「筛选后为空」的文案不得相同"
        );
    }

    #[test]
    fn empty_state_body_carries_the_no_data_reason() {
        let s = EnvGroupState::new();
        assert_eq!(s.status(), EnvStatus::Ready);
        assert_eq!(
            data_panel_body(&s),
            DataPanelBody::Empty(EmptyReason::NoData),
            "空状态必须判定为「还没有任何环境数据」这条关键路径"
        );
    }

    #[test]
    fn failed_load_is_fail_closed_not_empty() {
        let mut s = EnvGroupState::new();
        s.fail("gateway unreachable");
        assert_eq!(
            data_panel_body(&s),
            DataPanelBody::Failed("gateway unreachable".to_owned()),
            "加载失败必须显示错误，不得退化成空列表"
        );
    }
}
