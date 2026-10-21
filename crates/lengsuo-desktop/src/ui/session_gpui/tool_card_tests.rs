//! 工具卡片的纯逻辑测试（不开窗口）。
//!
//! 覆盖参考客户端的两条验收项：
//! - 「工具卡片五类真实触发渲染正确」⇒ `the_five_kinds_render_five_distinct_cards`；
//! - 「AI 回复无工具英文名」⇒ `no_tool_english_name_is_visible_anywhere`。
//!
//! 显式导入（不用 `use super::*`）：父模块的 `use gpui_kit::*` 会把 GPUI 自己的
//! `test` 属性宏带进作用域，遮蔽标准库的 `#[test]`，导致测试无法编译。

#[cfg(test)]
mod tests {
    use super::super::tool_fold::{
        category_of, describe, tool_call_details, visible_text, MAX_SUMMARY_CHARS,
    };
    use super::super::{SessionStreamGpui, SessionUiState};
    use crate::state::session_demo::demo_session;
    use crate::state::session_stream::MessageRole;
    use crate::ui::session_render::{self, Block};
    use crate::ui::tool_card::ToolCard;

    /// 演示会话里五类工具调用对应的**人话类别词**。
    const FIVE_HUMAN: [&str; 5] = ["读取", "创建", "修改", "执行命令", "搜索"];

    /// 取演示会话里所有工具卡片的渲染描述（按出现顺序）。
    fn demo_cards() -> Vec<crate::ui::session_gpui::tool_fold::ToolCardView> {
        let state = demo_session();
        let details = tool_call_details(state.messages());
        state
            .messages()
            .iter()
            .filter(|m| m.role == MessageRole::ToolCall)
            .zip(details.iter())
            .map(|(msg, detail)| {
                let card = ToolCard::from_message(msg);
                describe(&card, Some(detail), true)
            })
            .collect()
    }

    #[test]
    fn the_five_kinds_render_five_distinct_cards() {
        let cards = demo_cards();
        assert_eq!(cards.len(), 5, "五类真实触发一个都不能少");

        for (card, human) in cards.iter().zip(FIVE_HUMAN) {
            assert!(
                card.title.starts_with(human),
                "卡片标题应以人话词 {human} 开头，实际：{}",
                card.title
            );
            assert!(!card.icon.is_empty(), "每张卡片都要有图标");
            assert!(!card.summary.is_empty(), "每张卡片都要有参数摘要");
        }

        // 五类互不相同（不重样）。
        let mut titles: Vec<&str> = cards.iter().map(|c| c.title.as_str()).collect();
        titles.sort_unstable();
        let before = titles.len();
        titles.dedup();
        assert_eq!(titles.len(), before, "五类卡片的标题必须各不相同");
    }

    #[test]
    fn no_tool_english_name_is_visible_anywhere() {
        // 反例：遍历五类工具，断言界面上任何可见文字都不含英文工具名。
        let english = ["read", "write", "edit", "bash", "grep"];
        for card in demo_cards() {
            let text = visible_text(&card).to_lowercase();
            for name in english {
                assert!(
                    !text.contains(name),
                    "界面出现工具英文名 {name}，实际可见文字：{text}"
                );
            }
        }
    }

    #[test]
    fn the_five_cards_have_five_distinct_icons() {
        let cards = demo_cards();
        let mut icons: Vec<&str> = cards.iter().map(|c| c.icon).collect();
        icons.sort_unstable();
        let before = icons.len();
        icons.dedup();
        assert_eq!(icons.len(), before, "五类卡片应有五个可区分的图标");
    }

    #[test]
    fn the_demo_session_renders_a_tool_card_block_for_every_call() {
        let state = demo_session();
        let blocks = session_render::render(&state);
        let tool_blocks = blocks
            .iter()
            .filter(|b| matches!(b, Block::ToolCall(_)))
            .count();
        assert_eq!(tool_blocks, 5, "五条工具调用消息 ⇒ 五个工具卡块");
    }

    #[test]
    fn the_fold_state_is_remembered_per_message_id() {
        let state = demo_session();
        let details = tool_call_details(state.messages());
        let mut ui = SessionUiState::default();

        // 默认全部折叠。
        for detail in &details {
            assert!(ui.is_tool_folded(&detail.id), "默认折叠");
        }

        // 展开第 2 张（创建）——重绘后仍是展开态，其余不受影响。
        ui.toggle_tool_fold(&details[1].id);
        assert!(!ui.is_tool_folded(&details[1].id));
        assert!(ui.is_tool_folded(&details[0].id), "别的卡片不受影响");
        assert!(ui.is_tool_folded(&details[2].id), "别的卡片不受影响");

        // 再点一次 ⇒ 收起。
        ui.toggle_tool_fold(&details[1].id);
        assert!(ui.is_tool_folded(&details[1].id));
    }

    #[test]
    fn the_view_level_toggle_also_remembers_per_message_id() {
        let state = demo_session();
        let details = tool_call_details(state.messages());
        let mut view = SessionStreamGpui::new(state);

        view.toggle_tool_fold(&details[0].id);
        assert!(!view.ui_state().is_tool_folded(&details[0].id));
        assert!(
            view.ui_state().is_tool_folded(&details[1].id),
            "别的卡片不受影响"
        );
    }

    #[test]
    fn the_folded_summary_is_capped_and_the_expanded_body_is_not() {
        // 造一条**超长参数**的工具调用消息。
        let long = "z".repeat(200);
        let mut state = crate::state::session_stream::SessionStreamState::new();
        assert!(state.push_message(
            1,
            crate::state::session_stream::SessionMessage {
                role: MessageRole::ToolCall,
                content: long.clone(),
                tool: Some("Read".into()),
                ts_ms: 0,
            }
        ));
        let details = tool_call_details(state.messages());
        let card = ToolCard::from_message(&state.messages()[0]);

        let folded = describe(&card, Some(&details[0]), true);
        assert!(
            folded.summary.chars().count() <= MAX_SUMMARY_CHARS,
            "折叠态摘要 ≤ {MAX_SUMMARY_CHARS} 字，实际 {}",
            folded.summary.chars().count()
        );
        assert!(folded.summary.ends_with('…'), "折叠态超长要省略号");

        let expanded = describe(&card, Some(&details[0]), false);
        assert_eq!(expanded.args, long, "展开态显示完整参数，不截断");
        assert_eq!(expanded.args.chars().count(), 200);
    }

    #[test]
    fn the_category_word_never_leaks_the_english_name() {
        assert_eq!(category_of(Some("Read")), "读取");
        assert_eq!(category_of(Some("Bash")), "执行命令");
        assert_eq!(category_of(None), "执行操作");
    }

    #[test]
    fn every_tool_card_renders_an_element_without_panicking() {
        // 静态上下文（无交互宿主）：七种块分支都必须能出元素。
        let theme = gpui_kit::component::Theme::default();
        let ctx = super::super::BlockCtx::passive(&theme);
        let state = demo_session();
        for block in session_render::render(&state) {
            let _ = super::super::render_block_with(&block, &ctx);
        }
    }

    #[test]
    fn the_tool_card_element_id_is_unique_per_message() {
        // GPUI 的 `.id()` 必须同屏唯一：两张同屏卡片不得撞 id。
        let state = demo_session();
        let details = tool_call_details(state.messages());
        let mut ids: Vec<String> = details
            .iter()
            .map(|d| format!("tool-card-{}", d.id))
            .collect();
        let before = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), before, "同屏工具卡片的元素 id 必须唯一");
    }
}
