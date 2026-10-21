//! 握手状态机（**纯状态**，不碰 IO）。
//!
//! # 状态机
//!
//! ```text
//! WaitHello --(收到 Hello 且版本匹配)--> Handshake
//!     │                                     │
//!     │ 失败                                 │ 失败
//!     ▼                                     ▼
//!   Failed <─────────── 失败 ───────────────┘
//!
//! Handshake --(收到 KeyEnvelope 且解密成功)--> Ready
//! ```
//!
//! ⛔ **fail-closed**：任何一步失败都进 [`HandshakeState::Failed`]，
//! **绝不降级成「无密钥继续跑」**。调用方看到 `Failed` 就断链。
//!
//! # ⛔ 安全边界
//!
//! - 会话密钥**只存内存**（[`Handshake`] 的字段），⛔ 不写盘、不进环境变量、
//!   不进命令行、不进日志；[`Handshake`] 被丢弃时清零。
//! - 错误信息**不含密钥或信封内容**——见 [`HandshakeError`]。
//! - 会话密钥的**生成方式**（CSPRNG 随机 vs HKDF 派生）由调用方注入
//!   （见 [`crate::kernel_pipe::keys::SessionKeySource`]），本模块不替主理人选。

use zeroize::Zeroize;

use crate::kernel_pipe::keys::{
    self, EnvelopeError, EnvelopePlain, HELLO_NONCE_LEN, K_MACHINE_LEN, SESSION_KEY_LEN,
};
use crate::kernel_pipe::{Frame, FrameType};

/// 协议版本（线协议 §1：当前 `1`）。
///
/// 不匹配 ⇒ **双方都拒绝并断链**。
pub const PROTOCOL_VERSION: u32 = 1;

/// 握手失败的原因。
///
/// ⛔ 这些文本**不得**包含会话密钥或信封内容——本枚举只携带「哪一步失败」。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HandshakeError {
    /// 收到的帧类型不是当前状态期待的。
    #[error("帧类型不对：当前期待 {expected}，收到 {got:?}")]
    UnexpectedFrame {
        /// 当前状态期待的帧类型名。
        expected: &'static str,
        /// 实际收到的帧类型。
        got: FrameType,
    },

    /// Hello 载荷不是合法的 JSON / 字段缺失 / 类型不对。
    #[error("Hello 载荷非法")]
    MalformedHello,

    /// 协议版本不匹配 ⇒ 拒绝并断链。
    #[error("协议版本不匹配：期望 {expected}，收到 {got}")]
    VersionMismatch {
        /// 本端支持的协议版本。
        expected: u32,
        /// 对端声称的协议版本。
        got: u32,
    },

    /// 密钥信封坏了（截断 / 长度不自洽 / 认证失败）。
    #[error("密钥信封失败：{0}")]
    Envelope(#[from] EnvelopeError),

    /// 状态机已经结束（已 `Ready` 或已 `Failed`），不再接受新的帧。
    #[error("握手已结束")]
    AlreadyFinished,
}

/// 握手状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandshakeState {
    /// 等内核发 `Hello`。
    WaitHello,
    /// 已收到 `Hello`（版本匹配），握手进行中。
    ///
    /// 持有 Hello 的 8 字节 nonce——它是密钥信封 AAD 的一部分。
    Handshake {
        /// Hello 帧里的 8 字节 nonce。
        hello_nonce: [u8; HELLO_NONCE_LEN],
    },
    /// 握手完成：会话密钥已在内存中就位。
    Ready,
    /// 握手失败 ⇒ **调用方必须断链**。
    Failed(HandshakeError),
}

/// 握手状态机（纯状态，无 IO）。
///
/// 职责：判定「收到的帧能不能推进握手」，并在推进时把密钥材料放进内存。
/// ⛔ 不建管道、不读写字节——那些是 P2 接线卡的事。
///
/// ⛔ **`Debug` 是手写的**：`k_machine` 与 `session_key` **不打印**。
/// 派生 `Debug` 会把密钥原样写进日志 ⇒ 违反「⛔ 密钥不进日志」。
pub struct Handshake {
    state: HandshakeState,
    k_machine: [u8; K_MACHINE_LEN],
    /// 会话密钥：只在内存里；`Ready` 之后才填。
    session_key: Option<[u8; SESSION_KEY_LEN]>,
    /// 信封里带的容器 id。
    container_id: Option<Vec<u8>>,
    /// 信封里带的过期时间（Unix 秒）。
    expires_unix: Option<u64>,
}

