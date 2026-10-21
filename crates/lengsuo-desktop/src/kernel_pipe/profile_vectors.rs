//! 任务 R 的**跨仓固定向量**：`ConfigBundle` 帧与契约 §4.5 的清单形状。
//!
//! ⭐ 两侧必须**逐字节相同**：
//! - 客户端侧：[`crate::kernel_pipe::upload`]；
//! - 内核侧：`native/profile_bundle_test.cc`（内核侧自己拼一份默认载荷，
//!   与本文件逐字节比较）。
//!
//! ⛔ 固定向量**硬编码**——不在测试里调用外部脚本生成期望值：
//! 那样两边都错也会「一致」。
//!
//! ⛔ 载荷里不许有密钥、会话密钥、真实用户数据、`data_url`（拍板 3A）。

// ---------------------------------------------------------------------------
// ConfigBundle 的跨仓固定向量（任务 R）
// ---------------------------------------------------------------------------

/// 默认规则下的 `ConfigBundle` 载荷（**155 字节**，跨仓固定向量）。
///
/// ⭐ 两侧必须**逐字节相同**：
/// - 客户端侧：[`crate::kernel_pipe::upload::config_bundle_payload`]
/// - 内核侧：`native/profile_bundle_test.cc` 的
///   `TheDefaultSectionRoundTripsThroughTheParser`（内核侧自己拼一份，
///   与本行逐字节比较）。
///
/// ⛔ 载荷里不许有密钥、会话密钥、真实用户数据、`data_url`（拍板 3A）。
pub(crate) const CONFIG_BUNDLE_PAYLOAD: &str = concat!(
    r#"{"profile_sync":{"include":["*Local Storage*","*Extension State*","*History*"],"#,
    r#""exclude":["*Cache*","*Cookies*","*Network*"],"#,
    r#""max_package_bytes":33554432}}"#
);

/// `ConfigBundle` 帧头的 8 字节（`len` 大端 = 155、`type` = 0x03、`reserved` 全 0）。
pub(crate) const CONFIG_BUNDLE_HEADER: [u8; 8] = [0x00, 0x00, 0x00, 0x9b, 0x03, 0x00, 0x00, 0x00];

/// 契约 §4.5 的清单 JSON 形状（**跨仓固定向量**）。
///
/// 内核侧产出的清单就是这个形状；客户端侧据此打包。
/// ⛔ 条目**恰好三字段**（`path` / `size` / `kind`）。
pub(crate) const MANIFEST_JSON: &str =
    r#"{"files":[{"path":"Local Storage/leveldb/000003.log","size":1024,"kind":"file"}]}"#;

#[test]
fn the_config_bundle_payload_is_the_cross_repo_vector() {
    // ⭐ 客户端侧产出的载荷必须与固定向量逐字节相同。
    use crate::kernel_pipe::upload::{config_bundle_payload, ProfileSyncRules};
    assert_eq!(
        config_bundle_payload(&ProfileSyncRules::default()),
        CONFIG_BUNDLE_PAYLOAD,
        "ConfigBundle 载荷必须与跨仓固定向量逐字节相同"
    );
    // 长度钉死（帧头的 len 字段就是它）。
    assert_eq!(CONFIG_BUNDLE_PAYLOAD.len(), 155);
}

#[test]
fn the_config_bundle_frame_header_is_the_cross_repo_vector() {
    use crate::kernel_pipe::upload::{config_bundle_frame, ProfileSyncRules};
    let frame = config_bundle_frame(&ProfileSyncRules::default());
    let encoded = frame.encode().expect("应可编码");
    assert_eq!(
        &encoded[..8],
        &CONFIG_BUNDLE_HEADER,
        "帧头必须是 len=155（大端）、type=0x03、reserved=0"
    );
    assert_eq!(&encoded[8..], CONFIG_BUNDLE_PAYLOAD.as_bytes());
}

#[test]
fn the_manifest_shape_is_the_cross_repo_vector() {
    // ⭐ 契约 §4.5：内核侧产出的清单，客户端侧必须能按同一形状解析。
    use crate::kernel_pipe::zip_writer::ProfileManifest;
    let manifest = ProfileManifest::parse(MANIFEST_JSON).expect("应能按契约形状解析");
    assert_eq!(manifest.files.len(), 1);
    assert_eq!(manifest.files[0].path, "Local Storage/leveldb/000003.log");
    assert_eq!(manifest.files[0].size, 1024);
    assert_eq!(manifest.total_bytes, 1024);
}
