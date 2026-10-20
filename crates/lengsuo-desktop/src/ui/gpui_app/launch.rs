//! 窗口启动：标题文案、显示服务器预检、事件循环入口。
//!
//! Linux headless：窗口创建失败时返回 [`LaunchError::Window`] 而不是 panic / 卡死。

use std::sync::{Arc, Mutex};

use gpui_kit::*;

use super::AppShell;

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

/// 当前进程环境是否**指名**了一个显示服务器。
///
/// 只读环境变量（`DISPLAY` / `WAYLAND_DISPLAY`），不去连接：能不能连上要
/// 等平台初始化才知道，那时已经进事件循环了。
///
/// - Windows / macOS：没有这两个变量，但系统一定有窗口系统 ⇒ 一律 `true`。
/// - Linux / BSD：两个变量都空 ⇒ 退化成 headless 平台 ⇒ `false`。
pub(super) fn has_display_server() -> bool {
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
            let focus = view.read(cx).focus_handle();
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
