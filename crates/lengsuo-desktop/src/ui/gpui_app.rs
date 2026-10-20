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
//!   [`regions_for`] 从它持有的 [`WorkbenchLayout`] 读回，**不另存副本**；
//!   右侧 tab 条（[`RightPanelTabGpui`]）从同一份版式派生选中态，点击后经
//!   [`sync_layout`] 就地写给右侧面板与右分隔条。
//! - 命令面板浮在最上层，快捷键经 `palette_gpui::hotkey_from_keystroke` 转换后
//!   交给 [`crate::ui::command_palette::handle_hotkey`]。
//! - 环境 / 组织视图本批**不进主界面**，但通过 [`secondary_views`] 编译进二进制。
//!
//! Linux headless：窗口创建失败时返回 [`LaunchError::Window`] 而不是 panic / 卡死。
#![cfg(feature = "ui")]

use std::sync::{Arc, Mutex};

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

use crate::state::hotkeys::{Binding, HotkeyRegistry};
use crate::state::session_stream::{
    MessageRole, PendingApproval, SessionMessage, SessionStreamState,
};
use crate::ui::command_palette::{self, PaletteAction};
use crate::ui::env_gpui::EnvViewGpui;
use crate::ui::input_gpui::InputAreaGpui;
use crate::ui::layout::{RegionWidths, RightPanelLayout, WorkbenchLayout, STREAM_MIN_WIDTH};
use crate::ui::layout_gpui::{DividerGpui, DividerSide, RightPanelTabGpui};
use crate::ui::org_gpui::OrgViewGpui;
use crate::ui::org_view::OrgViewState;
use crate::ui::palette_gpui::{self, CommandPaletteGpui};
use crate::ui::right_panel_gpui::RightPanelGpui;
use crate::ui::session_gpui::SessionStreamGpui;
use crate::ui::sidebar_gpui::{self, SidebarViewGpui};

/// 系统窗口标题。
pub const WINDOW_TITLE: &str = "冷锁桌面端 · 工作台";
/// 标题栏右侧的快捷键提示。
pub const HOTKEY_HINT: &str = "Ctrl+K 打开命令面板";
/// 没有显示服务器时的错误文案。
pub const NO_DISPLAY_MESSAGE: &str =
    "当前环境没有可用的显示服务器（DISPLAY / WAYLAND_DISPLAY 均为空），无法打开窗口";

/// 启动失败的原因。
#[derive(Debug)]
pub enum LaunchError {
    /// 窗口创建失败（例如没有可用的显示服务器）。
    Window(String),
}

impl std::fmt::Display for LaunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Window(message) => write!(f, "打开 GPUI 窗口失败：{message}"),
        }
    }
}

impl std::error::Error for LaunchError {}

/// 一根演示消息。
fn message(role: MessageRole, content: &str, tool: Option<&str>) -> SessionMessage {
    SessionMessage {
        role,
        content: content.into(),
        tool: tool.map(str::to_owned),
        ts_ms: 0,
    }
}

/// 演示用会话流：手工构造，**不接真实数据源**。
///
/// 覆盖四类消息与一个挂起审批，供根视图组装时验证各渲染分支都有内容。
#[must_use]
pub fn demo_session() -> SessionStreamState {
    let mut state = SessionStreamState::new();
    state.push_message(
        1,
        message(MessageRole::User, "帮我看一下构建脚本为什么失败", None),
    );
    state.push_message(
        2,
        message(MessageRole::Assistant, "好的，我先读取入口脚本。", None),
    );
    state.push_message(
        3,
        message(MessageRole::ToolCall, "scripts/card_guard.py", Some("Read")),
    );
    state.push_message(
        4,
        message(
            MessageRole::ToolResult,
            "#!/usr/bin/env python3\nprint('ok')",
            Some("Read"),
        ),
    );
    state.push_approval(PendingApproval {
        id: "demo-approval-1".into(),
        tool: "Bash".into(),
        summary: "运行 cargo test --workspace".into(),
        resolved: None,
    });
    state
}

/// 内置快捷键表；内置表若自相冲突则退化为空表（不 panic）。
fn default_registry() -> HotkeyRegistry {
    HotkeyRegistry::with_defaults().unwrap_or_else(|_| {
        HotkeyRegistry::from_bindings(std::iter::empty::<Binding>()).expect("空绑定表必定无冲突")
    })
}

/// 环境 / 组织视图的演示实例。
///
/// 本批**不进主界面**（只做工作台四件），但要求代码编译进二进制。
/// 后续卡片接入导航时直接挂载这两个视图即可。
#[must_use]
pub fn secondary_views() -> (EnvViewGpui, OrgViewGpui) {
    (
        EnvViewGpui::demo(),
        OrgViewGpui::new(OrgViewState::default()),
    )
}

