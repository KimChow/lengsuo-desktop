//! 握手状态机（纯状态，**不碰 IO**）。
//!
//! 状态迁移（线协议 §1 / 任务书 §2.4）：
//!
//! ```text
//! WaitHello --(收到 Hello)--> Handshake --(收到 KeyEnvelope 且解密成功)--> Ready
//!      │                            │
//!      └── 失败 ────────────────────┴──> Failed（断链 + 报错）
//! ```
//!
//! 硬约定：
//! - `version` 不匹配 ⇒ **拒绝**（双方都断链）。
//! - 会话密钥**只存内存**；⛔ 不写盘、不进环境变量、不进命令行、不进日志。
//! - 解密失败 / 认证失败 ⇒ `Failed`，**绝不降级成「无密钥」**。
//! - 错误信息**不得**包含密钥或信封内容。
//!
//! ⛔ 本模块是**纯状态机**：喂进帧、拿回状态迁移结果。它不建管道、不读写 socket，
//! 那些归 P2。

use thiserror::Error;
use zeroize::Zeroizing;

use crate::kernel_pipe::keys::{self, EnvelopePlaintext, K_MACHINE_LEN, SESSION_KEY_LEN};
use crate::kernel_pipe::{Frame, FRAME_TYPE_HELLO, FRAME_TYPE_KEY_ENVELOPE};

/// 当前协议版本（线协议 §1：当前为 `1`）。
pub const PROTOCOL_VERSION: u32 = 1;

/// 握手失败原因。
///
/// ⛔ 每个变体的错误文本都**不含**密钥、信封内容或载荷内容。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum HandshakeError {
    /// 协议版本不匹配（线协议 §1：双方都拒绝并断链）。
    #[error("协议版本不匹配")]
    VersionMismatch,
    /// 期望的帧类型不对（该来 Hello 却来了别的）。
    #[error("握手帧类型不符合预期")]
    UnexpectedFrame,
    /// Hello 载荷格式非法（不是合法 JSON / 缺字段）。
    #[error("Hello 载荷非法")]
    MalformedHello,
    /// 密钥信封解不开（密钥不对 / 密文或标签被改 / 信封结构非法）。
    ///
    /// ⛔ **不区分**具体原因：那会给攻击者一个 oracle。
    #[error("密钥信封解封失败")]
    EnvelopeRejected,
    /// 会话密钥已过期（线协议 §1：过期后拒绝使用，fail-closed）。
    #[error("会话密钥已过期")]
    Expired,
    /// 状态机已经 `Failed`，任何后续输入都拒绝。
    #[error("握手已失败")]
    AlreadyFailed,
}

/// 握手状态机的状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeState {
    /// 等内核发 `Hello`。
    WaitHello,
    /// 已收到合法的 `Hello`，等 `KeyEnvelope`。
    Handshake,
    /// 握手完成，会话密钥已就位（**只在内存**）。
    Ready,
    /// 失败：调用方必须断链。
    Failed,
}

/// 一次状态迁移的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandshakeOutcome {
    /// 状态变了，继续等下一帧。
    Transitioned(HandshakeState),
    /// 握手完成，会话密钥已就位。
    Ready(Zeroizing<[u8; SESSION_KEY_LEN]>),
    /// 失败：调用方必须断链。
    Failed(HandshakeError),
}

impl HandshakeOutcome {
    /// 本次迁移是否成功。
    #[must_use]
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }

    /// 失败原因（成功时为 `None`）。
    #[must_use]
    pub fn error(&self) -> Option<HandshakeError> {
        match self {
            Self::Failed(err) => Some(*err),
            _ => None,
        }
    }
}

/// 内核发来的 `Hello`（线协议 §3：JSON `{version, nonce}`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hello {
    /// 协议版本。
    pub version: u32,
    /// 内核生成的 8 字节随机数（参与会话密钥派生，防重放）。
    pub nonce: [u8; 8],
}

/// 解析 `Hello` 载荷。
///
/// 载荷是 JSON 文本 `{version, nonce}`。
///
/// ⚠️ **nonce 的 JSON 编码形式线协议没写死**（见报告「语义冲突」一节）。
/// ⇒ 这里**同时接受**两种最自然的写法，解析出来是同一串字节：
/// - `"nonce": "0102030405060708"`（16 个十六进制字符，大小写皆可）
/// - `"nonce": [1, 2, 3, 4, 5, 6, 7, 8]`（8 个字节的数值数组）
///
/// 两种写法的字节含义一致 ⇒ 对端用哪种都能握手成功，不必返工改协议。
pub fn parse_hello(payload: &[u8]) -> Result<Hello, HandshakeError> {
    let text = std::str::from_utf8(payload).map_err(|_| HandshakeError::MalformedHello)?;
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|_| HandshakeError::MalformedHello)?;
    let version = value
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .ok_or(HandshakeError::MalformedHello)?;
    let nonce_value = value.get("nonce").ok_or(HandshakeError::MalformedHello)?;
    let nonce = parse_nonce(nonce_value).ok_or(HandshakeError::MalformedHello)?;
    Ok(Hello {
        version: u32::try_from(version).map_err(|_| HandshakeError::MalformedHello)?,
        nonce,
    })
}

