//! 指纹值的**获取与续期**：带 `sign` 请求 ⇒ 判断是否要更新 ⇒ 落缓存。
//!
//! # 流程
//!
//! 1. 读本地缓存（命中 ⇒ 拿到 `sign`；未命中 ⇒ 没有 `sign`）。
//! 2. 带 `sign`（有的话）问服务端：「要不要更新？目标值是什么？」
//! 3. 服务端说**不用更新** ⇒ 用缓存里的旧值（**跳过下载**）。
//! 4. 服务端说**要更新** ⇒ 目标值可能**内联**在答复里，也可能给一个
//!    **值载荷句柄** ⇒ 下载后解析。
//! 5. **校验**通过 ⇒ 落缓存（加密 + 原子）⇒ 并进 `ConfigBundle` 下发内核。
//!
//! # ⛔ 边界
//!
//! - ⛔ **客户端不生产目标值**：目标值只有服务端能给。
//! - ⛔ **`sign` 只回传不自己算**：客户端**不得**自行生成。
//! - ⛔ **值不走命令行**：只走 `ConfigBundle` 的 JSON 段。
//! - ⛔ **失败不阻塞**：拉取失败 ⇒ 记日志 + 计数 ⇒ 用旧值继续。

use crate::kernel_pipe::fp_values::{FpTargetSet, FpValueCacheEntry, FpValueStats, Redacted};
use crate::kernel_pipe::fp_values_cache::FpValueCache;

/// 值获取 / 续期的失败原因。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FpValueFetchError {
    /// 传输失败（网络 / 平台不可达）。
    #[error("值获取传输失败：{0}")]
    Transport(String),
    /// 服务端答复或值载荷的结构不合法。
    #[error("值获取内容畸形：{0}")]
    Malformed(String),
}

/// 向服务端要目标值的一次请求。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpValueRequest {
    /// 容器标识。
    pub container_id: String,
    /// 环境标识。
    pub env_id: String,
    /// 本地已有的会话凭证（⛔ 客户端不生成，只回传）。
    pub sign: Option<Redacted>,
}

/// 服务端的答复。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpValueCheck {
    /// 要不要更新。
    pub needs_update: bool,
    /// 服务端签发的会话凭证（⛔ 不进日志、不下发内核）。
    pub sign: Option<Redacted>,
    /// 内联的目标值集（服务端直接给了目标值时）。
    pub set: Option<FpTargetSet>,
    /// 值载荷句柄（服务端要客户端**下载**目标值时）。
    pub value_ref: Option<String>,
    /// 回采上报开关（服务端策略；缺省 ⇒ 沿用本地缓存里的值 ⇒ 再缺省 `false`）。
    pub report_enabled: Option<bool>,
}

/// 值获取的通道（**可注入**）。
///
/// ⛔ 本仓**不实现**平台侧的 HTTPS 客户端：调用方注入一个实现，
/// 这样失败分支能测（与 [`crate::kernel_pipe::upload::ProfileUploader`] 同一做法）。
pub trait FpValueTransport {
    /// 问服务端「要不要更新 / 目标值是什么」。
    ///
    /// `request.sign` 有值 ⇒ 带上它（「我已有这份缓存」）。
    fn request_targets(
        &mut self,
        request: &FpValueRequest,
    ) -> Result<FpValueCheck, FpValueFetchError>;

    /// 下载值载荷（服务端给的是句柄而不是内联目标值时）。
    fn download(&mut self, value_ref: &str) -> Result<String, FpValueFetchError>;
}

/// 拉取 / 续期的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpRefreshOutcome {
    /// 这次是否更新了值。
    pub updated: bool,
    /// 生效的目标值（没有 ⇒ 没有目标值 ⇒ 内核退回现有行为）。
    pub entry: Option<FpValueCacheEntry>,
}

