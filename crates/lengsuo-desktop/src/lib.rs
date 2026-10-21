//! `lengsuo-desktop`：GPUI 桌面壳的**界面层**（纯逻辑 + 渲染）。
//!
//! 职责边界：
//! - 本 crate 只负责**界面逻辑与渲染**：视图状态、主题令牌、交互语义、GPUI 视图。
//! - **不实现**协议客户端、不实现运行时；这些在打包时由 `lengsuo-agent` 侧提供并整合。
//! - 阶段一（本批）：纯逻辑，不依赖 gpui（`cargo test` 不开 `ui` feature 即可覆盖）。
//! - 阶段二：`ui = []` feature 打开后引入 GPUI 渲染层。
//!
//! 分层约定（自下而上）：
//! - `state`：视图状态机（纯 Rust，无 IO、无渲染）
//! - `theme`：设计令牌
//! - `kernel_pipe`：内核管道的**纯逻辑层**（帧编解码 / 密钥派生 / 握手状态机）
//! - `ui`：GPUI 视图（`feature = "ui"`）

#![forbid(unsafe_code)]

pub mod kernel_pipe;
pub mod state;
pub mod theme;
pub mod ui;
