//! 根视图的纯函数与**同步方法**：版式派生、发送回显、等待态解除判据、
//! 审批 / 文件意图的读取侧同步。
//!
//! 这里**不依赖窗口**的全是纯函数；`impl AppShell` 那几段需要 `Context`，
//! 但同样**不产生任何业务语义** —— 只做「权威 → 副本」的下发。

use gpui_kit::*;

use crate::state::file_intent::FileIntent;
use crate::state::session_stream::{MessageRole, SessionMessage, SessionStreamState};
use crate::ui::input_area::OutgoingInput;
use crate::ui::layout::{RegionWidths, RightPanelLayout, RightPanelTab, WorkbenchLayout};
use crate::ui::right_panel_gpui::RightPanelGpui;

use super::prefs_file;
use super::AppShell;

/// 由版式状态算三区宽度（渲染读宽度的**唯一入口**）。
///
/// 版式来自左分隔条持有的 [`WorkbenchLayout`]，本函数不持有任何状态，
/// 因此「渲染读到的宽度」与「分隔条持有的宽度」天然是同一份，不会有第二份副本。
///
/// fail-closed：窗口宽度取不到（NaN / 负值）时按 0 算 —— `regions` 会把会话流
/// 夹到 [`crate::ui::layout::STREAM_MIN_WIDTH`]，边栏 / 右栏取各自默认或当前宽度，三者都**不为 0**。
#[must_use]
pub(super) fn regions_for(layout: &WorkbenchLayout, viewport_width: f32) -> RegionWidths {
    let available = if viewport_width.is_finite() {
        viewport_width.max(0.0)
    } else {
        0.0
    };
    layout.regions(available)
}

/// 把 tab 条 / 右侧面板刚产生的选中态与折叠态**收回**给权威版式。
///
/// 与 [`tab_strip_layout`] 方向相反，两者必须成对使用：
/// - `tab_strip_layout` 是**下发**（权威 → 子视图）；
/// - 本函数是**回收**（子视图 → 权威），在子视图自己 `notify()` 之后、下发之前调用。
///
/// 2026-10-21 Windows 真机实测教训：只做下发不做回收时，tab 条的点击会改它私有的
/// `RightPanelLayout`，紧接着 `sync_layout` 用权威版式里**旧的**选中态覆盖回去，
/// 表现为「点 tab 完全没反应」。当时编译、clippy、289 个单元测试全绿 ——
/// 因为纯函数 `tab_strip_layout` 本身没错，错的是**调用顺序**缺了一环。
///
/// 只回写选中 tab 与折叠态，**不动宽度**（宽度归分隔条管）。
/// 返回是否真的发生了改变，供调用方决定要不要 `notify()`（避免 observe 死循环）。
pub(super) fn adopt_right_panel_selection(
    authority: &mut WorkbenchLayout,
    incoming: &RightPanelLayout,
) -> bool {
    let panel = authority.right_panel_mut();
    let changed = panel.tab() != incoming.tab() || panel.is_collapsed() != incoming.is_collapsed();
    if changed {
        panel.select_tab(incoming.tab());
        if incoming.is_collapsed() {
            panel.collapse();
        } else {
            panel.expand();
        }
    }
    changed
}

/// 渲染右侧面板 / tab 条时应使用的版式：以左分隔条持有的版式为准。
///
/// 回写到左侧时保留 `current` 的宽度（宽度归分隔条管，tab 条只管选中 / 折叠），
/// 回写右侧时整份以左侧为准（选中 tab 随即生效）。
#[must_use]
pub(super) fn tab_strip_layout(
    authority: &WorkbenchLayout,
    current: &RightPanelLayout,
) -> RightPanelLayout {
    let mut layout = current.clone();
    layout.select_tab(authority.right_panel().tab());
    if authority.right_panel().is_collapsed() {
        layout.collapse();
    } else {
        layout.expand();
    }
    layout
}

