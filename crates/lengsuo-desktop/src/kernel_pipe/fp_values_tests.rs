//! 指纹值治理（客户端侧）的测试。
//!
//! 覆盖任务书的必做项与验收：
//! - **跨仓对齐**：固定测试向量（客户端拼出的目标值 JSON 与内核侧 T3 的
//!   解析结果**逐字段相同**）。向量硬编码在测试源码里，⛔ **不许**在测试里
//!   调本仓之外的脚本生成期望值。
//! - **缓存损坏 ⇒ 重新拉取且浏览器能起来**。
//! - **值默认不采 `data_url`**。
//! - **落盘加密 / 原子写 / 按环境分目录**。
//! - **`sign` 不进日志、不进 `ConfigBundle`**。

use crate::kernel_pipe::fp_values::{
    config_bundle_frame_with_targets, config_bundle_payload_with_targets, fp_targets_section_json,
    short_digest, FpDimension, FpTargetSet, FpValueCacheEntry, FpValueStats, Redacted,
};
use crate::kernel_pipe::fp_values_cache::{sanitize_env_id, FpValueCache};
use crate::kernel_pipe::fp_values_fetch::{
    parse_check_json, refresh_values, FpValueCheck, FpValueFetchError, FpValueRequest,
    FpValueTransport,
};
use crate::kernel_pipe::fp_values_report::{
    parse_sample, report_samples, FpReportBatch, FpReportError, FpSample, FpSampleError,
    FpSampleReporter, SampleCollector, UnavailableReporter,
};
use crate::kernel_pipe::keys::K_MACHINE_LEN;
use crate::kernel_pipe::profile_rules::ProfileSyncRules;

// ---------------------------------------------------------------------------
// 跨仓固定测试向量（唯一真相，两侧逐字照抄）
// ---------------------------------------------------------------------------

/// 服务端下发的**值载荷**（固定向量）。
///
/// ⛔ 里面的哈希是**合成的测试夹具**，不是任何真实目标值。
const VALUE_PAYLOAD: &str = concat!(
    r#"{"similarity":8,"targets":["#,
    r#"{"dimension":"canvas","url_regex":"^https://example\\.com/","#,
    r#""target_hash":"a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90","#,
    r#""width":300,"length":150,"drew_text":true,"skip":false},"#,
    r#"{"dimension":"webgl","url_regex":"","#,
    r#""target_hash":"00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff","#,
    r#""width":512,"length":512}]}"#
);

/// 客户端拼出的**目标值段**（固定向量）。
///
/// ⭐ 这份字节就是内核侧 T3 要解析的那一段 ⇒ 两侧逐字相同。
const FP_TARGETS_SECTION: &str = concat!(
    r#"{"updated_at_unix":1700000000,"similarity":8,"report_enabled":false,"targets":["#,
    r#"{"dimension":"canvas","url_regex":"^https://example\\.com/","#,
    r#""target_hash":"a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90","#,
    r#""width":300,"length":150,"drew_text":true,"skip":false},"#,
    r#"{"dimension":"webgl","url_regex":"","#,
    r#""target_hash":"00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff","#,
    r#""width":512,"length":512}]}"#
);

/// 既有 `profile_sync` 段的跨仓固定向量（**必须逐字不变**）。
const PROFILE_SYNC_PAYLOAD: &str = concat!(
    r#"{"profile_sync":{"include":["*Local Storage*","*Extension State*","*History*"],"#,
    r#""exclude":["*Cache*","*Cookies*","*Network*"],"#,
    r#""max_package_bytes":33554432}}"#
);

// ---------------------------------------------------------------------------
// 夹具
// ---------------------------------------------------------------------------

/// 造一个临时目录（测试结束删掉）。
fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "lengsuo-fp-values-{tag}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("应能建临时目录");
    dir
}

/// 一把确定性的缓存密钥（测试夹具，⛔ 不是真实密钥）。
fn test_cache_key() -> [u8; 32] {
    FpValueCache::derive_key(&[0x5Au8; K_MACHINE_LEN])
}

