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
//!   - `profile_rules`：profile 同步的**收集规则**（白名单 / 黑名单 / 上限），
//!     与内核侧 `native/profile_bundle.*` 逐条对齐
//!   - `zip_writer`：profile 包的**清单层与写出层**（契约 §4.5 的清单形状 +
//!     流式 zip 写）
//!   - `upload`：profile 同步的**编排层**（打包 ⇒ 上传 ⇒ 失败不阻塞关闭）
//!   - `fp_values`：指纹值治理的**目标值模型**（解析 / 校验 / 下发给内核，
//!     与内核侧 T3 的目标值结构逐字段一致）
//!   - `fp_values_cache`：指纹值的**本地缓存**（每环境一份、加密落盘、原子写）
//!   - `fp_values_fetch`：指纹值的**获取与续期**（带 `sign` 请求 ⇒ 落缓存）
//!   - `fp_values_report`：指纹值的**回传**（聚合采样 ⇒ 送服务端；默认关闭）
//! - `ui`：GPUI 视图（`feature = "ui"`）

#![forbid(unsafe_code)]

pub mod kernel_pipe;
pub mod state;
pub mod theme;
pub mod ui;
