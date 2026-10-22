//! 指纹值治理（客户端侧）：**目标值模型、校验、下发**（纯逻辑，不碰 IO）。
//!
//! # 分工（对齐内核侧 T3）
//!
//! - **服务端生产目标值**（per-URL 的 `target_hash` 等），客户端**只搬运**。
//! - 本模块只做三件事：**解析/校验**远端下发的目标值、**渲染**成
//!   `ConfigBundle` 的 `fp_targets` 段、**统计**治理链路的计数。
//!
//! # ⛔ 边界
//!
//! - ⛔ **客户端不生产指纹值**：源码里不许出现任何具体的目标哈希。
//! - ⛔ **值不进命令行**：只走既有 `0x03 ConfigBundle` 的 JSON 段
//!   （⛔ 不新增帧类型）。
//! - ⛔ **`sign` 不进 `ConfigBundle`**：它是客户端↔服务端的会话凭证，
//!   内核不需要它、也不该看到它（见 [`Redacted`]）。
//! - ⛔ **日志不得打印 `sign` 原文与目标哈希原文** ⇒ 二者都用
//!   [`Redacted`] 包起来，其 `Debug` 只打印长度。
//!
//! # 与内核侧的字段对齐
//!
//! 一条目标值的字段与内核侧 T3 的目标值结构**逐字段一致**：
//! `dimension` / `url_regex` / `target_hash` / `width` / `length` /
//! `drew_text` / `skip`。其中 `drew_text` 与 `skip` 可缺省。
//!
//! ⛔ **畸形即整份拒绝**：一条目标值缺 `target_hash`（或字段类型不对）
//! ⇒ 整份目标值集作废。客户端作废 ⇒ 丢弃 + 计数 + 用旧值；
//! 内核侧作废 ⇒ 没有目标值 ⇒ 退回现有行为。**两侧结论一致。**

use crate::kernel_pipe::keys::to_hex;
use crate::kernel_pipe::upload::{config_bundle_payload, ProfileSyncRules};
use crate::kernel_pipe::{Frame, FrameType};
use sha2::{Digest, Sha256};

/// `ConfigBundle` 里目标值段的段名。
///
/// 与内核侧 T3 的段名逐字节相同（跨仓契约钉死）。
pub const FP_TARGETS_KEY: &str = "fp_targets";

// ---------------------------------------------------------------------------
// 不进日志的字符串
// ---------------------------------------------------------------------------

/// ⛔ **不进日志**的字符串（会话凭证 / 目标哈希）。
///
/// `Debug` 只打印长度 ⇒ 把它塞进任何 `{:?}` 都不会泄漏原文。
/// 真正要用原文的地方必须显式调 [`Redacted::expose`]。
#[derive(Clone, PartialEq, Eq)]
pub struct Redacted(String);

impl Redacted {
    /// 包一层。
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// ⛔ **取原文**——只在真的要发出去的地方调（服务端请求 / 下发载荷）。
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// 原文长度（字节）。
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// 原文是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Debug for Redacted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<redacted len={}>", self.0.len())
    }
}

/// 日志里用的**短摘要**（原文的前 8 位十六进制 SHA-256）。
///
/// ⛔ 不是原文，也**不可逆** ⇒ 可以进日志。
#[must_use]
pub fn short_digest(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    to_hex(&digest[..4])
}

// ---------------------------------------------------------------------------
// 维度与目标值
// ---------------------------------------------------------------------------

/// 指纹维度。
///
/// 线上取值只有 `"canvas"` 与 `"webgl"` 两个（与内核侧 T3 一致）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FpDimension {
    /// `canvas`。
    Canvas,
    /// `webgl`。
    Webgl,
}

impl FpDimension {
    /// 线上取值。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Canvas => "canvas",
            Self::Webgl => "webgl",
        }
    }

    /// 解析线上取值；不是已知维度 ⇒ `None`。
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "canvas" => Some(Self::Canvas),
            "webgl" => Some(Self::Webgl),
            _ => None,
        }
    }
}

/// 一条目标值。
///
/// ⛔ 字段与内核侧 T3 的目标值结构**逐字段一致**（见模块头）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpTarget {
    /// 维度。
    pub dimension: FpDimension,
    /// URL 定向正则。**空串** = 该维度的默认目标。
    pub url_regex: String,
    /// 期望的指纹哈希（⛔ 不进日志）。
    pub target_hash: Redacted,
    /// 采样宽度。
    pub width: u32,
    /// 采样长度。
    pub length: u32,
    /// 是否画字（可缺省）。
    pub drew_text: Option<bool>,
    /// 该 URL 是否跳过扰动（可缺省）。
    pub skip: Option<bool>,
}