/// 把一条已发出的输入变成会话流里的**本地回显**消息。
///
/// - 角色固定为 [`MessageRole::User`]，`tool` 为 `None`（用户消息不关联工具）。
/// - `ts_ms` 不取系统时钟（会让测试不确定）：取「新序号减一」，只保证单调，
///   **不是真实时间**。真实时间戳由运行时下发的事件携带。
/// - `seq` 由调用方用 [`next_echo_seq`] 从会话流序号派生，保证严格递增。
#[must_use]
pub(super) fn echo_message(next_seq: u64, out: &OutgoingInput) -> SessionMessage {
    SessionMessage {
        role: MessageRole::User,
        content: out.text.clone(),
        tool: None,
        ts_ms: next_seq.saturating_sub(1) as i64,
    }
}

/// 回显消息应使用的序号：会话流当前最大序号 + 1。
#[must_use]
pub(super) fn next_echo_seq(state: &SessionStreamState) -> u64 {
    state.last_seq() + 1
}

/// 「等待运行时确认」是否可以解除：存在已发出但未确认的消息，且会话流最新一条
/// 不是 `User`（即已有回应抵达）。
///
/// 会话流为空时**不解除**（没有回应）。回显本身是 `User` 消息，因此回显不会
/// 误触发解除 —— 必须等到非用户消息推进到末尾。
#[must_use]
pub(super) fn should_finish_send(pending: Option<u64>, state: &SessionStreamState) -> bool {
    if pending.is_none() {
        return false;
    }
    match state.messages().last() {
        Some(message) => message.role != MessageRole::User,
        None => false,
    }
}

/// 审批列表是否已与来源一致（只做读取侧同步的比较）。
///
/// 缺口 7 的收敛：根视图是唯一写入者，右栏只是读取侧副本；每次会话流变化时
/// 用本判据决定要不要把审批列表整体拷给右栏，避免 observe → notify → observe 循环。
#[must_use]
pub(super) fn approvals_in_sync(authority: &SessionStreamState, copy: &SessionStreamState) -> bool {
    authority.approvals() == copy.approvals()
}

/// 把跳转意图应用到右侧面板：写入意图 + 激活「文件预览」tab。
///
/// 返回是否真的发生了变化（供调用方决定要不要 `notify()`）。
///
/// **为什么要激活 tab**：对齐 A4「点文件跳预览」——点了文件必须有反应，
/// 否则意图白产生（面板停在「任务清单」，用户看不到任何变化）。
/// 选中态的**权威**仍是左分隔条：本函数只改面板自己的副本，随后面板的
/// `notify()` 会经 `adopt_right_panel` 把选中态**回收**给权威版式。
///
/// 没有意图（`None`）时**不动 tab**：只清掉旧意图。
#[must_use]
pub(super) fn apply_preview_intent(panel: &mut RightPanelGpui, intent: Option<FileIntent>) -> bool {
    if panel.preview_intent() == intent.as_ref() {
        return false;
    }
    let has_intent = intent.is_some();
    panel.set_preview_intent(intent);
    if has_intent {
        // `select_tab` 同时展开折叠态的窄条（窄条只用于恢复入口）。
        panel.layout_mut().select_tab(RightPanelTab::Preview);
    }
    true
}

impl AppShell {
    /// 把侧栏选中的导航入口同步到主区视图。
    ///
    /// 侧栏点「频道」/「智能体管理」后，主区据此切换显示哪一块
    /// （对齐 A15：入口不能是死按钮）。
    ///
    /// 没变化就不 `notify()`：observe → notify → observe 会绕圈。
    ///
    /// ⛔ 只做界面路由：频道 / 智能体管理的数据源在运行时侧，本仓不实现。
    pub(super) fn sync_main_view(&mut self, cx: &mut Context<Self>) {
        let selected = self.sidebar.read(cx).state().selected_nav();
        let view = crate::ui::main_view::resolve(selected);
        if self.main_view != view {
            self.main_view = view;
            cx.notify();
        }
    }