/// 由版式状态算三区宽度（渲染读宽度的**唯一入口**）。
///
/// 版式来自左分隔条持有的 [`WorkbenchLayout`]，本函数不持有任何状态，
/// 因此「渲染读到的宽度」与「分隔条持有的宽度」天然是同一份，不会有第二份副本。
///
/// fail-closed：窗口宽度取不到（NaN / 负值）时按 0 算 —— `regions` 会把会话流
/// 夹到 [`STREAM_MIN_WIDTH`]，边栏 / 右栏取各自默认或当前宽度，三者都**不为 0**。
#[must_use]
fn regions_for(layout: &WorkbenchLayout, viewport_width: f32) -> RegionWidths {
    let available = if viewport_width.is_finite() {
        viewport_width.max(0.0)
    } else {
        0.0
    };
    layout.regions(available)
}

/// 渲染右侧面板 / tab 条时应使用的版式：以左分隔条持有的版式为准。
///
/// 回写到左侧时保留 `current` 的宽度（宽度归分隔条管，tab 条只管选中 / 折叠），
/// 回写右侧时整份以左侧为准（选中 tab 随即生效）。
#[must_use]
fn tab_strip_layout(authority: &WorkbenchLayout, current: &RightPanelLayout) -> RightPanelLayout {
    let mut layout = current.clone();
    layout.select_tab(authority.right_panel().tab());
    if authority.right_panel().is_collapsed() {
        layout.collapse();
    } else {
        layout.expand();
    }
    layout
}

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
    /// 与命令面板共用同一套内置快捷键表（按 id 派发）。
    registry: HotkeyRegistry,
    /// 根视图焦点：全局按键先到这里。
    focus: FocusHandle,
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
                shell.sync_layout(cx)
            }),
            cx.observe(&tab_strip, |shell: &mut Self, _, cx| shell.sync_layout(cx)),
        ];

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
            registry: default_registry(),
            focus: cx.focus_handle(),
        }
    }

    /// 把版式（左分隔条 —— 唯一来源）下发到右分隔条与右侧面板。
    ///
    /// - 右分隔条整份覆盖：它只负责拖拽，不持有独立的版式主张。
    /// - 右侧面板**就地**用 [`RightPanelGpui::layout_mut`] 改选中 tab / 折叠态，
    ///   **不重建 `Entity`**（重建会丢面板里审批卡片的本地态）；宽度归分隔条管，
    ///   这里不动它的宽度字段。
    ///
    /// 回写前先比较，没变化就不 `notify()`，避免 observe → notify → observe 的循环。
    fn sync_layout(&mut self, cx: &mut Context<Self>) {
        let authority = self.left_divider.read(cx).layout().clone();

        let right_divider = self.right_divider.clone();
        right_divider.update(cx, |divider, cx| {
            if divider.layout() != &authority {
                *divider.layout_mut() = authority.clone();
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
        let theme = cx.theme();
        let viewport_width = f32::from(window.viewport_size().width);
        // 三区宽度**每次渲染都从左分隔条读回**（拖拽结果落在它内部），
        // 根视图不另存版式，因此不存在第二份不同步的副本。
        let widths = regions_for(self.left_divider.read(cx).layout(), viewport_width);

        // 左侧边栏：宽度来自分隔条持有的版式（默认 260px，可拖拽）。
        let sidebar = div()
            .w(px(widths.sidebar))
            .h_full()
            .child(self.sidebar.clone());

        // 中间会话流：吃掉剩余宽度，低于最小值时按最小值算。
        let stream = div()
            .flex_1()
            .min_w(px(STREAM_MIN_WIDTH))
            .h_full()
            .child(self.session.clone());

        // 右侧面板：上方是 tab 条（点击切换面板内容），下方是面板本身。
        let right_panel = div()
            .flex()
            .flex_col()
            .w(px(widths.right_panel))
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

/// 当前进程环境是否**指名**了一个显示服务器。
///
/// 只读环境变量（`DISPLAY` / `WAYLAND_DISPLAY`），不去连接：能不能连上要
/// 等平台初始化才知道，那时已经进事件循环了。
///
/// - Windows / macOS：没有这两个变量，但系统一定有窗口系统 ⇒ 一律 `true`。
/// - Linux / BSD：两个变量都空 ⇒ 退化成 headless 平台 ⇒ `false`。
fn has_display_server() -> bool {
    if !cfg!(any(target_os = "linux", target_os = "freebsd")) {
        return true;
    }
    let named = |name: &str| {
        std::env::var_os(name)
            .map(|value| !value.is_empty())
            .unwrap_or(false)
    };
    named("DISPLAY") || named("WAYLAND_DISPLAY")
}

/// 打开工作台窗口并进入事件循环。
///
/// 在 Windows 上这是目标平台；在 Linux 上仅用于编译验证（容器内通常没有
/// X11/Wayland 显示服务器，运行会失败，但编译应通过）。
///
/// 窗口创建失败时记录错误并退出事件循环，因此在没有显示服务器的环境下
/// 调用不会卡死，而是返回 [`LaunchError::Window`]。
pub fn launch() -> Result<(), LaunchError> {
    // **开跑前先探测显示服务器**：Linux 的 GPUI 后端在没有 `DISPLAY` /
    // `WAYLAND_DISPLAY` 时会退化成 headless 平台，而 headless 的
    // `open_window` **会成功**（返回一个不可见的窗口）—— 那样事件循环会
    // 一直跑下去，表现为「卡住」而不是报错。任务书要求「报错退出而非
    // panic / 卡死」，所以这里提前失败。
    if !has_display_server() {
        return Err(LaunchError::Window(NO_DISPLAY_MESSAGE.to_owned()));
    }

    let failure: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let failure_in_run = Arc::clone(&failure);

    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        let options = WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some(WINDOW_TITLE.into()),
                ..Default::default()
            }),
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(1280.), px(800.)),
                cx,
            ))),
            ..Default::default()
        };

        if let Err(error) = gpui_kit::open_window(options, cx, |window, cx| {
            let view = cx.new(|cx| AppShell::new(window, cx));
            // 先克隆焦点句柄再 focus：`view.read(cx)` 的不可变借用不能与
            // `focus(window, cx)` 的 `&mut cx` 重叠。
            let focus = view.read(cx).focus.clone();
            window.focus(&focus, cx);
            view
        }) {
            *failure_in_run.lock().unwrap() = Some(error.to_string());
            cx.quit();
        }
    });

    let failure = failure.lock().unwrap().take();
    match failure {
        Some(message) => Err(LaunchError::Window(message)),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    // 显式导入（不用 `use super::*`）：`gpui_kit::*` 会把 GPUI 的 `test` 属性宏
    // 带进来并遮蔽标准库的 `#[test]`，导致测试无法编译。
    use super::{
        default_registry, demo_session, has_display_server, regions_for, secondary_views,
        tab_strip_layout, HOTKEY_HINT, NO_DISPLAY_MESSAGE, WINDOW_TITLE,
    };
    use crate::state::session_stream::MessageRole;
    use crate::ui::layout::{
        RightPanelLayout, RightPanelTab, WorkbenchLayout, RIGHT_PANEL_COLLAPSED_WIDTH,
        RIGHT_PANEL_DEFAULT_WIDTH, SIDEBAR_DEFAULT_WIDTH, STREAM_MIN_WIDTH,
    };
    use crate::ui::layout_gpui::{DividerDrag, DividerSide};

    #[test]
    fn demo_session_covers_four_roles_and_one_pending_approval() {
        let state = demo_session();
        let mut roles: Vec<MessageRole> = state.messages().iter().map(|m| m.role).collect();
        roles.sort_by_key(|role| format!("{role:?}"));
        roles.dedup();
        assert_eq!(roles.len(), 4, "演示数据必须覆盖四类消息");
        assert_eq!(state.pending_approvals().count(), 1);
    }

    /// 版式单一来源：渲染读到的宽度 = 分隔条持有的版式算出的宽度。
    ///
    /// 拖拽改的是 [`DividerGpui`] 内部持有的那份 [`WorkbenchLayout`]；这里用与
    /// 分隔条同一条拖拽路径（[`DividerDrag`]）改宽度，再经 [`regions_for`]
    /// 读回 —— 根视图渲染走的就是这一条路径，中间没有任何副本。
    #[test]
    fn rendered_widths_come_from_the_divider_layout() {
        let mut divider_layout = WorkbenchLayout::new();
        let mut drag = DividerDrag::new();
        assert!(drag.begin(DividerSide::Left, 100.0, &divider_layout));
        assert!(drag.drag_to(150.0, &mut divider_layout));
        assert!(drag.end());

        let widths = regions_for(&divider_layout, 1280.0);
        let expected = divider_layout.regions(1280.0);
        assert_eq!(widths, expected);
        assert_eq!(widths.sidebar, SIDEBAR_DEFAULT_WIDTH + 50.0, "拖拽结果生效");
        assert_eq!(widths.right_panel, RIGHT_PANEL_DEFAULT_WIDTH);
        assert_eq!(widths.stream, 1280.0 - widths.sidebar - widths.right_panel);

        // 右侧分隔条拖拽同样从同一份版式读出。
        let mut drag = DividerDrag::new();
        assert!(drag.begin(DividerSide::Right, 900.0, &divider_layout));
        assert!(drag.drag_to(860.0, &mut divider_layout));
        let widths = regions_for(&divider_layout, 1280.0);
        assert_eq!(
            widths.right_panel,
            divider_layout.right_panel().effective_width()
        );
        assert_eq!(widths.right_panel, RIGHT_PANEL_DEFAULT_WIDTH + 40.0);
    }

    /// tab 切换映射：由版式里的 tab 派生的右栏布局，选中态与 tab 一致。
    ///
    /// `tab_strip_layout` 是 `sync_layout` 用来把手写进 `RightPanelGpui` /
    /// `RightPanelTabGpui` 的那个纯函数；这里断言它把选中 tab 原样带过去。
    #[test]
    fn tab_selection_drives_the_right_panel_layout() {
        for tab in RightPanelTab::ALL {
            let mut authority = WorkbenchLayout::new();
            authority.right_panel_mut().select_tab(tab);
            let panel = tab_strip_layout(&authority, &RightPanelLayout::new());
            assert_eq!(panel.tab(), tab, "右侧面板选中的 tab 必须与 tab 条一致");
            assert!(!panel.is_collapsed());

            let rendered = crate::ui::right_panel::render(&panel, &demo_session());
            assert_eq!(rendered.active_tab, tab);
            assert!(
                rendered
                    .tabs
                    .iter()
                    .any(|button| button.tab == tab && button.selected),
                "tab 条上被选中的入口必须就是当前 tab"
            );
        }
    }

    /// tab 条回写时保留自己的宽度（宽度归分隔条管），但选中 / 折叠以左侧为准。
    #[test]
    fn tab_strip_layout_keeps_width_but_follows_the_authority() {
        let mut authority = WorkbenchLayout::new();
        authority
            .right_panel_mut()
            .select_tab(RightPanelTab::Preview);
        let current = RightPanelLayout::new();
        let synced = tab_strip_layout(&authority, &current);
        assert_eq!(synced.tab(), RightPanelTab::Preview);
        assert_eq!(synced.width(), current.width(), "宽度不由 tab 条决定");

        authority.right_panel_mut().collapse();
        let collapsed = tab_strip_layout(&authority, &current);
        assert!(collapsed.is_collapsed(), "折叠态同样以左侧为准");
    }

    /// fail-closed：描述层最小值下三区宽度都不为 0（分隔条最小宽度必须保留）。
    #[test]
    fn minimum_widths_never_collapse_a_region() {
        let mut layout = WorkbenchLayout::new();
        layout.set_sidebar_width(0.0);
        layout.right_panel_mut().set_width(0.0);
        let widths = regions_for(&layout, 800.0);
        assert!(widths.sidebar > 0.0);
        assert!(widths.stream >= STREAM_MIN_WIDTH);
        assert!(widths.right_panel > 0.0);

        layout.right_panel_mut().collapse();
        let collapsed = regions_for(&layout, 800.0);
        assert_eq!(collapsed.right_panel, RIGHT_PANEL_COLLAPSED_WIDTH);
        assert!(collapsed.stream >= STREAM_MIN_WIDTH);

        // 窗口宽度取不到（NaN / 负值 / 0）时也不得渲染成 0 宽。
        for viewport in [f32::NAN, -1.0, 0.0] {
            let widths = regions_for(&WorkbenchLayout::new(), viewport);
            assert!(widths.sidebar > 0.0, "viewport={viewport}");
            assert!(widths.stream > 0.0, "viewport={viewport}");
            assert!(widths.right_panel > 0.0, "viewport={viewport}");
        }
    }

    #[test]
    fn root_registry_opens_palette_on_ctrl_k() {
        let registry = default_registry();
        let hotkey = crate::state::hotkeys::Hotkey::parse("Ctrl+K").expect("内置快捷键可解析");
        assert_eq!(
            registry.command_for(hotkey),
            Some(crate::state::hotkeys::command_ids::PALETTE_OPEN)
        );
    }

    #[test]
    fn window_text_is_non_empty() {
        assert!(!WINDOW_TITLE.is_empty());
        assert!(!HOTKEY_HINT.is_empty());
        assert!(!NO_DISPLAY_MESSAGE.is_empty());
    }

    #[test]
    fn headless_linux_has_no_display_server() {
        // 本机（47）没有显示服务器：预检必须判定为「无」，`launch()` 才能
        // 立刻返回错误而不是进事件循环卡死。
        if cfg!(any(target_os = "linux", target_os = "freebsd"))
            && std::env::var_os("DISPLAY").is_none()
            && std::env::var_os("WAYLAND_DISPLAY").is_none()
        {
            assert!(!has_display_server());
        }
        // 有窗口系统的目标平台一律视为有显示服务器。
        if !cfg!(any(target_os = "linux", target_os = "freebsd")) {
            assert!(has_display_server());
        }
    }

    #[test]
    fn secondary_views_are_constructible() {
        let (_env, _org) = secondary_views();
    }
}
