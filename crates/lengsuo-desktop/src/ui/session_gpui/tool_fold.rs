//! 工具卡片的**纯逻辑**部分：消息 id、参数摘要、折叠态记忆、图标映射。
//!
//! 与 [`super::tool_card_view`]（GPUI 渲染）分工：本文件**不依赖 GPUI**，
//! 可在无窗口环境测；渲染层只把它产出的描述画成元素，**不重新实现**任何判断。
//!
//! 硬约束：
//! - **R13-1：工具名对用户不可见。** 工具名只用于「选图标」与「算 id」，
//!   任何会显示到界面上的文案都取自 [`crate::ui::tool_card::ToolCard`] 的人话标题。
//! - 折叠态**只在内存**（挂 [`super::SessionUiState`]），**不落盘**。
//! - 摘要行最多 [`MAX_SUMMARY_CHARS`] 个字符（**含**省略号）。

use crate::state::session_stream::{MessageRole, SessionMessage};
use crate::theme::tool_titles::short_summary;
use crate::ui::tool_card::ToolCard;
use std::collections::HashMap;

/// 折叠态参数摘要的最大字符数（**含**省略号）。
///
/// 描述层的 [`crate::ui::tool_card::ToolCard::summary`] 允许「60 字正文 + 省略号」
/// 共 61 字；界面这一行必须 ≤ 60 字，故正文最多留 59 字给省略号。
pub const MAX_SUMMARY_CHARS: usize = 60;

/// 一条工具调用消息的接线细节（**内部用**，不直接显示）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallDetail {
    /// 稳定的消息 id：折叠态的键（重绘后不丢）。
    pub id: String,
    /// 工具名（**只用于选图标**；⛔ 不显示）。
    pub tool: Option<String>,
    /// 完整参数原文（展开态显示，**不截断**）。
    pub args: String,
}

/// 工具卡片的渲染描述（纯数据）：图标 + 人话标题 + 参数摘要 + 折叠态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCardView {
    /// 文字 / unicode 图标（⛔ 不引图标依赖）。
    pub icon: &'static str,
    /// 人话标题（**类别词**，如「读取」；⛔ 不含工具英文名）。
    pub title: String,
    /// 折叠态显示的参数摘要（≤ [`MAX_SUMMARY_CHARS`] 字）。
    pub summary: String,
    /// 展开态显示的完整参数（**不截断**）。
    pub args: String,
    /// 是否折叠。
    pub folded: bool,
}

/// 取出消息流里所有工具调用消息的接线细节（**按出现顺序**）。
///
/// 顺序与 `session_render::render` 产出的 `Block::ToolCall` 块**一一对应**
/// （每条 `ToolCall` 消息恰好产出一个工具卡块），接线层据此把 id 与参数挂到块上。
#[must_use]
pub fn tool_call_details(messages: &[SessionMessage]) -> Vec<ToolCallDetail> {
    let mut seen: HashMap<u32, usize> = HashMap::new();
    let mut out = Vec::new();
    for msg in messages.iter().filter(|m| m.role == MessageRole::ToolCall) {
        let hash = fingerprint(msg);
        let occurrence = seen.entry(hash).or_insert(0);
        let id = tool_call_id(hash, *occurrence);
        *occurrence += 1;
        out.push(ToolCallDetail {
            id,
            tool: msg.tool.clone(),
            args: msg.content.clone(),
        });
    }
    out
}

/// 工具调用消息的稳定 id：`tc-{指纹}-{同指纹出现次数}`。
///
/// - 指纹（FNV-1a，工具名 + 参数正文）保证**重绘后不变**、且**不受位置影响**
///   （在前面插入新消息不会让已有卡片的 id 变掉 ⇒ 折叠态不丢）；
/// - 同指纹计数保证**同屏唯一**（两条一模一样的工具调用不会撞 id）。
#[must_use]
pub fn tool_call_id(hash: u32, occurrence: usize) -> String {
    format!("tc-{hash:08x}-{occurrence}")
}

