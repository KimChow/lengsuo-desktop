//! 桌面端二进制入口：启动 GPUI 工作台。
//!
//! `Cargo.toml` 里本 bin 声明了 `required-features = ["ui"]`，所以只有开
//! `ui` feature 才会被编译。未开 feature 时（例如有人手写
//! `cargo run --bin lengsuo-desktop` 之外的路径）本文件整体被 `#![cfg]`
//! 掏空，打印提示并以非零码退出。
//!
//! 失败处理：**不 panic**。窗口创建失败（例如 Linux 容器里没有 X11/Wayland
//! 显示服务器）时打印错误并以退出码 `1` 结束，调用方不会看到 panic 回溯。

/// 未开 `ui` feature 时的提示文案。
#[cfg(not(feature = "ui"))]
const NO_UI_FEATURE_HINT: &str =
    "lengsuo-desktop 需要 `ui` feature 才能运行：\n  cargo run -p lengsuo-desktop --features ui --bin lengsuo-desktop";

#[cfg(not(feature = "ui"))]
fn main() {
    eprintln!("{NO_UI_FEATURE_HINT}");
    std::process::exit(2);
}

#[cfg(feature = "ui")]
fn main() {
    // 失败时（无显示服务器等）打印错误并以非零码退出，绝不 panic。
    if let Err(error) = lengsuo_desktop::ui::gpui_app::launch() {
        eprintln!("lengsuo-desktop 启动失败：{error}");
        std::process::exit(1);
    }
}