    /// 审批读取侧同步：根视图（经由会话流）是唯一写入者，右栏只是副本。
    pub(super) fn sync_approvals(&mut self, cx: &mut Context<Self>) {
        let authority = self.session.read(cx).state().clone();
        let right_panel = self.right_panel.clone();
        right_panel.update(cx, |panel, cx| {
            if !approvals_in_sync(&authority, panel.session()) {
                let base = authority.clone();
                let session = panel.session_mut();
                session.clear();
                for approval in base.approvals() {
                    session.push_approval(approval.clone());
                }
                cx.notify();
            }
        });
    }

    /// 文件跳转意图同步：会话流是唯一产生者，右栏预览 tab 只是**消费者**。
    ///
    /// 右栏不产生意图，也不判定文件是否存在。
    ///
    /// ⛔ 预览器本体未实现（P3，见 `docs/ui-parity-spec.md` A14）：这里只把
    /// 「要跳哪个文件」下发给右栏，**不读文件、不落盘**。
    pub(super) fn sync_file_intent(&mut self, cx: &mut Context<Self>) {
        let intent = self.session.read(cx).file_intent().cloned();
        let right_panel = self.right_panel.clone();
        right_panel.update(cx, |panel, cx| {
            if apply_preview_intent(panel, intent) {
                cx.notify();
            }
        });
    }

    /// tab 条刚被点击 ⇒ 把它的选中态 / 折叠态**回写**给权威版式（左分隔条）。
    ///
    /// 为什么必须有这一步：`sync_layout` 是「以左分隔条为唯一来源**下发**」，
    /// 而 tab 条自己的点击改的是它私有的 `RightPanelLayout`。若只观察 tab 条就直接
    /// `sync_layout`，下发会用权威版式里**旧的** tab 选中态把刚点出来的新状态覆盖掉
    /// —— 表现就是「点 tab 完全没反应」（2026-10-21 Windows 真机实测到的缺陷，
    /// 编译与单元测试都发现不了，因为纯函数 `tab_strip_layout` 本身是对的）。
    ///
    /// 只回写 tab 选中态与折叠态，**不动宽度**：宽度归分隔条管。
    pub(super) fn adopt_tab_strip(&mut self, cx: &mut Context<Self>) {
        let incoming = self.tab_strip.read(cx).layout().clone();
        self.left_divider.update(cx, |divider, cx| {
            if adopt_right_panel_selection(divider.layout_mut(), &incoming) {
                cx.notify();
            }
        });
    }

    /// 右分隔条刚被拖动 ⇒ 把它算出的右栏宽度**回写**给权威版式（左分隔条）。
    ///
    /// 与 [`Self::adopt_tab_strip`] 同一类修复：`sync_layout` 只做「权威 → 子视图」
    /// 下发，而右分隔条的拖拽改的是它私有副本里的 `right_panel` 宽度。
    /// 若只下发不回收，下一次同步会用权威里的旧宽度把拖动结果覆盖回去
    /// ⇒ 表现成「右分隔条被锁死、拖不动」。2026-10-21 Windows 真机实测发现。
    pub(super) fn adopt_right_divider(&mut self, cx: &mut Context<Self>) {
        let incoming_width = self.right_divider.read(cx).layout().right_panel().width();
        self.left_divider.update(cx, |divider, cx| {
            let panel = divider.layout_mut().right_panel_mut();
            if panel.width() != incoming_width {
                panel.set_width(incoming_width);
                cx.notify();
            }
        });
    }

    /// 右侧面板自己的「收起 / 展开」按钮 ⇒ 把折叠态**回写**给权威版式（左分隔条）。
    ///
    /// 与 [`Self::adopt_tab_strip`] 同一类问题：`sync_layout` 只做「权威 → 子视图」
    /// 下发，而面板的折叠按钮改的是它私有副本里的 `collapsed` 标志。
    /// 不回收的话，下一次下发会把折叠态覆盖回展开 ⇒ 表现成「点收起，折叠态弹回来」。
    ///
    /// 回收逻辑复用 [`adopt_right_panel_selection`]（与 tab 条共用同一段纯函数），
    /// **不另写一份**折叠 / 选中的判定。
    pub(super) fn adopt_right_panel(&mut self, cx: &mut Context<Self>) {
        let incoming = self.right_panel.read(cx).layout().clone();
        self.left_divider.update(cx, |divider, cx| {
            if adopt_right_panel_selection(divider.layout_mut(), &incoming) {
                cx.notify();
            }
        });
    }