impl std::fmt::Debug for Handshake {
    /// ⛔ **不打印任何密钥材料**——只打印状态与形状。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handshake")
            .field("state", &self.state)
            .field("k_machine", &"<redacted>")
            .field(
                "session_key",
                &self.session_key.as_ref().map(|_| "<redacted>"),
            )
            .field(
                "container_id_len",
                &self.container_id.as_ref().map(Vec::len),
            )
            .field("expires_unix", &self.expires_unix)
            .finish()
    }
}

impl Drop for Handshake {
    fn drop(&mut self) {
        // ⛔ 会话密钥只该在内存里活着：离开作用域就清零。
        if let Some(key) = self.session_key.as_mut() {
            key.zeroize();
        }
        self.k_machine.zeroize();
    }
}

impl Handshake {
    /// 新建握手状态机（`WaitHello`）。
    #[must_use]
    pub fn new(k_machine: [u8; K_MACHINE_LEN]) -> Self {
        Self {
            state: HandshakeState::WaitHello,
            k_machine,
            session_key: None,
            container_id: None,
            expires_unix: None,
        }
    }

    /// 当前状态。
    #[must_use]
    pub fn state(&self) -> &HandshakeState {
        &self.state
    }

    /// 是否已经握手成功。
    #[must_use]
    pub fn is_ready(&self) -> bool {
        matches!(self.state, HandshakeState::Ready)
    }

    /// 是否已经失败（调用方应断链）。
    #[must_use]
    pub fn is_failed(&self) -> bool {
        matches!(self.state, HandshakeState::Failed(_))
    }

    /// 握手成功后的会话密钥。
    ///
    /// ⛔ 只在内存里；调用方**不得**把它写盘、放进环境变量、命令行或日志。
    #[must_use]
    pub fn session_key(&self) -> Option<&[u8; SESSION_KEY_LEN]> {
        self.session_key.as_ref()
    }

    /// 握手成功后从信封里取到的容器 id。
    #[must_use]
    pub fn container_id(&self) -> Option<&[u8]> {
        self.container_id.as_deref()
    }

    /// 握手成功后从信封里取到的过期时间（Unix 秒）。
    #[must_use]
    pub fn expires_unix(&self) -> Option<u64> {
        self.expires_unix
    }

    /// 推进一帧。
    ///
    /// 成功 ⇒ 状态前进（`WaitHello` → `Handshake` → `Ready`），返回 `Ok(())`。
    /// 失败 ⇒ 状态进 [`HandshakeState::Failed`]，返回 `Err`。
    ///
    /// ⛔ 任何失败都**不会**留下部分明文，也不会进入 `Ready`。
    pub fn on_frame(&mut self, frame: &Frame) -> Result<(), HandshakeError> {
        match self.state.clone() {
            HandshakeState::WaitHello => self.on_hello(frame),
            HandshakeState::Handshake { hello_nonce } => self.on_key_envelope(frame, &hello_nonce),
            HandshakeState::Ready | HandshakeState::Failed(_) => {
                Err(HandshakeError::AlreadyFinished)
            }
        }
    }

    /// `WaitHello` 状态：只接受 `Hello` 帧。
    fn on_hello(&mut self, frame: &Frame) -> Result<(), HandshakeError> {
        if frame.frame_type != FrameType::Hello {
            return Err(self.fail(HandshakeError::UnexpectedFrame {
                expected: "Hello",
                got: frame.frame_type,
            }));
        }

        let (version, nonce) = match parse_hello(&frame.payload) {
            Some(parsed) => parsed,
            None => return Err(self.fail(HandshakeError::MalformedHello)),
        };

        if version != PROTOCOL_VERSION {
            // ⭐ 版本不匹配 ⇒ 拒绝并断链（线协议 §1）。
            return Err(self.fail(HandshakeError::VersionMismatch {
                expected: PROTOCOL_VERSION,
                got: version,
            }));
        }

        self.state = HandshakeState::Handshake { hello_nonce: nonce };
        Ok(())
    }