/// 解析 nonce（十六进制串或字节数组）。
fn parse_nonce(value: &serde_json::Value) -> Option<[u8; 8]> {
    if let Some(text) = value.as_str() {
        return parse_nonce_hex(text);
    }
    let array = value.as_array()?;
    if array.len() != 8 {
        return None;
    }
    let mut out = [0u8; 8];
    for (index, item) in array.iter().enumerate() {
        out[index] = u8::try_from(item.as_u64()?).ok()?;
    }
    Some(out)
}

/// 解析 8 字节 nonce 的十六进制表示（16 个十六进制字符）。
fn parse_nonce_hex(text: &str) -> Option<[u8; 8]> {
    if text.len() != 16 {
        return None;
    }
    let mut out = [0u8; 8];
    for (index, chunk) in text.as_bytes().chunks(2).enumerate() {
        let hi = hex_value(chunk[0])?;
        let lo = hex_value(chunk[1])?;
        out[index] = (hi << 4) | lo;
    }
    Some(out)
}

/// 单个十六进制字符的值（大小写皆可）。
fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// 握手状态机（**纯状态**，不碰 IO）。
///
/// 典型用法（P2 的服务端）：
///
/// ```ignore
/// let mut hs = Handshake::new(&k_machine);
/// match hs.on_frame(&frame) {
///     HandshakeOutcome::Transitioned(HandshakeState::Handshake) => { /* 发 KeyEnvelope */ }
///     HandshakeOutcome::Ready(key) => { /* key 只在内存里用 */ }
///     HandshakeOutcome::Failed(err) => { /* 断链 + 报错 */ }
///     _ => {}
/// }
/// ```
pub struct Handshake {
    state: HandshakeState,
    /// 机器码派生密钥（信封的加解密密钥）。**只在内存**。
    k_machine: Zeroizing<[u8; K_MACHINE_LEN]>,
    /// 内核发来的 nonce（参与会话密钥派生）。
    nonce: Option<[u8; 8]>,
}

impl Handshake {
    /// 新建状态机，初始为 `WaitHello`。
    #[must_use]
    pub fn new(k_machine: &[u8; K_MACHINE_LEN]) -> Self {
        Self {
            state: HandshakeState::WaitHello,
            k_machine: Zeroizing::new(*k_machine),
            nonce: None,
        }
    }

    /// 当前状态。
    #[must_use]
    pub fn state(&self) -> HandshakeState {
        self.state
    }

    /// 是否已完成握手。
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.state == HandshakeState::Ready
    }

    /// 喂进一帧，推进状态机。
    ///
    /// ⛔ 失败即 `Failed`：**绝不**回落到「无密钥继续跑」。
    pub fn on_frame(&mut self, frame: &Frame) -> HandshakeOutcome {
        if self.state == HandshakeState::Failed {
            return HandshakeOutcome::Failed(HandshakeError::AlreadyFailed);
        }
        match self.state {
            HandshakeState::WaitHello => self.on_hello_frame(frame),
            HandshakeState::Handshake => self.on_envelope_frame(frame),
            HandshakeState::Ready | HandshakeState::Failed => {
                HandshakeOutcome::Failed(HandshakeError::UnexpectedFrame)
            }
        }
    }

    /// `WaitHello` 态：只接受 `Hello` 帧。
    fn on_hello_frame(&mut self, frame: &Frame) -> HandshakeOutcome {
        if frame.frame_type() != FRAME_TYPE_HELLO {
            return self.fail(HandshakeError::UnexpectedFrame);
        }
        let hello = match parse_hello(frame.payload()) {
            Ok(hello) => hello,
            Err(err) => return self.fail(err),
        };
        if hello.version != PROTOCOL_VERSION {
            return self.fail(HandshakeError::VersionMismatch);
        }
        self.nonce = Some(hello.nonce);
        self.state = HandshakeState::Handshake;
        HandshakeOutcome::Transitioned(HandshakeState::Handshake)
    }

    /// `Handshake` 态：只接受 `KeyEnvelope` 帧。
    fn on_envelope_frame(&mut self, frame: &Frame) -> HandshakeOutcome {
        if frame.frame_type() != FRAME_TYPE_KEY_ENVELOPE {
            return self.fail(HandshakeError::UnexpectedFrame);
        }
        let envelope = match keys::KeyEnvelope::decode(frame.payload()) {
            Ok(envelope) => envelope,
            Err(_) => return self.fail(HandshakeError::EnvelopeRejected),
        };
        let plain: EnvelopePlaintext = match keys::open_envelope(&self.k_machine, &envelope) {
            Ok(plain) => plain,
            Err(_) => return self.fail(HandshakeError::EnvelopeRejected),
        };
        self.state = HandshakeState::Ready;
        HandshakeOutcome::Ready(Zeroizing::new(plain.session_key))
    }

    /// 进入 `Failed` 态。
    fn fail(&mut self, err: HandshakeError) -> HandshakeOutcome {
        self.state = HandshakeState::Failed;
        HandshakeOutcome::Failed(err)
    }
}