    /// 把当前版式**写回**偏好文件（尽力而为）。
    ///
    /// - **拖拽过程中不写**：拖拽时每个鼠标移动事件都会触发一次同步，
    ///   逐帧写磁盘既无必要又可能把文件写坏。松手那一次同步才落盘
    ///   （=「松手才提交」的语义）。
    /// - 只在版式**真的变了**时写（`last_written` 去重）。
    /// - 写失败**只记日志**，绝不影响交互（fail-closed）。
    pub(super) fn persist_layout(&mut self, cx: &mut Context<Self>) {
        if self.left_divider.read(cx).is_dragging() || self.right_divider.read(cx).is_dragging() {
            return;
        }
        let current = self.left_divider.read(cx).layout().export_prefs();
        if self.last_written == Some(current) {
            return;
        }
        match prefs_file::write_layout(current) {
            Ok(()) => self.last_written = Some(current),
            Err(reason) => {
                eprintln!("[lengsuo-desktop] 版式偏好未落盘：{reason}");
            }
        }
    }

    /// 把版式（左分隔条 —— 唯一来源）下发到右分隔条与右侧面板。
    ///
    /// - 右分隔条整份覆盖：它只负责拖拽，不持有独立的版式主张。
    /// - 右侧面板**就地**用 [`crate::ui::right_panel_gpui::RightPanelGpui::layout_mut`]
    ///   改选中 tab / 折叠态，**不重建 `Entity`**（重建会丢面板里审批卡片的本地态）；
    ///   宽度归分隔条管，这里不动它的宽度字段。
    ///
    /// 回写前先比较，没变化就不 `notify()`，避免 observe → notify → observe 的循环。
    ///
    /// **最后必须 `cx.notify()` 自己**：三区宽度是根视图渲染时从左分隔条读回的，
    /// 子实体的 `notify()` 只会重绘子视图本身；没有这一步，拖动分隔条后
    /// 根视图不会重排三区 —— 表现成「分隔条跟手走、面板宽度不动」（2026-10-21
    /// Windows 真机实测：分隔条元素位置变了，侧栏右边界纹丝不动）。
    /// GPUI 对重复 `notify` 幂等，这里无条件调用是安全的。
    pub(super) fn sync_layout(&mut self, cx: &mut Context<Self>) {
        let authority = self.left_divider.read(cx).layout().clone();

        // 右分隔条**只下发宽度**，不整份覆盖：它自己也是拖拽的输入源，
        // 整份覆盖会把它刚拖出来的结果冲掉（同 `adopt_right_divider` 的注释）。
        let right_divider = self.right_divider.clone();
        let authority_width = authority.right_panel().width();
        right_divider.update(cx, |divider, cx| {
            let panel = divider.layout_mut().right_panel_mut();
            if panel.width() != authority_width {
                panel.set_width(authority_width);
                cx.notify();
            }
        });

        let right_panel = self.right_panel.clone();
        right_panel.update(cx, |panel, cx| {
            let synced = tab_strip_layout(&authority, panel.layout());
            if panel.layout() != &synced {
                *panel.layout_mut() = synced;
                cx.notify();
            }
        });

        let tab_strip = self.tab_strip.clone();
        tab_strip.update(cx, |strip, cx| {
            let synced = tab_strip_layout(&authority, strip.layout());
            if strip.layout() != &synced {
                *strip.layout_mut() = synced;
                cx.notify();
            }
        });

        // 版式变了 ⇒ 写回偏好文件（e22）。写失败只记日志，不影响交互。
        self.persist_layout(cx);

        cx.notify();
    }
}