    /// `Handshake` 状态：只接受 `KeyEnvelope` 帧。
    fn on_key_envelope(
        &mut self,
        frame: &Frame,
        hello_nonce: &[u8; HELLO_NONCE_LEN],
    ) -> Result<(), HandshakeError> {
        if frame.frame_type != FrameType::KeyEnvelope {
            return Err(self.fail(HandshakeError::UnexpectedFrame {
                expected: "KeyEnvelope",
                got: frame.frame_type,
            }));
        }

        let envelope = match keys::KeyEnvelope::decode(&frame.payload) {
            Ok(envelope) => envelope,
            Err(err) => return Err(self.fail(HandshakeError::Envelope(err))),
        };

        // AAD = 帧头 8 字节（len/type/reserved）|| Hello 的 8 字节 nonce。
        // ⚠️ len 用**收到的载荷实际长度**——AAD 必须与线上帧头逐字节相同。
        let aad = envelope_aad(frame.payload.len(), hello_nonce);

        let plain: EnvelopePlain = match keys::open_envelope(&self.k_machine, &aad, &envelope) {
            Ok(plain) => plain,
            Err(err) => return Err(self.fail(HandshakeError::Envelope(err))),
        };

        self.session_key = Some(plain.session_key);
        self.container_id = Some(plain.container_id);
        self.expires_unix = Some(plain.expires_unix);
        self.state = HandshakeState::Ready;
        Ok(())
    }

    /// 记失败并把状态置为 `Failed`。
    fn fail(&mut self, err: HandshakeError) -> HandshakeError {
        // ⛔ 失败 ⇒ 丢掉可能已经拿到的会话密钥。
        if let Some(key) = self.session_key.as_mut() {
            key.zeroize();
        }
        self.session_key = None;
        self.container_id = None;
        self.expires_unix = None;
        self.state = HandshakeState::Failed(err.clone());
        err
    }

    /// runner 侧：构造要发给内核的 `KeyEnvelope` 帧。
    ///
    /// ⛔ **只允许在 `Handshake` 状态调用**（需要 Hello 的 nonce 才能算 AAD）。
    /// - `session_key`：由调用方按 [`keys::SessionKeySource`] 注入（**本模块不选**）。
    /// - `gcm_nonce`：12 字节，**每次信封独立随机**。
    ///
    /// 返回的帧的载荷就是线协议 §3.1 的载荷布局；
    /// 封进密文的 AAD 与帧头逐字节一致（见 [`envelope_aad`]）。
    pub fn build_key_envelope_frame(
        &self,
        session_key: &[u8; SESSION_KEY_LEN],
        container_id: &[u8],
        expires_unix: u64,
        gcm_nonce: [u8; keys::GCM_NONCE_LEN],
    ) -> Result<Frame, HandshakeError> {
        let hello_nonce = match self.state {
            HandshakeState::Handshake { hello_nonce } => hello_nonce,
            _ => return Err(HandshakeError::AlreadyFinished),
        };

        // 密文长度 == 明文长度（GCM 是流模式），所以载荷长度可以先算出来。
        let payload_len = keys::envelope_payload_len(container_id.len());
        let aad = envelope_aad(payload_len, &hello_nonce);

        let plain = EnvelopePlain {
            session_key: *session_key,
            container_id: container_id.to_vec(),
            expires_unix,
        };

        let envelope = keys::seal_envelope(&self.k_machine, &aad, &plain, gcm_nonce)?;
        let payload = envelope.encode();

        debug_assert_eq!(
            payload.len(),
            payload_len,
            "载荷长度与 AAD 里声明的必须一致"
        );

        Ok(Frame::new(FrameType::KeyEnvelope, payload))
    }
}

/// 密钥信封的 AAD（16 字节）= 帧头 8 字节 || Hello nonce 8 字节。
///
/// ⭐ `payload_len` 必须是**该帧载荷的实际长度**——AAD 逐字节复制线上帧头，
/// 头被改一个字节即认证失败（线协议 §5）。
#[must_use]
pub fn envelope_aad(payload_len: usize, hello_nonce: &[u8; HELLO_NONCE_LEN]) -> [u8; 16] {
    let len = u32::try_from(payload_len).unwrap_or(u32::MAX);
    let mut aad = [0u8; 16];
    aad[0..4].copy_from_slice(&len.to_be_bytes());
    aad[4] = FrameType::KeyEnvelope.code();
    // aad[5..8] 保持 0（reserved）
    aad[8..16].copy_from_slice(hello_nonce);
    aad
}

