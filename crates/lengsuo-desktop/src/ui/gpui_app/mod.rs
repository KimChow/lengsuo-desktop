//! 应用入口组装：把 e1–e7 的接线层拼成一个可运行的工作台。
//!
//! 布局（标题栏 + 三区 + 输入区）：
//!
//! ```text
//! ┌──────────────────────────────────────────────┐
//! │ 标题栏（WINDOW_TITLE）                        │
//! ├──────────┬──────────────────────┬────────────┤
//! │ Sidebar  │   SessionStream      │ RightPanel │
//! │          │                      │            │
//! │          ├──────────────────────┴────────────┤
//! │          │  InputAreaGpui（中右两列下方）       │
//! └──────────┴───────────────────────────────────┘
//! ```
//!
//! 分层约定（与 [`crate::ui::env_gpui`] 黄金范例一致）：
//! - 根视图**不持有任何业务语义**：数据全部来自 [`crate::state`] 的 demo 构造，
//!   子视图各自负责「描述 → 元素」的映射。
//! - **版式唯一持有者是左分隔条**（[`DividerGpui`]）：三区宽度每次渲染都经
//!   [`sync::regions_for`] 从它持有的 [`WorkbenchLayout`] 读回，**不另存副本**；
//!   右侧 tab 条（[`RightPanelTabGpui`]）从同一份版式派生选中态，点击后经
//!   [`AppShell::sync_layout`] 就地写给右侧面板与右分隔条。
//! - 命令面板浮在最上层，快捷键经 `palette_gpui::hotkey_from_keystroke` 转换后
//!   交给 [`crate::ui::command_palette::handle_hotkey`]。
//! - 环境 / 组织视图本批**不进主界面**，但通过 [`secondary_views`] 编译进二进制。
//!
//! e14 发送回显（**界面层本地回显，不投递**）：
//! - 输入区发送时把 `OutgoingInput` 入自己的 `outbox` 并 `notify()`；根视图**只读消费**
//!   输入区自己的 `outbox`（[`InputAreaGpui::take_pending`]），**不重新实现队列**。
//!   （未使用 `set_on_send`：它的回调拿不到 `cx`，无法写会话流。）
//! - 回显发生在 [`AppShell::drain_outbox`]（由 observe 输入区 + 每次 `render` 触发），
//!   经 [`sync::echo_message`] / [`sync::next_echo_seq`]
//!   变成一条 `User` 消息推进会话流。这一步**不写网络、不产生任何投递**，
//!   文案上只能称「本地回显 / 待运行时确认」。
//! - 回显时 **`in_flight` 保持 true**（描述层「等待确认期间禁重发」）。只有会话流
//!   出现非 `User` 的回应（[`sync::should_finish_send`]）时，才由
//!   [`AppShell::settle_pending`] 调 [`InputAreaGpui::finish_send`] 解除等待。
//!
//! 文件划分：`demo.rs`（演示数据）、`launch.rs`（窗口启动）、`sync.rs`（纯函数）、
//! `tests.rs`（单元测试）。
#![cfg(feature = "ui")]

mod demo;
mod launch;
pub(crate) mod prefs_file;
mod sync;
#[cfg(test)]
mod tests;

pub use demo::{demo_session, secondary_views};
pub use launch::{launch, LaunchError, HOTKEY_HINT, NO_DISPLAY_MESSAGE, WINDOW_TITLE};
use sync::{
    adopt_right_panel_selection, approvals_in_sync, echo_message, next_echo_seq, regions_for,
    should_finish_send,
};

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

use crate::state::hotkeys::HotkeyRegistry;
use crate::ui::command_palette::{self, PaletteAction};
use crate::ui::input_gpui::InputAreaGpui;
use crate::ui::layout::{WorkbenchLayout, STREAM_MIN_WIDTH};
use crate::ui::layout_gpui::{DividerGpui, DividerSide, RightPanelTabGpui};
use crate::ui::palette_gpui::{self, CommandPaletteGpui};
use crate::ui::right_panel_gpui::RightPanelGpui;
use crate::ui::session_gpui::SessionStreamGpui;
use crate::ui::settings_gpui::SettingsGpui;

/// 打开设置窗口的命令 id（命令表 `theme::palette` 里的 `app.settings`）。
const SETTINGS_COMMAND_ID: &str = "app.settings";
/// 设置窗口的标题栏文案。
const SETTINGS_WINDOW_TITLE: &str = "设置";
use crate::ui::sidebar_gpui::{self, SidebarViewGpui};

use demo::default_registry;
use sync::tab_strip_layout;

