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
//! - 宽度取自 [`crate::ui::layout`]（边栏 260px / 右栏 320px 默认）。
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
use crate::ui::layout::{WorkbenchLayout, STREAM_MIN_WIDTH};
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

/// 工作台根视图：三区 + 输入区 + 命令面板浮层。
struct AppShell {
    sidebar: Entity<SidebarViewGpui>,
    session: Entity<SessionStreamGpui>,
    right_panel: Entity<RightPanelGpui>,
    input: Entity<InputAreaGpui>,
    palette: Entity<CommandPaletteGpui>,
    /// 版式状态：只用于计算三区宽度，拖拽调宽后续再接。
    layout: WorkbenchLayout,
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

        Self {
            sidebar,
            session,
            right_panel,
            input,
            palette,
            layout,
            registry: default_registry(),
            focus: cx.focus_handle(),
        }
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
        let widths = self.layout.regions(viewport_width);

        // 左侧边栏：宽度来自版式状态（默认 260px）。
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

        // 右侧面板：宽度来自版式状态（默认 320px）。
        let right_panel = div()
            .w(px(widths.right_panel))
            .h_full()
            .child(self.right_panel.clone());

        // 中右两列：上面是会话流 + 右栏，下面是输入区。
        let right_column = div()
            .flex()
            .flex_col()
            .flex_1()
            .child(div().flex().flex_1().child(stream).child(right_panel))
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
            .child(div().flex().flex_1().child(sidebar).child(right_column))
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
        default_registry, demo_session, has_display_server, secondary_views, HOTKEY_HINT,
        NO_DISPLAY_MESSAGE, WINDOW_TITLE,
    };
    use crate::state::session_stream::MessageRole;
    use crate::ui::layout::{
        WorkbenchLayout, RIGHT_PANEL_DEFAULT_WIDTH, SIDEBAR_DEFAULT_WIDTH, STREAM_MIN_WIDTH,
    };

    #[test]
    fn demo_session_covers_four_roles_and_one_pending_approval() {
        let state = demo_session();
        assert_eq!(state.messages().len(), 4);
        let roles: Vec<MessageRole> = state.messages().iter().map(|m| m.role).collect();
        assert!(roles.contains(&MessageRole::User));
        assert!(roles.contains(&MessageRole::Assistant));
        assert!(roles.contains(&MessageRole::ToolCall));
        assert!(roles.contains(&MessageRole::ToolResult));
        assert_eq!(state.pending_approvals().count(), 1);
        assert!(!state.is_stale(), "演示数据不应处于断档态");
    }

    #[test]
    fn root_layout_uses_the_three_region_defaults() {
        let layout = WorkbenchLayout::new();
        let widths = layout.regions(1280.0);
        assert_eq!(widths.sidebar, SIDEBAR_DEFAULT_WIDTH);
        assert_eq!(widths.sidebar, 260.0);
        assert_eq!(widths.right_panel, RIGHT_PANEL_DEFAULT_WIDTH);
        assert_eq!(widths.right_panel, 320.0);
        assert_eq!(widths.stream, 1280.0 - 260.0 - 320.0);
        // 窗口过窄时会话流仍保住最小宽度（整体横向溢出，不压缩会话流）。
        let narrow = layout.regions(400.0);
        assert_eq!(narrow.stream, STREAM_MIN_WIDTH);
    }

    #[test]
    fn root_registry_opens_palette_on_ctrl_k() {
        let registry = default_registry();
        let hotkey = crate::state::hotkeys::Hotkey::parse("Ctrl+K").expect("内置快捷键可解析");
        assert_eq!(
            registry.command_for(hotkey),
            Some(crate::state::hotkeys::command_ids::PALETTE_OPEN),
            "根视图注册表必须把 Ctrl+K 解析为打开命令面板"
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