/// 一份目标值集（**服务端下发**的形状）。
///
/// 形状 = `{"similarity": N, "targets": [...]}`。
///
/// ⛔ `similarity` 是**汉明距离阈值**，由服务端下发（跨仓契约第 4 节）
/// ⇒ 客户端与内核都**不许**各自定一个常量。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpTargetSet {
    /// 汉明距离阈值。
    pub similarity: u32,
    /// 目标值条目。
    pub targets: Vec<FpTarget>,
}

impl FpTargetSet {
    /// 一份**空**目标值集（没有目标 ⇒ 不下发该段）。
    #[must_use]
    pub fn empty() -> Self {
        Self {
            similarity: 0,
            targets: Vec::new(),
        }
    }

    /// 从 JSON 文本解析（服务端下发 / 值载荷下载回来的形状）。
    pub fn parse_json(json: &str) -> Result<Self, FpValueError> {
        let value: serde_json::Value =
            serde_json::from_str(json).map_err(|_| FpValueError::MalformedJson)?;
        Self::parse_value(&value)
    }

    /// 从已解析的 JSON 值解析。
    ///
    /// ⛔ **畸形即整份拒绝**（不做「部分遵守」）：一条目标值不合格 ⇒
    /// 整份作废。
    pub fn parse_value(value: &serde_json::Value) -> Result<Self, FpValueError> {
        let object = value.as_object().ok_or(FpValueError::MalformedStructure)?;
        let similarity = read_u32(
            object
                .get("similarity")
                .ok_or(FpValueError::MalformedStructure)?,
        )?;
        let array = object
            .get("targets")
            .ok_or(FpValueError::MalformedStructure)?
            .as_array()
            .ok_or(FpValueError::MalformedStructure)?;
        let mut targets = Vec::with_capacity(array.len());
        for item in array {
            targets.push(FpTarget::parse_value(item)?);
        }
        Ok(Self {
            similarity,
            targets,
        })
    }
}

/// 目标值 / 缓存条目的解析失败。
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FpValueError {
    /// 不是合法 JSON。
    #[error("目标值载荷不是合法 JSON")]
    MalformedJson,
    /// JSON 合法但结构不合法（字段缺失、类型不对、取值非法）。
    #[error("目标值结构不合法")]
    MalformedStructure,
}

impl FpTarget {
    /// 解析一条目标值。
    fn parse_value(value: &serde_json::Value) -> Result<Self, FpValueError> {
        let object = value.as_object().ok_or(FpValueError::MalformedStructure)?;

        let dimension_text = object
            .get("dimension")
            .and_then(serde_json::Value::as_str)
            .ok_or(FpValueError::MalformedStructure)?;
        let dimension =
            FpDimension::parse(dimension_text).ok_or(FpValueError::MalformedStructure)?;

        let url_regex = object
            .get("url_regex")
            .and_then(serde_json::Value::as_str)
            .ok_or(FpValueError::MalformedStructure)?;

        let hash_text = object
            .get("target_hash")
            .and_then(serde_json::Value::as_str)
            .ok_or(FpValueError::MalformedStructure)?;
        if hash_text.is_empty() {
            return Err(FpValueError::MalformedStructure);
        }

        let width = read_u32(
            object
                .get("width")
                .ok_or(FpValueError::MalformedStructure)?,
        )?;
        let length = read_u32(
            object
                .get("length")
                .ok_or(FpValueError::MalformedStructure)?,
        )?;
        if width == 0 || length == 0 {
            return Err(FpValueError::MalformedStructure);
        }

        Ok(Self {
            dimension,
            url_regex: url_regex.to_string(),
            target_hash: Redacted::new(hash_text),
            width,
            length,
            drew_text: read_optional_bool(object.get("drew_text"))?,
            skip: read_optional_bool(object.get("skip"))?,
        })
    }
}

/// 读一个 `u32`（负数 / 浮点 / 超出范围 ⇒ 畸形）。
fn read_u32(value: &serde_json::Value) -> Result<u32, FpValueError> {
    let number = value.as_u64().ok_or(FpValueError::MalformedStructure)?;
    u32::try_from(number).map_err(|_| FpValueError::MalformedStructure)
}

