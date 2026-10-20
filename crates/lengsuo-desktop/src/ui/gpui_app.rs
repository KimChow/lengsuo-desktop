//! GPUI 最小窗口骨架：一个中文标题 + 一个中文占位符的多行输入框。
//!
//! 目的只有一个：让中文 IME 问题可复现、可验证（配合 `d1` 卡）。
//! 这里**不接**真实运行时、协议与业务状态，数据也不来自 [`crate::state`]。
#![cfg(feature = "ui")]

use std::sync::{Arc, Mutex};

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

use super::gpui_input::SkeletonInput;

/// 系统窗口标题。
pub const WINDOW_TITLE: &str = "冷锁桌面端 · GPUI 骨架";
/// 页面大标题（中文，用于确认 CJK 字体已加载）。
pub const HEADING: &str = "冷锁桌面端";
/// 页面说明，提示这张骨架的用途。
pub const HINT: &str =
    "GPUI 最小骨架：标题与输入框均为中文。请在下方输入框触发输入法，观察候选窗位置。";

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

/// 根视图：只组合标题、说明与输入框。
struct SkeletonView {
    input: Entity<SkeletonInput>,
}

impl SkeletonView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| SkeletonInput::new(window, cx));
        Self { input }
    }
}

impl Render for SkeletonView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .p_6()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(div().text_2xl().font_bold().child(HEADING))
            .child(div().text_sm().child(HINT))
            .child(self.input.clone())
    }
}

/// 打开最小窗口并进入事件循环。
///
/// 在 Windows 上这是目标平台；在 Linux 上仅用于编译验证（容器内通常没有
/// X11/Wayland 显示服务器，运行会失败，但编译应通过）。
///
/// 窗口创建失败时记录错误并退出事件循环，因此在没有显示服务器的环境下
/// 调用不会卡死，而是返回 [`LaunchError::Window`]。
pub fn launch() -> Result<(), LaunchError> {
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
                size(px(960.), px(680.)),
                cx,
            ))),
            ..Default::default()
        };

        if let Err(error) = gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| SkeletonView::new(window, cx))
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
