//! 指纹值的**回传**：聚合内核上报的采样结果 ⇒ 送服务端。
//!
//! # 拍板
//!
//! - **3A**：`data_url`（画布位图）**默认不采** ⇒ 本模块**根本不读**它：
//!   [`FpSample`] **没有**这个字段 ⇒ 它不可能被回传。
//! - **2A**：回传失败**不阻塞**容器关闭 ⇒ 失败只记日志 + 计数。
//! - ⛔ 回采**默认关闭**；开启开关**只影响本环境**。
//!
//! # 通道
//!
//! 内核经**既有上行 action 表**上报采样结果（`CanvasSample` /
//! `WebGLSample`）⇒ 本模块解析、聚合成一批 ⇒ 交给**可注入**的回传通道。
//!
//! ⛔ 本仓**不实现**平台侧的 HTTPS 客户端（同 [`crate::kernel_pipe::upload`]）。

use crate::kernel_pipe::fp_values::{FpDimension, FpValueStats, Redacted};

/// 一条采样结果（内核回采到的实际指纹）。
///
/// ⛔ **没有** `data_url` 字段 ⇒ 位图不可能被回传（拍板 3A）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpSample {
    /// 维度（canvas / webgl）。
    pub dimension: FpDimension,
    /// 页面 URL。
    pub url: String,
    /// 采样宽度。
    pub width: u32,
    /// 采样长度。
    pub length: u32,
    /// 采样得到的指纹哈希（⛔ 不进日志）。
    pub hash: Redacted,
    /// 是否画字（`CanvasSample` 有；`WebGLSample` 无）。
    pub drew_text: Option<bool>,
}

/// 采样解析失败。
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FpSampleError {
    /// 不是合法 JSON。
    #[error("采样事件不是合法 JSON")]
    NotJson,
    /// 不是采样类 action（`PageEvent` / `Heartbeat` 等）。
    #[error("不是采样事件")]
    NotASample,
    /// 必填字段缺失。
    #[error("采样事件缺必填字段")]
    MissingField,
    /// 字段类型不对。
    #[error("采样事件字段类型不对")]
    BadField,
}

/// 一批要回传的采样结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpReportBatch {
    /// 容器标识。
    pub container_id: String,
    /// 环境标识（开关只影响本环境 ⇒ 批次里带上环境）。
    pub env_id: String,
    /// 会话凭证（⛔ 不进日志）。
    pub sign: Option<Redacted>,
    /// 采样结果。
    pub samples: Vec<FpSample>,
}

impl FpReportBatch {
    /// 批次载荷的 JSON 文本。
    ///
    /// ⛔ 载荷里**没有** `data_url`（拍板 3A）。
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push_str("{\"container_id\":\"");
        out.push_str(&json_escape(&self.container_id));
        out.push_str("\",\"env_id\":\"");
        out.push_str(&json_escape(&self.env_id));
        out.push_str("\",\"sign\":");
        match &self.sign {
            Some(sign) => {
                out.push('"');
                out.push_str(&json_escape(sign.expose()));
                out.push('"');
            }
            None => out.push_str("null"),
        }
        out.push_str(",\"samples\":[");
        for (index, sample) in self.samples.iter().enumerate() {
            if index != 0 {
                out.push(',');
            }
            out.push_str("{\"dimension\":\"");
            out.push_str(sample.dimension.as_str());
            out.push_str("\",\"url\":\"");
            out.push_str(&json_escape(&sample.url));
            out.push_str("\",\"width\":");
            out.push_str(&sample.width.to_string());
            out.push_str(",\"length\":");
            out.push_str(&sample.length.to_string());
            out.push_str(",\"hash\":\"");
            out.push_str(&json_escape(sample.hash.expose()));
            out.push('"');
            if let Some(drew_text) = sample.drew_text {
                out.push_str(",\"drew_text\":");
                out.push_str(if drew_text { "true" } else { "false" });
            }
            out.push('}');
        }
        out.push_str("]}");
        out
    }
}

/// 回传通道（**可注入**）。
pub trait FpSampleReporter {
    /// 把一批采样结果送回服务端。
    ///
    /// 返回 `Err` 表示回传失败。**调用方不许因此阻塞容器关闭**（拍板 2A）。
    fn report(&mut self, batch: &FpReportBatch) -> Result<(), FpReportError>;
}

/// 回传失败。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FpReportError {
    /// 传输失败。
    #[error("回传失败：{0}")]
    Transport(String),
}

