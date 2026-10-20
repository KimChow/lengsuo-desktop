//! 视图状态层（纯逻辑，无 IO、无渲染、无 async）。
//!
//! 本层为渲染层供数：所有界面读取都走这里，不直接触碰协议或运行时。

pub mod env_list;
pub mod org;
pub mod session_stream;