/// 拉取 / 续期目标值。
///
/// ⛔ **永远不返回错误**：失败 ⇒ 记日志 + 计数 ⇒ 用旧值继续
/// （⛔ 不许让浏览器起不来）。
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn refresh_values(
    transport: &mut dyn FpValueTransport,
    cache: &FpValueCache,
    env_id: &str,
    container_id: &str,
    now_unix: u64,
    log: &mut dyn FnMut(&str),
    stats: &mut FpValueStats,
) -> FpRefreshOutcome {
    let cached = cache.load(env_id, stats);
    let request = FpValueRequest {
        container_id: container_id.to_string(),
        env_id: env_id.to_string(),
        sign: cached.as_ref().and_then(|entry| entry.sign.clone()),
    };

    let check = match transport.request_targets(&request) {
        Ok(check) => check,
        Err(err) => {
            log(&format!(
                "[lengsuo-desktop] 值治理：拉取失败（{err}），沿用旧值"
            ));
            return FpRefreshOutcome {
                updated: false,
                entry: cached,
            };
        }
    };

    if !check.needs_update {
        log("[lengsuo-desktop] 值治理：服务端说无需更新，沿用旧值");
        return FpRefreshOutcome {
            updated: false,
            entry: cached,
        };
    }

    let Some(set) = resolve_target_set(transport, &check, log, stats) else {
        stats.validation_failures += 1;
        log("[lengsuo-desktop] 值治理：目标值校验失败，沿用旧值");
        return FpRefreshOutcome {
            updated: false,
            entry: cached,
        };
    };

    let report_enabled = check.report_enabled.unwrap_or_else(|| {
        cached
            .as_ref()
            .map(|entry| entry.report_enabled)
            .unwrap_or(false)
    });
    let entry = FpValueCacheEntry {
        sign: check
            .sign
            .clone()
            .or_else(|| cached.as_ref().and_then(|old| old.sign.clone())),
        container_id: container_id.to_string(),
        updated_at_unix: now_unix,
        similarity: set.similarity,
        report_enabled,
        targets: set.targets,
    };

    if !cache.store(env_id, &entry) {
        log("[lengsuo-desktop] 值治理：值缓存落盘失败");
        return FpRefreshOutcome {
            updated: false,
            entry: Some(entry),
        };
    }
    stats.updates += 1;
    FpRefreshOutcome {
        updated: true,
        entry: Some(entry),
    }
}

/// 取出目标值集：内联的直接用，否则下载值载荷再解析。
fn resolve_target_set(
    transport: &mut dyn FpValueTransport,
    check: &FpValueCheck,
    log: &mut dyn FnMut(&str),
    stats: &mut FpValueStats,
) -> Option<FpTargetSet> {
    if let Some(set) = &check.set {
        return Some(set.clone());
    }
    let value_ref = check.value_ref.as_ref()?;
    let text = match transport.download(value_ref) {
        Ok(text) => text,
        Err(err) => {
            log(&format!(
                "[lengsuo-desktop] 值治理：值载荷下载失败（{err}），沿用旧值"
            ));
            stats.validation_failures += 1;
            return None;
        }
    };
    match FpTargetSet::parse_json(&text) {
        Ok(set) => Some(set),
        Err(err) => {
            log(&format!(
                "[lengsuo-desktop] 值治理：值载荷畸形（{err}），沿用旧值"
            ));
            stats.validation_failures += 1;
            None
        }
    }
}

/// 解析服务端答复（`needs_update` / `sign` / 目标值 / 值载荷句柄）。
///
/// ⛔ **畸形即拒绝**：答复结构不合法 ⇒ [`FpValueFetchError::Malformed`]。
pub fn parse_check_json(json: &str) -> Result<FpValueCheck, FpValueFetchError> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|_| FpValueFetchError::Malformed("答复不是合法 JSON".to_string()))?;
    let object = value
        .as_object()
        .ok_or_else(|| FpValueFetchError::Malformed("答复不是 JSON 对象".to_string()))?;

    let needs_update = object
        .get("needs_update")
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| FpValueFetchError::Malformed("缺 needs_update".to_string()))?;

    let sign = match object.get("sign") {
        None | Some(serde_json::Value::Null) => None,
        Some(item) => {
            let text = item
                .as_str()
                .ok_or_else(|| FpValueFetchError::Malformed("sign 不是字符串".to_string()))?;
            if text.is_empty() {
                None
            } else {
                Some(Redacted::new(text))
            }
        }
    };

    let set = match object.get("targets") {
        None | Some(serde_json::Value::Null) => None,
        Some(targets) => {
            // 内联目标值时，`similarity` 是 `targets` 的**兄弟**字段
            // ⇒ 拼成目标值集的形状 `{"similarity":N,"targets":[…]}` 再解析。
            let similarity = object.get("similarity").cloned().ok_or_else(|| {
                FpValueFetchError::Malformed("内联目标值缺 similarity".to_string())
            })?;
            let mut inline = serde_json::Map::new();
            inline.insert("similarity".to_string(), similarity);
            inline.insert("targets".to_string(), targets.clone());
            Some(
                FpTargetSet::parse_value(&serde_json::Value::Object(inline)).map_err(|err| {
                    FpValueFetchError::Malformed(format!("内联目标值不合法：{err}"))
                })?,
            )
        }
    };

    let value_ref = match object.get("value_ref") {
        None | Some(serde_json::Value::Null) => None,
        Some(item) => Some(
            item.as_str()
                .ok_or_else(|| FpValueFetchError::Malformed("value_ref 不是字符串".to_string()))?
                .to_string(),
        ),
    };

    let report_enabled =
        match object.get("report_enabled") {
            None | Some(serde_json::Value::Null) => None,
            Some(item) => Some(item.as_bool().ok_or_else(|| {
                FpValueFetchError::Malformed("report_enabled 不是布尔".to_string())
            })?),
        };

    Ok(FpValueCheck {
        needs_update,
        sign,
        set,
        value_ref,
        report_enabled,
    })
}