/// 从固定向量造一个缓存条目。
fn fixed_entry() -> FpValueCacheEntry {
    let set = FpTargetSet::parse_json(VALUE_PAYLOAD).expect("固定向量应能解析");
    FpValueCacheEntry {
        sign: Some(Redacted::new("sign-fixture-value")),
        container_id: "ct-01".to_string(),
        updated_at_unix: 1_700_000_000,
        similarity: set.similarity,
        report_enabled: false,
        targets: set.targets,
    }
}

// ---------------------------------------------------------------------------
// 跨仓对齐：固定测试向量
// ---------------------------------------------------------------------------

#[test]
fn the_fixed_vector_parses_into_the_fixed_section_byte_for_byte() {
    // ⭐ 跨仓对齐证据：客户端拼出的段 = 内核侧 T3 解析的那一段，逐字节相同；期望值硬编码在测试源码里（⛔ 不许现场生成）。
    let set = FpTargetSet::parse_json(VALUE_PAYLOAD).expect("固定向量应能解析");
    assert_eq!(set.similarity, 8, "similarity 由服务端下发");
    assert_eq!(set.targets.len(), 2);
    assert_eq!(set.targets[0].dimension, FpDimension::Canvas);
    assert_eq!(set.targets[1].dimension, FpDimension::Webgl);
    assert_eq!(set.targets[0].url_regex, r"^https://example\.com/");
    assert_eq!(set.targets[0].width, 300);
    assert_eq!(set.targets[0].length, 150);
    assert_eq!(set.targets[0].drew_text, Some(true));
    assert_eq!(set.targets[0].skip, Some(false));
    assert_eq!(
        set.targets[1].url_regex, "",
        "空 url_regex = 该维度的默认目标"
    );

    let entry = fixed_entry();
    assert_eq!(
        fp_targets_section_json(&entry),
        FP_TARGETS_SECTION,
        "客户端拼出的目标值段必须与跨仓固定向量逐字节相同"
    );
}