/// 工作台根视图：三区 + 输入区 + 命令面板浮层。
///
/// 版式**不在本结构里另存**：唯一持有者是 [`Self::left_divider`]，
/// 渲染时用 [`regions_for`] 读回；右分隔条与右侧面板只是它的下游副本。
struct AppShell {
    sidebar: Entity<SidebarViewGpui>,
    session: Entity<SessionStreamGpui>,
    right_panel: Entity<RightPanelGpui>,
    input: Entity<InputAreaGpui>,
    palette: Entity<CommandPaletteGpui>,
    /// 左侧分隔条：三区版式的**唯一持有者**（拖拽结果也落在它内部）。
    left_divider: Entity<DividerGpui>,
    /// 右侧分隔条：宽度由 [`AppShell::sync_layout`] 从左侧回写。
    right_divider: Entity<DividerGpui>,
    /// 右侧面板 tab 条：点 tab 改自身选中态，再经 [`AppShell::sync_layout`] 下发。
    tab_strip: Entity<RightPanelTabGpui>,
    /// 子视图 `notify()` 时回调：把版式变化下发到右侧面板与右分隔条。
    _layout_sync: Vec<Subscription>,
    /// 会话流 / 输入区变化的订阅（回显、解除等待态、审批读取侧同步）。
    _session_sync: Vec<Subscription>,
    /// 已发出、尚未收到运行时确认的回显序号；`Some` 时才允许解除等待态。
    pending_seq: Option<u64>,
    /// 与命令面板共用同一套内置快捷键表（按 id 派发）。
    registry: HotkeyRegistry,
    /// 根视图焦点：全局按键先到这里。
    focus: FocusHandle,
}