/// 读一个可选的 `bool`（缺省 ⇒ `None`；类型不对 ⇒ 畸形）。
fn read_optional_bool(value: Option<&serde_json::Value>) -> Result<Option<bool>, FpValueError> {
    match value {
        None => Ok(None),
        Some(item) => item
            .as_bool()
            .map(Some)
            .ok_or(FpValueError::MalformedStructure),
    }
}

// ---------------------------------------------------------------------------
// 缓存条目（客户端本地）
// ---------------------------------------------------------------------------

/// **一个环境的值缓存条目**（加密落盘的内容）。
///
/// 形状：`{sign, container_id, updated_at_unix, similarity, report_enabled,
/// targets[]}`。
///
/// - `sign`：**服务端签发**的「我已有这份缓存」凭证（⛔ 客户端不得自行生成）。
/// - `similarity`：汉明距离阈值（服务端下发，⛔ 不许客户端定常量）。
/// - `report_enabled`：回采上报开关（**默认关闭**，只影响本环境）。
/// - `targets[]`：目标值条目（字段与内核侧逐字段一致）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpValueCacheEntry {
    /// 会话凭证（⛔ 不进日志、不进 `ConfigBundle`）。
    pub sign: Option<Redacted>,
    /// 容器标识。
    pub container_id: String,
    /// 更新时间（Unix 秒）。
    pub updated_at_unix: u64,
    /// 汉明距离阈值。
    pub similarity: u32,
    /// 回采上报是否开启（**默认关闭**）。
    pub report_enabled: bool,
    /// 目标值条目。
    pub targets: Vec<FpTarget>,
}

impl FpValueCacheEntry {
    /// 缓存条目的 JSON 文本（**加密前**的明文）。
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push_str("{\"sign\":");
        match &self.sign {
            Some(sign) => {
                out.push('"');
                out.push_str(&json_escape(sign.expose()));
                out.push('"');
            }
            None => out.push_str("null"),
        }
        out.push_str(",\"container_id\":\"");
        out.push_str(&json_escape(&self.container_id));
        out.push_str("\",\"updated_at_unix\":");
        out.push_str(&self.updated_at_unix.to_string());
        out.push_str(",\"similarity\":");
        out.push_str(&self.similarity.to_string());
        out.push_str(",\"report_enabled\":");
        out.push_str(if self.report_enabled { "true" } else { "false" });
        out.push_str(",\"targets\":");
        push_targets(&mut out, &self.targets);
        out.push('}');
        out
    }

    /// 解析缓存条目的 JSON 文本。
    pub fn parse_json(json: &str) -> Result<Self, FpValueError> {
        let value: serde_json::Value =
            serde_json::from_str(json).map_err(|_| FpValueError::MalformedJson)?;
        let object = value.as_object().ok_or(FpValueError::MalformedStructure)?;

        let sign = match object.get("sign") {
            None | Some(serde_json::Value::Null) => None,
            Some(item) => {
                let text = item.as_str().ok_or(FpValueError::MalformedStructure)?;
                if text.is_empty() {
                    None
                } else {
                    Some(Redacted::new(text))
                }
            }
        };
        let container_id = object
            .get("container_id")
            .and_then(serde_json::Value::as_str)
            .ok_or(FpValueError::MalformedStructure)?
            .to_string();
        let updated_at_unix = object
            .get("updated_at_unix")
            .and_then(serde_json::Value::as_u64)
            .ok_or(FpValueError::MalformedStructure)?;
        let similarity = read_u32(
            object
                .get("similarity")
                .ok_or(FpValueError::MalformedStructure)?,
        )?;
        let report_enabled = object
            .get("report_enabled")
            .and_then(serde_json::Value::as_bool)
            .ok_or(FpValueError::MalformedStructure)?;
        let array = object
            .get("targets")
            .ok_or(FpValueError::MalformedStructure)?
            .as_array()
            .ok_or(FpValueError::MalformedStructure)?;
        let mut targets = Vec::with_capacity(array.len());
        for item in array {
            targets.push(FpTarget::parse_value(item)?);
        }

        Ok(Self {
            sign,
            container_id,
            updated_at_unix,
            similarity,
            report_enabled,
            targets,
        })
    }
}

// ---------------------------------------------------------------------------
// 下发（ConfigBundle）
// ---------------------------------------------------------------------------

