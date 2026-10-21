//! 内核管道（kernel pipe）的**纯逻辑层**：帧编解码 + 密钥派生 + 握手状态机。
//!
//! 边界（对齐 `docs/kernel-wire-contract.md`）：
//! - 本层**不碰 IO**：不建命名管道、不起进程、不收发 socket。
//!   接线（起管道、启动内核、传 `--lengsuo-kernel-pipe=<name>`）归 P2。
//! - 本层**不依赖 GPUI**，与 [`crate::state`] 同一分层纪律：纯逻辑、可单测。
//! - 字节布局**逐字节对齐**跨仓线协议 `kernel-wire-contract.md` §2/§3。
//!   固定测试向量抄自该文档（唯一真相），两侧必须逐字节相同。
//!
//! 帧格式（线协议 §2）：
//!
//! ```text
//! 偏移  长度  字段
//! 0     4     len      u32 大端，payload 字节数（不含本头 8 字节）
//! 4     1     type     u8
//! 5     3     reserved u8×3，必须为 0
//! 8     len   payload
//! ```
//!
//! ⛔ **大端（网络序）**——跨仓最容易错的一处，测试用固定字节数组锁死。

use thiserror::Error;

pub mod handshake;
pub mod keys;

#[cfg(test)]
mod tests;

/// 帧头长度：`len`(4) + `type`(1) + `reserved`(3)。
pub const FRAME_HEADER_LEN: usize = 8;

/// 单帧 payload 上限：8 MiB（线协议 §2）。
///
/// 8388608 = `0x00800000`，恰为上限本身**合法**，再大 1 字节即非法。
pub const MAX_PAYLOAD_LEN: usize = 8 * 1024 * 1024;

/// `0x01` Hello：内核 → runner，载荷 JSON `{version, nonce}`。
pub const FRAME_TYPE_HELLO: u8 = 0x01;
/// `0x02` KeyEnvelope：runner → 内核，载荷为二进制密钥信封。
pub const FRAME_TYPE_KEY_ENVELOPE: u8 = 0x02;
/// `0x03` ConfigBundle：runner → 内核，载荷 JSON（配置包）。
pub const FRAME_TYPE_CONFIG_BUNDLE: u8 = 0x03;
/// `0x04` ActionRequest：runner → 内核，载荷 JSON（指令）。
pub const FRAME_TYPE_ACTION_REQUEST: u8 = 0x04;
/// `0x05` ActionResult：内核 → runner，载荷 JSON `{ok, error_code}`。
pub const FRAME_TYPE_ACTION_RESULT: u8 = 0x05;
/// `0x81` Event：内核 → runner，载荷 JSON（上行事件）。
pub const FRAME_TYPE_EVENT: u8 = 0x81;
/// `0x82` EventAck：runner → 内核，载荷 JSON `{seq}`。
pub const FRAME_TYPE_EVENT_ACK: u8 = 0x82;

/// 帧编解码错误。
///
/// ⛔ 错误文本**不得**包含密钥、信封内容或载荷内容——只说「哪一条约束被违反」。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum FrameError {
    /// 帧头被截断（不足 8 字节）。
    #[error("帧头被截断")]
    HeaderTruncated,
    /// `len` 超过 8 MiB 上限。
    #[error("帧 payload 超过 8 MiB 上限")]
    PayloadTooLarge,
    /// `reserved` 三字节非零。
    #[error("帧头 reserved 字段非零")]
    ReservedNotZero,
    /// `type` 不是已知帧类型。
    #[error("未知的帧类型：{0:#04x}")]
    UnknownType(u8),
    /// payload 字节数不足 `len` 声明的长度。
    #[error("帧 payload 不完整")]
    PayloadTruncated,
    /// 缓冲区尾部有多余字节（`decode` 要求恰好一帧）。
    #[error("帧尾有多余字节")]
    TrailingBytes,
}

/// 已知帧类型判定（线协议 §3 的 7 个值，一个不多一个不少）。
#[must_use]
pub fn is_known_frame_type(frame_type: u8) -> bool {
    matches!(
        frame_type,
        FRAME_TYPE_HELLO
            | FRAME_TYPE_KEY_ENVELOPE
            | FRAME_TYPE_CONFIG_BUNDLE
            | FRAME_TYPE_ACTION_REQUEST
            | FRAME_TYPE_ACTION_RESULT
            | FRAME_TYPE_EVENT
            | FRAME_TYPE_EVENT_ACK
    )
}