/// 打开设置窗口（`app.settings` 命令派发时调用）。
///
/// ⛔ 失败不 panic：开窗失败（无显示服务器等）时静默返回，主界面不受影响。
fn open_settings_window(cx: &mut App) {
    let options = WindowOptions {
        titlebar: Some(TitlebarOptions {
            title: Some(SETTINGS_WINDOW_TITLE.into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    // `gpui_kit::open_window` 返回 `Result` ⇒ 失败就放弃这次开窗。
    let _ = gpui_kit::open_window(options, cx, |_window, cx| cx.new(SettingsGpui::new));
}

impl AppShell {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let layout = WorkbenchLayout::new();
        let session_state = demo_session();

        let sidebar = cx.new(|_| SidebarViewGpui::new(sidebar_gpui::demo_state()));
        let session = cx.new(|_| SessionStreamGpui::new(session_state.clone()));
        let right_panel =
            cx.new(|_| RightPanelGpui::from_state(layout.right_panel(), &session_state));
        let input = cx.new(|cx| InputAreaGpui::new(window, cx));
        let palette = cx.new(CommandPaletteGpui::new);

        // 版式：左分隔条持有唯一一份；右分隔条与 tab 条各持副本，初始值一致。
        let left_divider = cx.new(|_| DividerGpui::with_layout(DividerSide::Left, layout.clone()));
        let right_divider =
            cx.new(|_| DividerGpui::with_layout(DividerSide::Right, layout.clone()));
        let tab_strip = cx.new(|_| RightPanelTabGpui::from_layout(layout.right_panel()));

        // 任一分隔条或 tab 条 `notify()` ⇒ 把左侧（唯一来源）的版式下发到下游。
        // 分隔条自身的 `notify()` 只重绘自己；根视图的三区宽度在渲染时读回，
        // 但右分隔条与右侧面板需要被显式通知才会跟着变。
        let _layout_sync = vec![
            cx.observe(&left_divider, |shell: &mut Self, _, cx| {
                shell.sync_layout(cx)
            }),
            cx.observe(&right_divider, |shell: &mut Self, _, cx| {
                shell.adopt_right_divider(cx);
                shell.sync_layout(cx);
            }),
            cx.observe(&tab_strip, |shell: &mut Self, _, cx| {
                shell.adopt_tab_strip(cx);
                shell.sync_layout(cx);
            }),
        ];

        // 会话流推进 ⇒ 解除等待态 + 审批读取侧同步。
        // 输入区自己 `notify()`（发送后）⇒ 消费它排队的输入做本地回显。
        let _session_sync = vec![
            cx.observe(&session, |shell: &mut Self, _, cx| {
                shell.settle_pending(cx);
                shell.sync_approvals(cx);
            }),
            cx.observe(&input, |shell: &mut Self, _, cx| shell.drain_outbox(cx)),
        ];

        // 命令派发回执：`app.settings` ⇒ 打开设置窗口。
        //
        // 回执带 `&mut Context<CommandPaletteGpui>`：开窗需要 `&mut App`，
        // 而 `Context` 实现了 `DerefMut<Target = App>` ⇒ `&mut **cx` 即 `&mut App`。
        // 其余命令 id 只上报意图：界面层不实现业务语义。
        palette.update(cx, |view, _cx| {
            view.set_on_execute(|id, panel_cx| {
                if id != SETTINGS_COMMAND_ID {
                    return;
                }
                open_settings_window(panel_cx);
            });
        });

        Self {
            sidebar,
            session,
            right_panel,
            input,
            palette,
            left_divider,
            right_divider,
            tab_strip,
            _layout_sync,
            _session_sync,
            pending_seq: None,
            registry: default_registry(),
            focus: cx.focus_handle(),
        }
    }

    /// 取走输入区队列里的待投递输入，逐条**本地回显**到会话流。
    ///
    /// 根视图只 `take_pending` 消费输入区自己的 `outbox`，不另建队列；
    /// 回显期间**不清 `in_flight`**，等 [`Self::settle_pending`] 解除。
    fn drain_outbox(&mut self, cx: &mut Context<Self>) {
        let mut echoed = false;
        let input = self.input.clone();
        while let Some(outgoing) = input.update(cx, |input, _| input.take_pending()) {
            let session = self.session.clone();
            let pending = session.update(cx, |view, cx| {
                let state = view.state_mut();
                let seq = next_echo_seq(state);
                if state.push_message(seq, echo_message(seq, &outgoing)) {
                    cx.notify();
                    Some(seq)
                } else {
                    None
                }
            });
            if let Some(seq) = pending {
                self.pending_seq = Some(seq);
                echoed = true;
            }
        }
        if echoed {
            cx.notify();
        }
    }

    /// 会话流推进后：若已有回应抵达，解除输入区的「等待确认」态。
    fn settle_pending(&mut self, cx: &mut Context<Self>) {
        let ready = {
            let session = self.session.read(cx);
            should_finish_send(self.pending_seq, session.state())
        };
        if !ready {
            return;
        }
        self.pending_seq = None;
        self.input.update(cx, |input, cx| input.finish_send(cx));
    }

    /// 审批读取侧同步：根视图（经由会话流）是唯一写入者，右栏只是副本。
    fn sync_approvals(&mut self, cx: &mut Context<Self>) {
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

    /// tab 条刚被点击 ⇒ 把它的选中态 / 折叠态**回写**给权威版式（左分隔条）。
    ///
    /// 为什么必须有这一步：`sync_layout` 是「以左分隔条为唯一来源**下发**」，
    /// 而 tab 条自己的点击改的是它私有的 `RightPanelLayout`。若只观察 tab 条就直接
    /// `sync_layout`，下发会用权威版式里**旧的** tab 选中态把刚点出来的新状态覆盖掉
    /// —— 表现就是「点 tab 完全没反应」（2026-10-21 Windows 真机实测到的缺陷，
    /// 编译与单元测试都发现不了，因为纯函数 `tab_strip_layout` 本身是对的）。
    ///
    /// 只回写 tab 选中态与折叠态，**不动宽度**：宽度归分隔条管。
    fn adopt_tab_strip(&mut self, cx: &mut Context<Self>) {
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
    fn adopt_right_divider(&mut self, cx: &mut Context<Self>) {
        let incoming_width = self.right_divider.read(cx).layout().right_panel().width();
        self.left_divider.update(cx, |divider, cx| {
            let panel = divider.layout_mut().right_panel_mut();
            if panel.width() != incoming_width {
                panel.set_width(incoming_width);
                cx.notify();
            }
        });
    }

    /// 把版式（左分隔条 —— 唯一来源）下发到右分隔条与右侧面板。
    ///
    /// - 右分隔条整份覆盖：它只负责拖拽，不持有独立的版式主张。
    /// - 右侧面板**就地**用 [`RightPanelGpui::layout_mut`] 改选中 tab / 折叠态，
    ///   **不重建 `Entity`**（重建会丢面板里审批卡片的本地态）；宽度归分隔条管，
    ///   这里不动它的宽度字段。
    ///
    /// 回写前先比较，没变化就不 `notify()`，避免 observe → notify → observe 的循环。
    ///
    /// **最后必须 `cx.notify()` 自己**：三区宽度是根视图渲染时从左分隔条读回的，
    /// 子实体的 `notify()` 只会重绘子视图本身；没有这一步，拖动分隔条后
    /// 根视图不会重排三区 —— 表现成「分隔条跟手走、面板宽度不动」（2026-10-21
    /// Windows 真机实测：分隔条元素位置变了，侧栏右边界纹丝不动）。
    /// GPUI 对重复 `notify` 幂等，这里无条件调用是安全的。
    fn sync_layout(&mut self, cx: &mut Context<Self>) {
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

        cx.notify();
    }

    /// 处理根视图的按键：只做快捷键转换与面板开关，其余按键放行。
    fn handle_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(hotkey) = palette_gpui::hotkey_from_keystroke(&event.keystroke) else {
            return;
        };
        let registry = self.registry.clone();
        let palette = self.palette.clone();
        let action = palette.update(cx, |view, view_cx| {
            let action = command_palette::handle_hotkey(view.state_mut(), &registry, hotkey);
            if !matches!(action, PaletteAction::Ignored) {
                view_cx.notify();
            }
            action
        });
        // 打开时把键盘焦点交给面板（面板内部已处理方向键 / 回车 / Esc）。
        if matches!(action, PaletteAction::Open) {
            let focus = palette.read(cx).focus_handle().clone();
            window.focus(&focus, cx);
        }
    }

    /// 根视图焦点句柄（启动时用于把键盘焦点交给根视图）。
    fn focus_handle(&self) -> FocusHandle {
        self.focus.clone()
    }

    /// 标题栏：窗口标题 + 快捷键提示。
    fn title_bar(theme: &gpui_kit::component::Theme) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .px_4()
            .py_2()
            .bg(theme.title_bar)
            .border_b_1()
            .border_color(theme.title_bar_border)
            .child(div().text_sm().font_bold().child(WINDOW_TITLE))
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(HOTKEY_HINT),
            )
    }
}

impl Render for AppShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 兜底回显：输入区发送后 `cx.notify()` 会经 observe 触发 `drain_outbox`；
        // 这里在组装元素树之前再消费一次，保证同一帧内回显已写入会话流。只读消费，不投递。
        self.drain_outbox(cx);

        let viewport_width = f32::from(window.viewport_size().width);
        // 三区宽度**每次渲染都从左分隔条读回**（拖拽结果落在它内部），
        // 根视图不另存版式，因此不存在第二份不同步的副本。
        let widths = regions_for(self.left_divider.read(cx).layout(), viewport_width);
        // 把实际宽度**下发给侧栏视图**：它此前写死 `SIDEBAR_DEFAULT_WIDTH` 渲染
        // 自身内容，导致拖动后内容仍按 260 画（2026-10-21 真机实测）。
        // 必须在 `cx.theme()` 之前做（那里开始借用 `cx`）。
        self.sidebar
            .update(cx, |view, _| view.set_width(widths.sidebar));

        let theme = cx.theme();
        // 左侧边栏：宽度来自分隔条持有的版式（默认 260px，可拖拽）。
        // **必须 `flex_none()`**：在 flex 行里默认 `flex_shrink: 1`，只写 `.w()` 会被
        // 收缩，表现成「拖了分隔条、边栏宽度却没变」（2026-10-21 真机实测）。
        let sidebar = div()
            .w(px(widths.sidebar))
            .flex_none()
            .h_full()
            .child(self.sidebar.clone());

        // 中间会话流：吃掉剩余宽度，低于最小值时按最小值算。
        let stream = div()
            .flex_1()
            .min_w(px(STREAM_MIN_WIDTH))
            .h_full()
            .child(self.session.clone());

        // 右侧面板：上方是 tab 条（点击切换面板内容），下方是面板本身。
        // 同 sidebar：必须 `flex_none()`，否则拖拽改的宽度会被 flex 收缩吞掉。
        let right_panel = div()
            .flex()
            .flex_col()
            .w(px(widths.right_panel))
            .flex_none()
            .h_full()
            .child(self.tab_strip.clone())
            .child(div().flex_1().child(self.right_panel.clone()));

        // 中右两列：上面是会话流 + 分隔条 + 右栏，下面是输入区。
        let stream_and_panel = div()
            .flex()
            .flex_1()
            .child(stream)
            .child(self.right_divider.clone())
            .child(right_panel);
        let right_column = div()
            .flex()
            .flex_col()
            .flex_1()
            .child(stream_and_panel)
            .child(self.input.clone());

        div()
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.background)
            .text_color(theme.foreground)
            .id("lengsuo-app-shell")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.handle_key(event, window, cx);
            }))
            .child(Self::title_bar(theme))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .child(sidebar)
                    .child(self.left_divider.clone())
                    .child(right_column),
            )
            // 命令面板浮在最上层；关闭时它自己渲染为空元素。
            .child(self.palette.clone())
    }
}