/// 解析 `Hello` 载荷 ⇒ `(version, nonce)`。
///
/// 载荷是 JSON `{version, nonce}`（线协议 §3）。
/// ⚠️ 契约**没有钉死** 8 字节 nonce 在 JSON 里的写法，这里接受两种：
/// - **十六进制字符串**（16 个 hex 字符）——本实现认定的**规范写法**；
/// - 8 个整数的数组。
///
/// 两种写法解出来的都是**同样的 8 个字节**，因此 AAD 与跨仓比对不受影响。
/// （base64 写法需要额外依赖，本卡不能改 `Cargo.toml` ⇒ 不支持。）
fn parse_hello(payload: &[u8]) -> Option<(u32, [u8; HELLO_NONCE_LEN])> {
    let value: serde_json::Value = serde_json::from_slice(payload).ok()?;
    let object = value.as_object()?;

    let version = object.get("version")?.as_u64()?;
    let version = u32::try_from(version).ok()?;

    let raw = object.get("nonce")?;
    let nonce = parse_nonce(raw)?;

    Some((version, nonce))
}

/// 把 JSON 里的 nonce 值解成 8 个字节。
fn parse_nonce(raw: &serde_json::Value) -> Option<[u8; HELLO_NONCE_LEN]> {
    if let Some(text) = raw.as_str() {
        return decode_hex8(text);
    }
    if let Some(items) = raw.as_array() {
        if items.len() != HELLO_NONCE_LEN {
            return None;
        }
        let mut out = [0u8; HELLO_NONCE_LEN];
        for (idx, item) in items.iter().enumerate() {
            out[idx] = u8::try_from(item.as_u64()?).ok()?;
        }
        return Some(out);
    }
    None
}

/// 十六进制 ⇒ 8 字节（大小写都收）。
fn decode_hex8(text: &str) -> Option<[u8; HELLO_NONCE_LEN]> {
    if text.len() != HELLO_NONCE_LEN * 2 {
        return None;
    }
    let mut out = [0u8; HELLO_NONCE_LEN];
    for (idx, slot) in out.iter_mut().enumerate() {
        let hi = hex_val(text.as_bytes()[idx * 2])?;
        let lo = hex_val(text.as_bytes()[idx * 2 + 1])?;
        *slot = (hi << 4) | lo;
    }
    Some(out)
}

