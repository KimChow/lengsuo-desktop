//! A 段：会话流纯函数 `render_block` —— 四种块分支各画一遍。

use super::{assert_rendered, draw_probe};
use gpui_kit::IntoElement as _;

use crate::ui::session_gpui;
use crate::ui::session_render::{Align, Block, FileItem, STALE_NOTICE};

/// 四种块分支各画一遍：用户气泡 / 助手正文 / 文件网格 / 断档提示。
#[test]
fn session_blocks_all_render_content() {
    let blocks = [
        Block::UserBubble {
            text: "你好".to_owned(),
            align: Align::Right,
        },
        Block::AssistantText {
            text: "这是助手正文。".to_owned(),
            align: Align::Left,
        },
        Block::FilesGrid {
            files: vec![
                FileItem::new("src/main.rs"),
                FileItem::new("docs/readme.md"),
            ],
        },
        Block::StaleNotice { text: STALE_NOTICE },
    ];

    for block in &blocks {
        let bounds = draw_probe(|_ctx| session_gpui::render_block(block).into_any_element());
        assert_rendered(bounds, "会话块");
    }
}

/// 文本真的被排版了：更长的文本必须撑出更宽的元素。
///
/// 只断言「不 panic」抓不到「文本没被排版」（例如字体度量返回 0）；这条抓得到。
#[test]
fn session_longer_text_lays_out_wider() {
    let short = draw_probe(|_ctx| {
        session_gpui::render_block(&Block::UserBubble {
            text: "短".to_owned(),
            align: Align::Right,
        })
        .into_any_element()
    });
    let long = draw_probe(|_ctx| {
        session_gpui::render_block(&Block::UserBubble {
            text: "这是一条明显更长的用户消息".repeat(30),
            align: Align::Right,
        })
        .into_any_element()
    });

    let short = assert_rendered(short, "短文本气泡");
    let long = assert_rendered(long, "长文本气泡");
    assert!(
        long.size.width > short.size.width,
        "长文本没有撑出更宽的元素 ⇒ 文本没被真正排版（long={:?} short={:?}）",
        long.size.width,
        short.size.width
    );
}
