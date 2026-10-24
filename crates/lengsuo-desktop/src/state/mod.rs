//! 视图状态层（纯逻辑，无 IO、无渲染、无 async）。
//!
//! 本层为渲染层供数：所有界面读取都走这里，不直接触碰协议或运行时。
//!
//! 下面的 `pub mod` 声明由主理人一次性落盘，使各任务卡只需改自己新建的
//! 文件，避免多卡共改本文件造成路径归属冲突。

pub mod datasource;
pub mod env_group;
pub mod env_list;
pub mod file_intent;
pub mod hotkeys;
pub mod layout_prefs;
pub mod org;
pub mod org_perm;
pub mod palette_state;
pub mod prefs;
pub mod preview_kind;
pub mod preview_payload;
pub mod session_demo;
pub mod session_stream;
pub mod terminal;