/// 一帧：8 字节头 + payload。
///
/// 头**按收到的原始字节保存**（不是重新算的）：GCM 的 AAD 就是这 8 字节，
/// 重新算虽然结果相同，但保留原字节才谈得上「头被改即认证失败」。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    header: [u8; FRAME_HEADER_LEN],
    frame_type: u8,
    payload: Vec<u8>,
}

impl Frame {
    /// 构造一帧（校验帧类型与 payload 长度上限）。
    pub fn new(frame_type: u8, payload: Vec<u8>) -> Result<Self, FrameError> {
        if !is_known_frame_type(frame_type) {
            return Err(FrameError::UnknownType(frame_type));
        }
        if payload.len() > MAX_PAYLOAD_LEN {
            return Err(FrameError::PayloadTooLarge);
        }
        Ok(Self {
            header: build_header(frame_type, payload.len()),
            frame_type,
            payload,
        })
    }

    /// 帧类型。
    #[must_use]
    pub fn frame_type(&self) -> u8 {
        self.frame_type
    }

    /// payload 字节。
    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// 帧头 8 字节——**GCM 的 AAD**（线协议 §5）。
    #[must_use]
    pub fn header(&self) -> &[u8; FRAME_HEADER_LEN] {
        &self.header
    }

    /// 整帧字节（头 + payload）。
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(FRAME_HEADER_LEN + self.payload.len());
        out.extend_from_slice(&self.header);
        out.extend_from_slice(&self.payload);
        out
    }
}

/// 造 8 字节帧头：`len` 大端 + `type` + 三个 0。
fn build_header(frame_type: u8, payload_len: usize) -> [u8; FRAME_HEADER_LEN] {
    let mut header = [0u8; FRAME_HEADER_LEN];
    header[..4].copy_from_slice(&(payload_len as u32).to_be_bytes());
    header[4] = frame_type;
    header
}

/// 编码一帧：返回「头 + payload」的线字节。
///
/// 与 [`decode`] 互为逆运算：未知帧类型、payload 超 8 MiB 一律**拒绝**
/// （fail-closed：宁可不发，也不发一帧对端必然拒收的字节）。
pub fn encode(frame_type: u8, payload: &[u8]) -> Result<Vec<u8>, FrameError> {
    Ok(Frame::new(frame_type, payload.to_vec())?.encode())
}

/// 解码**恰好一帧**（缓冲区尾部必须没有多余字节）。
pub fn decode(bytes: &[u8]) -> Result<Frame, FrameError> {
    let (frame, consumed) = decode_prefix(bytes)?;
    if consumed != bytes.len() {
        return Err(FrameError::TrailingBytes);
    }
    Ok(frame)
}

/// 解码缓冲区的**第一帧**，并返回它占用的字节数（供 P2 做流式拆帧）。
///
/// 失败时**不返回任何部分明文**（线协议 §2 的 fail-closed 要求）。
pub fn decode_prefix(bytes: &[u8]) -> Result<(Frame, usize), FrameError> {
    if bytes.len() < FRAME_HEADER_LEN {
        return Err(FrameError::HeaderTruncated);
    }
    let payload_len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    if payload_len > MAX_PAYLOAD_LEN {
        return Err(FrameError::PayloadTooLarge);
    }
    if bytes[5..8] != [0u8, 0u8, 0u8] {
        return Err(FrameError::ReservedNotZero);
    }
    let frame_type = bytes[4];
    if !is_known_frame_type(frame_type) {
        return Err(FrameError::UnknownType(frame_type));
    }
    let total = FRAME_HEADER_LEN + payload_len;
    if bytes.len() < total {
        return Err(FrameError::PayloadTruncated);
    }
    let mut header = [0u8; FRAME_HEADER_LEN];
    header.copy_from_slice(&bytes[..FRAME_HEADER_LEN]);
    Ok((
        Frame {
            header,
            frame_type,
            payload: bytes[FRAME_HEADER_LEN..total].to_vec(),
        },
        total,
    ))
}