/// 目标值段（`fp_targets`）的 JSON —— **下发给内核的那一段**。
///
/// 键序固定：`updated_at_unix` → `similarity` → `report_enabled` → `targets`。
///
/// ⛔ 段里**没有** `sign`（会话凭证不下发内核）。
/// ⛔ 段里**没有** `data_url`（拍板 3A：默认不采）。
#[must_use]
pub fn fp_targets_section_json(entry: &FpValueCacheEntry) -> String {
    let mut out = String::new();
    out.push_str("{\"updated_at_unix\":");
    out.push_str(&entry.updated_at_unix.to_string());
    out.push_str(",\"similarity\":");
    out.push_str(&entry.similarity.to_string());
    out.push_str(",\"report_enabled\":");
    out.push_str(if entry.report_enabled {
        "true"
    } else {
        "false"
    });
    out.push_str(",\"targets\":");
    push_targets(&mut out, &entry.targets);
    out.push('}');
    out
}

/// 整份 `ConfigBundle` 的载荷文本（**含** `profile_sync` 与 `fp_targets` 两段）。
///
/// - `entry` 为 `None` 或**目标值为空** ⇒ **不下发** `fp_targets` 段
///   ⇒ 内核侧退回现有行为。
/// - `profile_sync` 段的字节**逐字不变**（复用 [`config_bundle_payload`]），
///   ⇒ 既有跨仓固定向量继续成立。
#[must_use]
pub fn config_bundle_payload_with_targets(
    rules: &ProfileSyncRules,
    entry: Option<&FpValueCacheEntry>,
) -> String {
    let base = config_bundle_payload(rules);
    let Some(entry) = entry else {
        return base;
    };
    if entry.targets.is_empty() {
        return base;
    }
    // base 一定是 `{"profile_sync":{…}}` ⇒ 去掉末尾的 `}` 再续一段。
    if !base.ends_with('}') {
        return base;
    }
    let mut out = String::with_capacity(base.len() + 512);
    out.push_str(&base[..base.len() - 1]);
    out.push_str(",\"");
    out.push_str(FP_TARGETS_KEY);
    out.push_str("\":");
    out.push_str(&fp_targets_section_json(entry));
    out.push('}');
    out
}

/// 整份 `ConfigBundle` 帧（**含**目标值段）。
#[must_use]
pub fn config_bundle_frame_with_targets(
    rules: &ProfileSyncRules,
    entry: Option<&FpValueCacheEntry>,
) -> Frame {
    Frame::new(
        FrameType::ConfigBundle,
        config_bundle_payload_with_targets(rules, entry).into_bytes(),
    )
}

/// 把一组目标值写进 JSON 数组。
fn push_targets(out: &mut String, targets: &[FpTarget]) {
    out.push('[');
    for (index, target) in targets.iter().enumerate() {
        if index != 0 {
            out.push(',');
        }
        out.push_str("{\"dimension\":\"");
        out.push_str(target.dimension.as_str());
        out.push_str("\",\"url_regex\":\"");
        out.push_str(&json_escape(&target.url_regex));
        out.push('"');
        out.push_str(",\"target_hash\":\"");
        out.push_str(&json_escape(target.target_hash.expose()));
        out.push('"');
        out.push_str(",\"width\":");
        out.push_str(&target.width.to_string());
        out.push_str(",\"length\":");
        out.push_str(&target.length.to_string());
        if let Some(drew_text) = target.drew_text {
            out.push_str(",\"drew_text\":");
            out.push_str(if drew_text { "true" } else { "false" });
        }
        if let Some(skip) = target.skip {
            out.push_str(",\"skip\":");
            out.push_str(if skip { "true" } else { "false" });
        }
        out.push('}');
    }
    out.push(']');
}

/// 极简 JSON 字符串转义（与 `upload.rs` 的同名助手同算法）。
fn json_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// 观测
// ---------------------------------------------------------------------------

/// 值治理链路的计数。
///
/// ⛔ 这里**只有计数**，没有原文：`sign` 与目标哈希都不进日志。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FpValueStats {
    /// 缓存命中次数。
    pub cache_hits: u64,
    /// 缓存未命中次数（含「损坏 ⇒ 丢弃」）。
    pub cache_misses: u64,
    /// 缓存损坏并被丢弃的次数。
    pub cache_corrupt: u64,
    /// 值更新次数（成功落盘）。
    pub updates: u64,
    /// 校验失败次数（目标值结构不合法）。
    pub validation_failures: u64,
    /// 回传成功次数。
    pub report_success: u64,
    /// 回传失败次数（⛔ 不阻塞容器关闭）。
    pub report_failures: u64,
    /// 因开关关闭而**未**回传的次数。
    pub report_skipped: u64,
}

/// 当前 Unix 秒。
#[must_use]
pub fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