/// 通道**尚未接入**时的默认回传器：**诚实地失败**。
///
/// ⛔ **不返回 `Ok`**：否则计数会说「回传了 N 次」而实际一个字节都没
/// 出去 ⇒ **计数撒谎**。（与 `upload::UnavailableUploader` 同一做法。）
#[derive(Debug, Clone, Copy, Default)]
pub struct UnavailableReporter;

impl FpSampleReporter for UnavailableReporter {
    fn report(&mut self, _batch: &FpReportBatch) -> Result<(), FpReportError> {
        Err(FpReportError::Transport("回传通道尚未接入".to_string()))
    }
}

/// 采样结果的收集器（顺带数「丢弃了多少条」）。
#[derive(Debug, Clone, Default)]
pub struct SampleCollector {
    samples: Vec<FpSample>,
    dropped: u64,
}

impl SampleCollector {
    /// 空收集器。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 收一条上行事件的载荷。
    ///
    /// 不是采样事件 / 字段缺失 / 类型不对 ⇒ **丢弃 + 计数**（不发送）。
    pub fn push(&mut self, payload: &[u8]) {
        match parse_sample(payload) {
            Ok(sample) => self.samples.push(sample),
            Err(_) => self.dropped += 1,
        }
    }

    /// 收进来的采样结果。
    #[must_use]
    pub fn samples(&self) -> &[FpSample] {
        &self.samples
    }

    /// 被丢弃的事件数。
    #[must_use]
    pub fn dropped(&self) -> u64 {
        self.dropped
    }
}

/// 回传一批采样结果。
///
/// - `enabled = false` ⇒ **什么都不做**（默认关闭；开启只影响本环境）。
/// - 失败 ⇒ 记日志 + 计数 ⇒ **返回 `false`，但绝不阻塞容器关闭**（拍板 2A）。
#[must_use]
pub fn report_samples(
    reporter: &mut dyn FpSampleReporter,
    batch: &FpReportBatch,
    enabled: bool,
    log: &mut dyn FnMut(&str),
    stats: &mut FpValueStats,
) -> bool {
    if !enabled {
        stats.report_skipped += 1;
        return false;
    }
    match reporter.report(batch) {
        Ok(()) => {
            stats.report_success += 1;
            true
        }
        Err(err) => {
            // ⛔ 2A：失败是静默的，只靠日志/计数发现。容器照常关闭。
            stats.report_failures += 1;
            log(&format!(
                "[lengsuo-desktop] 值治理：回传失败（{err}），不影响容器关闭"
            ));
            false
        }
    }
}

/// 解析一条采样事件。
///
/// ⛔ **`data_url` 根本不读** ⇒ 位图不可能进回传载荷（拍板 3A）。
pub fn parse_sample(payload: &[u8]) -> Result<FpSample, FpSampleError> {
    let value: serde_json::Value =
        serde_json::from_slice(payload).map_err(|_| FpSampleError::NotJson)?;
    let object = value.as_object().ok_or(FpSampleError::NotJson)?;

    let action = object
        .get("action")
        .and_then(serde_json::Value::as_str)
        .ok_or(FpSampleError::MissingField)?;
    let dimension = match action {
        "CanvasSample" => FpDimension::Canvas,
        "WebGLSample" => FpDimension::Webgl,
        _ => return Err(FpSampleError::NotASample),
    };

    let width = read_u32(object.get("width"))?;
    let length = read_u32(object.get("length"))?;
    let url = object
        .get("url")
        .and_then(serde_json::Value::as_str)
        .ok_or(FpSampleError::MissingField)?;
    let hash = object
        .get("hash")
        .and_then(serde_json::Value::as_str)
        .ok_or(FpSampleError::MissingField)?;
    if hash.is_empty() {
        return Err(FpSampleError::BadField);
    }

    // `WebGLSample` 没有 `drew_text` ⇒ 只在 canvas 事件里读它。
    let drew_text = match dimension {
        FpDimension::Canvas => match object.get("drew_text") {
            None | Some(serde_json::Value::Null) => None,
            Some(item) => Some(item.as_bool().ok_or(FpSampleError::BadField)?),
        },
        FpDimension::Webgl => None,
    };

    Ok(FpSample {
        dimension,
        url: url.to_string(),
        width,
        length,
        hash: Redacted::new(hash),
        drew_text,
    })
}

/// 读一个 `u32` 字段（缺 ⇒ `MissingField`；类型不对 ⇒ `BadField`）。
fn read_u32(value: Option<&serde_json::Value>) -> Result<u32, FpSampleError> {
    let number = value
        .ok_or(FpSampleError::MissingField)?
        .as_u64()
        .ok_or(FpSampleError::BadField)?;
    u32::try_from(number).map_err(|_| FpSampleError::BadField)
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