/// 单个 hex 字符的数值。
fn hex_val(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod handshake_tests {
    use super::*;
    use crate::kernel_pipe::keys::{self, k_machine, EnvelopeError};
    use crate::kernel_pipe::tests::{
        fixed_k_machine, hex_bytes32, CONTAINER_ID, EXPIRES_UNIX, GCM_NONCE, HELLO_NONCE,
        K_MACHINE_HEX, SESSION_KEY_HEX,
    };

    /// 造一个能推进到 `Handshake` 状态的机。
    fn handshake_in_handshake(k: &[u8; K_MACHINE_LEN]) -> Handshake {
        let mut hs = Handshake::new(*k);
        hs.on_frame(&hello_frame(PROTOCOL_VERSION, &HELLO_NONCE))
            .expect("Hello 应被接受");
        hs
    }

    /// 造一个 `Hello` 帧（JSON `{version, nonce}`，nonce 用十六进制字符串）。
    fn hello_frame(version: u32, nonce: &[u8; HELLO_NONCE_LEN]) -> Frame {
        let nonce_hex: String = nonce.iter().map(|b| format!("{b:02x}")).collect();
        let payload = format!(r#"{{"version":{version},"nonce":"{nonce_hex}"}}"#);
        Frame::new(FrameType::Hello, payload.into_bytes())
    }

    /// 造一个 `KeyEnvelope` 帧，**AAD 与帧头自洽**（len 字段 = 载荷实际长度）。
    ///
    /// ⚠️ 这与线协议 §3.1 **钉死的** AAD（`len` = `0x55`）不同——原因见
    /// `tests.rs` 头部「一处契约内部的矛盾」一节。契约钉死的那组 AAD
    /// 只在 `tests.rs` 的集成测试里使用。
    fn envelope_frame(k: &[u8; K_MACHINE_LEN]) -> Frame {
        let payload_len = keys::envelope_payload_len(CONTAINER_ID.len());
        let aad = envelope_aad(payload_len, &HELLO_NONCE);
        let envelope = keys::seal_envelope(
            k,
            &aad,
            &EnvelopePlain {
                session_key: hex_bytes32(SESSION_KEY_HEX),
                container_id: CONTAINER_ID.to_vec(),
                expires_unix: EXPIRES_UNIX,
            },
            GCM_NONCE,
        )
        .expect("应可封装");
        Frame::new(FrameType::KeyEnvelope, envelope.encode())
    }

    #[test]
    fn handshake_reaches_ready_on_the_happy_path() {
        let k = fixed_k_machine();
        let mut hs = Handshake::new(k);
        assert!(matches!(hs.state(), HandshakeState::WaitHello));
        assert!(!hs.is_ready());

        hs.on_frame(&hello_frame(PROTOCOL_VERSION, &HELLO_NONCE))
            .expect("Hello 应被接受");
        assert!(matches!(hs.state(), HandshakeState::Handshake { .. }));

        hs.on_frame(&envelope_frame(&k))
            .expect("KeyEnvelope 应被接受");
        assert!(hs.is_ready());
        assert!(matches!(hs.state(), HandshakeState::Ready));
        assert_eq!(
            keys::to_hex(hs.session_key().expect("应有会话密钥")),
            SESSION_KEY_HEX
        );
        assert_eq!(hs.container_id(), Some(CONTAINER_ID));
        assert_eq!(hs.expires_unix(), Some(EXPIRES_UNIX));
    }

    #[test]
    fn handshake_fails_on_version_mismatch() {
        let mut hs = Handshake::new(fixed_k_machine());
        let err = hs
            .on_frame(&hello_frame(PROTOCOL_VERSION + 1, &HELLO_NONCE))
            .expect_err("版本不匹配必须失败");
        assert_eq!(
            err,
            HandshakeError::VersionMismatch {
                expected: PROTOCOL_VERSION,
                got: PROTOCOL_VERSION + 1
            }
        );
        assert!(hs.is_failed(), "版本不匹配必须进 Failed");
        assert!(!hs.is_ready());
        assert!(hs.session_key().is_none(), "失败时不得留下会话密钥");
    }

    #[test]
    fn handshake_fails_when_the_envelope_cannot_be_decrypted() {
        let k = fixed_k_machine();
        let mut hs = handshake_in_handshake(&k);

        // 用错误的 k_machine 封装 ⇒ 解不开。
        let other = k_machine("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
        let bad = envelope_frame(&other);
        let err = hs.on_frame(&bad).expect_err("解密失败必须失败");
        assert!(
            matches!(err, HandshakeError::Envelope(EnvelopeError::DecryptFailed)),
            "实际是 {err:?}"
        );
        assert!(hs.is_failed());
        assert!(!hs.is_ready());
        assert!(hs.session_key().is_none(), "失败时不得留下会话密钥");
    }

    #[test]
    fn handshake_fails_on_a_tampered_envelope() {
        let k = fixed_k_machine();
        let mut hs = handshake_in_handshake(&k);
        let mut frame = envelope_frame(&k);
        // 改密文里的 1 字节（密文区在载荷的 [29..74)）。
        frame.payload[30] ^= 0x01;
        let err = hs.on_frame(&frame).expect_err("密文被改必须失败");
        assert!(matches!(
            err,
            HandshakeError::Envelope(EnvelopeError::DecryptFailed)
        ));
        assert!(hs.is_failed());
    }

    #[test]
    fn handshake_rejects_unexpected_frame_types() {
        let mut hs = Handshake::new(fixed_k_machine());
        let err = hs
            .on_frame(&Frame::new(FrameType::ConfigBundle, vec![]))
            .expect_err("WaitHello 收到 ConfigBundle 必须失败");
        assert_eq!(
            err,
            HandshakeError::UnexpectedFrame {
                expected: "Hello",
                got: FrameType::ConfigBundle
            }
        );

        let k = fixed_k_machine();
        let mut hs = handshake_in_handshake(&k);
        let err = hs
            .on_frame(&Frame::new(FrameType::Event, vec![]))
            .expect_err("Handshake 收到 Event 必须失败");
        assert_eq!(
            err,
            HandshakeError::UnexpectedFrame {
                expected: "KeyEnvelope",
                got: FrameType::Event
            }
        );
    }

    #[test]
    fn handshake_rejects_a_malformed_hello() {
        for payload in [
            &b"not json"[..],
            &b"{}"[..],
            &br#"{"version":1}"#[..],
            &br#"{"nonce":"0102030405060708"}"#[..],
            &br#"{"version":"1","nonce":"0102030405060708"}"#[..],
            &br#"{"version":1,"nonce":"0102030405"}"#[..],
            &br#"{"version":1,"nonce":"010203040506070809"}"#[..],
            &br#"{"version":1,"nonce":"zz02030405060708"}"#[..],
        ] {
            let mut hs = Handshake::new(fixed_k_machine());
            let err = hs
                .on_frame(&Frame::new(FrameType::Hello, payload.to_vec()))
                .expect_err("非法 Hello 必须失败");
            assert_eq!(err, HandshakeError::MalformedHello, "载荷 {payload:?}");
            assert!(hs.is_failed());
        }
    }

    #[test]
    fn handshake_accepts_the_array_form_of_the_nonce() {
        // 契约没钉死 nonce 在 JSON 里的写法 ⇒ 十六进制串与 8 个整数的数组都认。
        let payload = br#"{"version":1,"nonce":[1,2,3,4,5,6,7,8]}"#;
        let mut hs = Handshake::new(fixed_k_machine());
        hs.on_frame(&Frame::new(FrameType::Hello, payload.to_vec()))
            .expect("数组写法的 nonce 应被接受");
        assert!(matches!(hs.state(), HandshakeState::Handshake { .. }));
    }

    #[test]
    fn handshake_refuses_frames_after_it_finished() {
        let k = fixed_k_machine();
        let mut hs = Handshake::new(k);
        hs.on_frame(&hello_frame(PROTOCOL_VERSION, &HELLO_NONCE))
            .expect("Hello 应被接受");
        let err = hs
            .on_frame(&hello_frame(PROTOCOL_VERSION, &HELLO_NONCE))
            .expect_err("Handshake 状态收到第二个 Hello 必须失败");
        assert_eq!(
            err,
            HandshakeError::UnexpectedFrame {
                expected: "KeyEnvelope",
                got: FrameType::Hello
            }
        );
    }

    #[test]
    fn failure_messages_never_contain_key_material() {
        let k = fixed_k_machine();
        let session_bytes = hex_bytes32(SESSION_KEY_HEX);

        // 收集所有失败路径的错误文本。
        let mut messages: Vec<String> = Vec::new();

        let mut hs = Handshake::new(k);
        messages.push(
            hs.on_frame(&hello_frame(PROTOCOL_VERSION + 1, &HELLO_NONCE))
                .expect_err("版本不匹配必须失败")
                .to_string(),
        );

        let mut hs = Handshake::new(k);
        messages.push(
            hs.on_frame(&Frame::new(FrameType::Hello, b"{".to_vec()))
                .expect_err("非法 Hello 必须失败")
                .to_string(),
        );

        let mut hs = handshake_in_handshake(&k);
        let other = k_machine("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
        messages.push(
            hs.on_frame(&envelope_frame(&other))
                .expect_err("解密失败必须失败")
                .to_string(),
        );

        let mut hs = handshake_in_handshake(&k);
        messages.push(
            hs.on_frame(&Frame::new(FrameType::Event, b"payload".to_vec()))
                .expect_err("帧类型不对必须失败")
                .to_string(),
        );

        for message in &messages {
            assert!(
                !message.contains(SESSION_KEY_HEX),
                "错误文本不得含会话密钥的十六进制：{message}"
            );
            for byte in session_bytes {
                let needle = format!("{byte:02x}");
                assert!(
                    !message.contains(&needle),
                    "错误文本不得含会话密钥字节 {needle}：{message}"
                );
            }
            assert!(
                !message.contains(K_MACHINE_HEX),
                "错误文本不得含 k_machine：{message}"
            );
        }
    }
}