#[test]
fn the_config_bundle_carries_both_sections_and_keeps_the_old_vector_intact() {
    // `profile_sync` 段的字节**逐字不变** ⇒ 既有跨仓固定向量继续成立。
    let entry = fixed_entry();
    let payload = config_bundle_payload_with_targets(&ProfileSyncRules::default(), Some(&entry));
    assert!(payload.starts_with(PROFILE_SYNC_PAYLOAD.trim_end_matches('}')));
    assert!(
        payload.contains(r#""fp_targets":"#),
        "必须有独立的 fp_targets 段：{payload}"
    );
    // 两段是**并列**的（⛔ 不许把目标值混进既有段的字段里）。
    let parsed: serde_json::Value = serde_json::from_str(&payload).expect("应是合法 JSON");
    let object = parsed.as_object().expect("应是 JSON 对象");
    assert_eq!(object.len(), 2, "顶层恰好两段：{payload}");
    assert!(object.contains_key("profile_sync"));
    assert!(object.contains_key("fp_targets"));
}

#[test]
fn no_targets_means_no_fp_targets_section() {
    // 目标值为空 ⇒ **不下发该段** ⇒ 内核侧退回现有行为。
    let mut entry = fixed_entry();
    entry.targets.clear();
    let payload = config_bundle_payload_with_targets(&ProfileSyncRules::default(), Some(&entry));
    assert_eq!(payload, PROFILE_SYNC_PAYLOAD, "目标值为空 ⇒ 不下发该段");
    let no_entry = config_bundle_payload_with_targets(&ProfileSyncRules::default(), None);
    assert_eq!(no_entry, PROFILE_SYNC_PAYLOAD, "没有条目 ⇒ 也不下发该段");
}

#[test]
fn the_config_bundle_frame_is_a_well_formed_frame() {
    let frame =
        config_bundle_frame_with_targets(&ProfileSyncRules::default(), Some(&fixed_entry()));
    assert_eq!(frame.frame_type.code(), 0x03, "帧类型必须是 0x03");
    let encoded = frame.encode().expect("应可编码");
    let (decoded, used) = crate::kernel_pipe::decode_frame(&encoded).expect("应能解码");
    assert_eq!(used, encoded.len());
    assert_eq!(decoded.payload, frame.payload);
}

// ---------------------------------------------------------------------------
// ⛔ 边界：sign 不进 ConfigBundle、不进日志
// ---------------------------------------------------------------------------

#[test]
fn the_sign_never_reaches_the_kernel_or_the_log() {
    let entry = fixed_entry();
    let payload = config_bundle_payload_with_targets(&ProfileSyncRules::default(), Some(&entry));
    assert!(!payload.contains("sign-fixture-value"), "sign 不许下发内核");
    assert!(!payload.contains("\"sign\""), "段里不许有 sign 字段");

    // ⛔ Debug 只打印长度 ⇒ `{:?}` 不会泄漏原文。
    let sign = Redacted::new("super-secret-sign");
    let debug = format!("{sign:?}");
    assert!(
        !debug.contains("super-secret-sign") && debug.contains("len="),
        "Debug 只许报长度：{debug}"
    );

    // 短摘要不是原文、也不可逆。
    let digest = short_digest("super-secret-sign");
    assert_ne!(digest, "super-secret-sign");
    assert_eq!(digest.len(), 8);
    assert!(!digest.contains("super"));
}

#[test]
fn the_similarity_threshold_comes_from_the_configuration() {
    // ⛔ 阈值由服务端下发 ⇒ 客户端不许定常量：不同的值都要被接受。
    for threshold in [0u32, 8, 64] {
        let json = format!(
            r#"{{"similarity":{threshold},"targets":[{{"dimension":"canvas","url_regex":"","target_hash":"aabb","width":1,"length":1}}]}}"#
        );
        let set = FpTargetSet::parse_json(&json).expect("应能解析");
        assert_eq!(set.similarity, threshold, "阈值必须来自配置");
    }
}

// ---------------------------------------------------------------------------
// 校验：畸形即整份拒绝
// ---------------------------------------------------------------------------

#[test]
fn a_malformed_target_set_is_refused_not_partially_obeyed() {
    for payload in [
        "{not json",
        r#"{"targets":[]}"#,
        r#"{"similarity":8}"#,
        r#"{"similarity":8,"targets":{}}"#,
        r#"{"similarity":8,"targets":[{"url_regex":"","target_hash":"a","width":1,"length":1}]}"#,
        r#"{"similarity":8,"targets":[{"dimension":"audio","url_regex":"","target_hash":"a","width":1,"length":1}]}"#,
        r#"{"similarity":8,"targets":[{"dimension":"canvas","target_hash":"a","width":1,"length":1}]}"#,
        r#"{"similarity":8,"targets":[{"dimension":"canvas","url_regex":"","width":1,"length":1}]}"#,
        r#"{"similarity":8,"targets":[{"dimension":"canvas","url_regex":"","target_hash":"","width":1,"length":1}]}"#,
        r#"{"similarity":8,"targets":[{"dimension":"canvas","url_regex":"","target_hash":"a","width":0,"length":1}]}"#,
        r#"{"similarity":8,"targets":[{"dimension":"canvas","url_regex":"","target_hash":"a","width":1,"length":"x"}]}"#,
        r#"{"similarity":-1,"targets":[]}"#,
        r#"{"similarity":8,"targets":[{"dimension":"canvas","url_regex":"","target_hash":"a","width":1,"length":1,"skip":"yes"}]}"#,
    ] {
        assert!(
            FpTargetSet::parse_json(payload).is_err(),
            "畸形目标值集 {payload} 必须被拒绝"
        );
    }
}

// ⛔ 缺 `target_hash` 的那一条就在上面的列表里：缺项 ⇒ 整份拒绝 ⇒
// 客户端用旧值、内核退回现有行为（两侧结论一致）。

// ---------------------------------------------------------------------------
// 缓存：加密落盘 / 原子写 / 按环境分目录
// ---------------------------------------------------------------------------

#[test]
fn the_cache_is_encrypted_at_rest_and_scoped_per_environment() {
    let root = temp_dir("cache");
    let cache = FpValueCache::new(&root, test_cache_key());
    let entry = fixed_entry();
    let mut stats = FpValueStats::default();

    assert!(cache.store("env-a", &entry), "应能落盘");
    assert!(cache.store("env-b", &entry), "应能落盘");

    // ⛔ 不许所有环境共用一个文件。
    let path_a = cache.cache_path("env-a").expect("应有路径");
    let path_b = cache.cache_path("env-b").expect("应有路径");
    assert_ne!(path_a, path_b, "两个环境必须是两个文件");
    assert!(path_a.exists() && path_b.exists());

    // ⛔ 落盘的是密文：文件里搜不到明文目标哈希与 sign。
    let bytes = std::fs::read(&path_a).expect("应能读");
    let text = String::from_utf8_lossy(&bytes);
    assert!(
        !text.contains("a1b2c3d4e5f60718293a4b5c6d7e8f90"),
        "缓存文件里不许有明文目标哈希"
    );
    assert!(
        !text.contains("sign-fixture-value"),
        "缓存文件里不许有明文 sign"
    );

    // 读回来必须一模一样。
    let loaded = cache.load("env-a", &mut stats).expect("应能读出");
    assert_eq!(loaded, entry, "往返必须逐字段相同");
    assert_eq!(stats.cache_hits, 1);
    assert_eq!(stats.cache_misses, 0);

    // ⭐ 原子写不留临时文件。
    let leftovers: Vec<_> = std::fs::read_dir(path_a.parent().expect("应有父目录"))
        .expect("应能列目录")
        .filter_map(Result::ok)
        .map(|item| item.file_name().to_string_lossy().to_string())
        .filter(|name| name.ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty(), "不该留下临时文件：{leftovers:?}");

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_corrupted_cache_is_discarded_and_the_browser_can_still_come_up() {
    // ⭐ 验收证据：「缓存损坏 ⇒ 重新拉取且浏览器能起来」。
    let root = temp_dir("corrupt");
    let cache = FpValueCache::new(&root, test_cache_key());
    let mut stats = FpValueStats::default();

    // 先写一份正常的缓存，再把它**破坏**。
    assert!(cache.store("env-a", &fixed_entry()));
    let path = cache.cache_path("env-a").expect("应有路径");
    std::fs::write(&path, b"this-is-not-a-valid-encrypted-cache").expect("应能写坏");

    // 损坏 ⇒ 丢弃（未命中）⇒ **不抛错**（⛔ 不许让浏览器起不来）。
    let loaded = cache.load("env-a", &mut stats);
    assert!(loaded.is_none(), "损坏的缓存必须被丢弃");
    assert_eq!(stats.cache_corrupt, 1, "损坏必须被计数");
    assert_eq!(stats.cache_misses, 1);

    // 重新拉取 ⇒ 拿到好值 ⇒ 配置包照常下发 ⇒ 浏览器能起来。
    let mut transport = FakeTransport::with_inline(VALUE_PAYLOAD);
    let outcome = refresh_values(
        &mut transport,
        &cache,
        "env-a",
        "ct-01",
        1_700_000_000,
        &mut |_line| {},
        &mut stats,
    );
    assert!(outcome.updated, "应当重新拉取并更新");
    let entry = outcome.entry.expect("应拿到目标值");
    let payload = config_bundle_payload_with_targets(&ProfileSyncRules::default(), Some(&entry));
    assert!(
        payload.contains(r#""fp_targets":"#),
        "配置包应照常带目标值段"
    );
    assert!(stats.updates >= 1, "更新必须被计数");

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_cache_written_with_another_key_cannot_be_read_back() {
    // ⛔ 缓存是**本机**的：另一把密钥解不开 ⇒ 当作损坏处理。
    let root = temp_dir("wrongkey");
    let cache = FpValueCache::new(&root, test_cache_key());
    assert!(cache.store("env-a", &fixed_entry()));

    let other = FpValueCache::new(&root, [0x11u8; 32]);
    let mut stats = FpValueStats::default();
    assert!(
        other.load("env-a", &mut stats).is_none(),
        "另一把密钥读不出来"
    );
    assert_eq!(stats.cache_corrupt, 1);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_environment_id_can_not_escape_the_cache_root() {
    // ⛔ 环境标识来自远端配置 ⇒ 不能拼出路径穿越。
    assert_eq!(sanitize_env_id("env-01").as_deref(), Some("env-01"));
    assert_eq!(sanitize_env_id("a/b").as_deref(), Some("a_b"));
    assert_eq!(sanitize_env_id(".."), None);
    assert_eq!(sanitize_env_id("."), None);
    assert_eq!(sanitize_env_id(""), None);
}

// ---------------------------------------------------------------------------
// 获取与续期
// ---------------------------------------------------------------------------

/// 一个可控的假通道。
struct FakeTransport {
    check: Option<FpValueCheck>,
    payload: Option<String>,
    seen_sign: Option<String>,
    fail_request: bool,
    fail_download: bool,
}

impl FakeTransport {
    fn with_inline(payload: &str) -> Self {
        let mut transport = Self {
            check: None,
            payload: Some(payload.to_string()),
            seen_sign: None,
            fail_request: false,
            fail_download: false,
        };
        transport.check = Some(FpValueCheck {
            needs_update: true,
            sign: Some(Redacted::new("server-issued-sign")),
            set: FpTargetSet::parse_json(payload).ok(),
            value_ref: None,
            report_enabled: None,
        });
        transport
    }

    fn failing() -> Self {
        Self {
            check: None,
            payload: None,
            seen_sign: None,
            fail_request: true,
            fail_download: false,
        }
    }
}

impl FpValueTransport for FakeTransport {
    fn request_targets(
        &mut self,
        request: &FpValueRequest,
    ) -> Result<FpValueCheck, FpValueFetchError> {
        self.seen_sign = request.sign.as_ref().map(|sign| sign.expose().to_string());
        if self.fail_request {
            return Err(FpValueFetchError::Transport("平台不可达".to_string()));
        }
        self.check
            .clone()
            .ok_or_else(|| FpValueFetchError::Transport("无答复".to_string()))
    }

    fn download(&mut self, _value_ref: &str) -> Result<String, FpValueFetchError> {
        if self.fail_download {
            return Err(FpValueFetchError::Transport("下载失败".to_string()));
        }
        self.payload
            .clone()
            .ok_or_else(|| FpValueFetchError::Transport("无值载荷".to_string()))
    }
}

#[test]
fn the_refresh_carries_the_cached_sign_back_to_the_server() {
    // 「我已有这份缓存」的凭证 ⇒ 第二次请求必须带上 `sign`。
    let root = temp_dir("sign");
    let cache = FpValueCache::new(&root, test_cache_key());
    let mut stats = FpValueStats::default();

    let mut transport = FakeTransport::with_inline(VALUE_PAYLOAD);
    let _ = refresh_values(
        &mut transport,
        &cache,
        "env-a",
        "ct-01",
        1_700_000_000,
        &mut |_line| {},
        &mut stats,
    );
    assert_eq!(
        transport.seen_sign.as_deref(),
        None,
        "第一次没有缓存 ⇒ 请求不带 sign（⛔ 客户端不自行生成）"
    );

    // 第二次：缓存命中 ⇒ 请求带上缓存里的 sign ⇒ 服务端说不用更新 ⇒
    // **跳过下载**。
    let mut transport = FakeTransport::with_inline(VALUE_PAYLOAD);
    transport.check = Some(FpValueCheck {
        needs_update: false,
        sign: None,
        set: None,
        value_ref: None,
        report_enabled: None,
    });
    let outcome = refresh_values(
        &mut transport,
        &cache,
        "env-a",
        "ct-01",
        1_700_000_001,
        &mut |_line| {},
        &mut stats,
    );
    assert!(!outcome.updated, "服务端说不用更新 ⇒ 不更新");
    assert_eq!(
        transport.seen_sign.as_deref(),
        Some("server-issued-sign"),
        "缓存命中时必须带上 sign"
    );
    assert!(outcome.entry.is_some(), "旧值继续生效");

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_failed_fetch_never_blocks_and_keeps_the_old_values() {
    // ⛔ 拉取失败 ⇒ 记日志 + 计数 ⇒ 用旧值继续（不许失败就崩）。
    let root = temp_dir("failfetch");
    let cache = FpValueCache::new(&root, test_cache_key());
    let mut stats = FpValueStats::default();
    assert!(cache.store("env-a", &fixed_entry()));

    let mut transport = FakeTransport::failing();
    let mut lines: Vec<String> = Vec::new();
    let outcome = refresh_values(
        &mut transport,
        &cache,
        "env-a",
        "ct-01",
        1_700_000_002,
        &mut |line| lines.push(line.to_string()),
        &mut stats,
    );
    assert!(!outcome.updated);
    assert!(outcome.entry.is_some(), "旧值必须继续生效");
    assert!(
        lines.iter().any(|line| line.contains("拉取失败")),
        "失败必须记日志：{lines:?}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_malformed_server_answer_is_refused() {
    for json in [
        "{not json",
        r#"{}"#,
        r#"{"needs_update":"yes"}"#,
        r#"{"needs_update":true,"sign":42}"#,
        r#"{"needs_update":true,"value_ref":7}"#,
    ] {
        assert!(
            parse_check_json(json).is_err(),
            "畸形答复 {json} 必须被拒绝"
        );
    }
    let ok = parse_check_json(
        r#"{"needs_update":true,"sign":"s","similarity":8,"targets":[{"dimension":"canvas","url_regex":"","target_hash":"aabb","width":1,"length":1}]}"#,
    )
    .expect("应能解析");
    assert!(ok.needs_update);
    assert!(ok.set.is_some(), "内联目标值应被解析出来");
    assert_eq!(ok.set.expect("应有目标值").similarity, 8);
}

// ---------------------------------------------------------------------------
// 回传：默认关闭 / 默认不采 data_url / 失败不阻塞
// ---------------------------------------------------------------------------

#[test]
fn the_default_path_never_reports_a_data_url() {
    // ⭐ 验收证据：「值默认不采 data_url」。
    // 事件里**带了** `data_url` ⇒ 解析结果里没有它 ⇒ 载荷里也没有它。
    let payload = br#"{"from":"renderer","action":"CanvasSample","profile":"p","width":300,"length":150,"url":"https://example.com/","drew_text":true,"hash":"abcdef","data_url":"data:image/png;base64,AAAA"}"#;
    let sample = parse_sample(payload).expect("应能解析");
    let batch = FpReportBatch {
        container_id: "ct-01".to_string(),
        env_id: "env-a".to_string(),
        sign: Some(Redacted::new("server-issued-sign")),
        samples: vec![sample],
    };
    let text = batch.to_json();
    let leaked = ["data_url", "data:image", "base64"]
        .iter()
        .any(|k| text.contains(k));
    assert!(!leaked, "位图不许被回传：{text}");
}

#[test]
fn reporting_is_off_by_default_and_only_affects_this_environment() {
    // ⛔ 默认关闭 ⇒ 什么都不发；开启只影响**本环境**（开关存在本环境的
    // 缓存条目里）。
    let batch = FpReportBatch {
        container_id: "ct-01".to_string(),
        env_id: "env-a".to_string(),
        sign: None,
        samples: vec![],
    };
    let mut stats = FpValueStats::default();
    let mut reporter = CountingReporter {
        calls: 0,
        fail: false,
    };
    let sent = report_samples(&mut reporter, &batch, false, &mut |_line| {}, &mut stats);
    assert!(!sent, "默认关闭 ⇒ 不该发送");
    assert_eq!(reporter.calls, 0, "默认关闭 ⇒ 一次都不该发");
    assert_eq!(stats.report_skipped, 1);

    // 默认的缓存条目：回采关闭。
    assert!(!fixed_entry().report_enabled, "默认必须是关闭");
}

#[test]
fn a_failed_report_never_blocks_the_close() {
    // 拍板 2A：回传失败**不阻塞**容器关闭 ⇒ 只记日志 + 计数。
    let batch = FpReportBatch {
        container_id: "ct-01".to_string(),
        env_id: "env-a".to_string(),
        sign: None,
        samples: vec![],
    };
    let mut stats = FpValueStats::default();
    let mut lines: Vec<String> = Vec::new();
    let mut reporter = CountingReporter {
        calls: 0,
        fail: true,
    };
    let sent = report_samples(
        &mut reporter,
        &batch,
        true,
        &mut |line| lines.push(line.to_string()),
        &mut stats,
    );
    assert!(!sent, "失败不该报成功");
    assert_eq!(stats.report_failures, 1, "失败必须被计数");
    assert_eq!(stats.report_success, 0);
    assert!(
        lines.iter().any(|line| line.contains("回传失败")),
        "失败必须记日志：{lines:?}"
    );

    // 默认回传器**诚实地失败**（计数不许撒谎）。
    let mut stats = FpValueStats::default();
    let mut reporter = UnavailableReporter;
    let sent = report_samples(&mut reporter, &batch, true, &mut |_line| {}, &mut stats);
    assert!(!sent);
    assert_eq!(stats.report_success, 0, "没有通道就不许说成功");
    assert_eq!(stats.report_failures, 1);
}

#[test]
fn malformed_or_non_sample_events_are_dropped_and_counted() {
    let mut collector = SampleCollector::new();
    collector.push(br#"{"action":"PageEvent","url":"https://example.com/"}"#);
    collector.push(br#"{"action":"Heartbeat","ts_unix":1}"#);
    collector.push(b"{not json");
    collector.push(br#"{"action":"CanvasSample","url":"https://example.com/","hash":"a"}"#);
    assert_eq!(collector.dropped(), 4, "非采样事件与坏事件必须被丢弃并计数");
    assert!(collector.samples().is_empty());

    // 必填字段齐全的采样事件才收。
    collector.push(
        br#"{"action":"CanvasSample","profile":"p","width":300,"length":150,"url":"https://example.com/","drew_text":true,"hash":"abcdef"}"#,
    );
    assert_eq!(collector.samples().len(), 1);
    assert_eq!(collector.dropped(), 4, "好事件不许影响丢弃计数");

    // `WebGLSample` 没有 `drew_text`。
    let webgl = parse_sample(
        br#"{"action":"WebGLSample","profile":"p","width":512,"length":512,"url":"https://example.com/","hash":"abcdef"}"#,
    )
    .expect("应能解析");
    assert_eq!(webgl.dimension, FpDimension::Webgl);
    assert_eq!(webgl.drew_text, None);
}

/// 记调用次数的回传器（可模拟失败）。
struct CountingReporter {
    calls: usize,
    fail: bool,
}

impl FpSampleReporter for CountingReporter {
    fn report(&mut self, _batch: &FpReportBatch) -> Result<(), FpReportError> {
        self.calls += 1;
        if self.fail {
            Err(FpReportError::Transport("平台不可达".to_string()))
        } else {
            Ok(())
        }
    }
}

#[test]
fn a_sample_without_the_required_fields_is_refused() {
    for payload in [
        &br#"{"action":"CanvasSample","url":"u","hash":"a"}"#[..],
        &br#"{"action":"CanvasSample","width":300,"length":150,"hash":"a"}"#[..],
        &br#"{"action":"CanvasSample","width":300,"length":150,"url":"u"}"#[..],
        &br#"{"action":"CanvasSample","width":"x","length":150,"url":"u","hash":"a"}"#[..],
        &br#"{"action":"CanvasSample","width":300,"length":150,"url":"u","hash":""}"#[..],
    ] {
        assert!(
            parse_sample(payload).is_err(),
            "缺必填字段的事件必须被拒绝：{}",
            String::from_utf8_lossy(payload)
        );
    }
    assert_eq!(
        parse_sample(br#"{"action":"PageEvent","url":"u"}"#),
        Err(FpSampleError::NotASample)
    );
}

#[test]
fn a_sample_batch_is_built_from_the_collected_events() {
    let mut collector = SampleCollector::new();
    collector.push(
        br#"{"action":"CanvasSample","profile":"p","width":300,"length":150,"url":"https://example.com/","drew_text":true,"hash":"abcdef"}"#,
    );
    collector.push(
        br#"{"action":"WebGLSample","profile":"p","width":512,"length":512,"url":"https://example.com/","hash":"123456"}"#,
    );
    let samples: Vec<FpSample> = collector.samples().to_vec();
    assert_eq!(samples.len(), 2);
    let batch = FpReportBatch {
        container_id: "ct-01".to_string(),
        env_id: "env-a".to_string(),
        sign: None,
        samples,
    };
    let text = batch.to_json();
    assert!(text.contains("\"dimension\":\"canvas\""));
    assert!(text.contains("\"dimension\":\"webgl\""));
    assert!(!text.contains("data_url"));
}