/// 消息指纹：工具名 + 参数正文。
fn fingerprint(msg: &SessionMessage) -> u32 {
    let mut hash: u32 = 0x811c_9dc5; // FNV-1a 32 位偏移基数
    const PRIME: u32 = 0x0100_0193;
    for part in [
        msg.tool.as_deref().unwrap_or_default(),
        msg.content.as_str(),
    ] {
        for byte in part.as_bytes() {
            hash ^= u32::from(*byte);
            hash = hash.wrapping_mul(PRIME);
        }
        hash ^= 0xff; // 分隔两个字段
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

/// 折叠态显示的参数摘要：最长 [`MAX_SUMMARY_CHARS`] 字（**含**省略号）。
///
/// 短参数原样返回；超长则截断并加 `…`。
#[must_use]
pub fn fold_summary(text: &str) -> String {
    short_summary(text.trim(), MAX_SUMMARY_CHARS - 1)
}

/// 标题里的**短**参数（≤ [`MAX_TITLE_ARG_CHARS`] 字，含省略号）。
#[must_use]
pub fn short_title_arg(text: &str) -> String {
    short_summary(text.trim(), MAX_TITLE_ARG_CHARS - 1)
}

/// 工具类别 → 文字 / unicode 图标。
///
/// ⛔ **不引图标依赖**（`IconName` → `AnyElement` 缺 `From` 实现，见速查表 §7）；
/// 这里用 unicode 符号表达类别，**不显示**工具英文名。
#[must_use]
pub fn tool_icon(tool: Option<&str>) -> &'static str {
    match tool.unwrap_or_default() {
        "Read" => "▤",
        "Write" => "✚",
        "Edit" => "✎",
        "Bash" => "▸",
        "Grep" => "◎",
        "Glob" => "◇",
        "WebFetch" => "↗",
        "WebSearch" => "○",
        _ => "•",
    }
}

/// 标题里参数的最大字符数（**短**形式；参考客户端的 `short(path)`）。
///
/// **含**省略号；完整摘要另有其位（第二行，≤ [`MAX_SUMMARY_CHARS`] 字）。
pub const MAX_TITLE_ARG_CHARS: usize = 30;

/// 工具卡片的渲染描述（纯数据）。
///
/// - 标题走 [`crate::theme::tool_titles::tool_title`]：**人话类别词 + 短参数**
///   （如「读取 src/main.rs」），**不含**工具英文名；
/// - 第二行是参数摘要（折叠态 ≤ [`MAX_SUMMARY_CHARS`] 字）；展开态显示**完整**参数；
/// - 没有接线细节时（静态预览）退回描述层自己的标题 / 摘要。
#[must_use]
pub fn describe(card: &ToolCard, detail: Option<&ToolCallDetail>, folded: bool) -> ToolCardView {
    let tool = detail.and_then(|d| d.tool.as_deref());
    let args = detail.map_or_else(|| card.summary.clone(), |d| d.args.clone());
    ToolCardView {
        icon: tool_icon(tool),
        title: title_of(card, tool, &args),
        summary: fold_summary(&args),
        args,
        folded,
    }
}

/// 卡片标题：人话类别词 + 短参数。
///
/// 走已有的映射表（[`crate::theme::tool_titles::tool_title`]），**不另造一套**；
/// 未知工具由映射层兜底为「执行操作」，不暴露内部名。
fn title_of(card: &ToolCard, tool: Option<&str>, args: &str) -> String {
    match tool {
        Some(tool) => {
            let short = short_title_arg(args);
            crate::theme::tool_titles::tool_title(tool, Some(&short))
        }
        None => card.title.clone(),
    }
}

/// 人话**类别词**（如「读取」「执行命令」）。
///
/// 只给「徽标 / 分组」这类只需要类别的场景用；⛔ **不得**单独作为卡片标题
/// （那会丢掉参数信息），也**不得**显示工具英文名。
#[must_use]
pub fn category_of(tool: Option<&str>) -> String {
    crate::theme::tool_titles::title_of(tool.unwrap_or_default())
}

/// 界面上一张工具卡片的**全部可见文字**（供 R13-1 反例测试扫描）。
#[must_use]
pub fn visible_text(view: &ToolCardView) -> String {
    format!("{} {} {}", view.icon, view.title, view.summary)
}

impl super::SessionUiState {
    /// 某张工具卡片是否折叠（默认**折叠**）。
    ///
    /// 键是**消息 id**（见 [`tool_call_id`]）：重绘后不丢，换一张卡片不受影响。
    #[must_use]
    pub fn is_tool_folded(&self, id: &str) -> bool {
        self.tool_folded.get(id).copied().unwrap_or(true)
    }

    /// 切换某张工具卡片的折叠态（按消息 id 记忆；**不落盘**）。
    pub fn toggle_tool_fold(&mut self, id: &str) {
        let next = !self.is_tool_folded(id);
        self.tool_folded.insert(id.to_owned(), next);
    }
}
