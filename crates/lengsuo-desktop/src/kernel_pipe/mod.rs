//! 内核管道（kernel pipe）的**纯逻辑层**：帧编解码 + 密钥 + 握手状态机。
//!
//! # 边界
//!
//! 本模块**只做纯逻辑**：字节进、字节出、状态进、状态出。
//! ⛔ 不碰 IO（不建管道、不读写 socket、不起进程）——那是 P2 接线卡的事。
//! ⛔ 不依赖 GPUI、不依赖 `crate::ui`——照 `state/` 的分层惯例，纯逻辑独立可测。
//!
//! # 线协议
//!
//! 字节布局的**唯一真相**是跨仓契约 `kernel-wire-contract.md`（内核仓）。
//! 本模块的每一个常量都对齐它，测试里逐字节锁死固定测试向量。
//! 摘要：
//!
//! ```text
//! 偏移  长度  字段
//! 0     4     len      u32 大端，payload 字节数（不含本头 8 字节）
//! 4     1     type     u8
//! 5     3     reserved u8×3，必须为 0
//! 8     len   payload
//! ```
//!
//! ⭐ **字节序是大端（网络序）**——跨仓最容易错的一处，测试里用手工字节数组固定死。
//!
//! # 子模块
//!
//! - [`keys`]：机器码派生、密钥信封编解码（纯逻辑）。
//! - [`handshake`]：握手状态机（纯状态，不碰 IO）。
//! - [`server`]：**IO 层**——管道服务端（监听 + 收发循环）。
//! - [`launch`]：启动内核进程并拼命令行。
//! - [`profile_rules`]：profile 同步的收集规则（白名单 / 黑名单 / 上限，纯逻辑）。
//! - [`zip_writer`]：profile 文件清单 ⇄ zip 归档（清单解析 + 扫目录 + 流式写）。
//! - [`upload`]：profile 云同步的编排层（打包 + 上传 + 失败不阻塞）。

pub mod handshake;
pub mod keys;
pub mod launch;
pub mod machine;
pub mod profile_rules;
pub mod server;
pub mod upload;
pub mod zip_writer;

#[cfg(test)]
pub(crate) mod machine_tests;
#[cfg(test)]
pub(crate) mod profile_vectors;

#[cfg(test)]
pub(crate) mod tests;

/// 帧头长度（字节）。`len` 字段描述的是**不含本头**的 payload 长度。
pub const HEADER_LEN: usize = 8;

/// 单帧 payload 上限：8 MiB。
///
/// 超过上限的 `len` **解码失败**——不等数据、不截断、不做部分明文。
/// 上限本身取自线协议 §2；`8 * 1024 * 1024 == 0x0080_0000`。
pub const MAX_PAYLOAD_LEN: u32 = 8 * 1024 * 1024;

/// 帧类型（线协议 §3）。
///
/// ⚠️ 这些值是**跨仓约定**，改一个就要两侧同时改（并同步改固定测试向量）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FrameType {
    /// `0x01` 内核 → runner：JSON `{version, nonce}`。
    Hello = 0x01,
    /// `0x02` runner → 内核：二进制密钥信封（见 [`keys`]）。
    KeyEnvelope = 0x02,
    /// `0x03` runner → 内核：JSON 配置包。
    ConfigBundle = 0x03,
    /// `0x04` runner → 内核：JSON 指令。
    ActionRequest = 0x04,
    /// `0x05` 内核 → runner：JSON `{ok, error_code}`。
    ActionResult = 0x05,
    /// `0x81` 内核 → runner：JSON 上行事件。
    Event = 0x81,
    /// `0x82` runner → 内核：JSON `{seq}`。
    EventAck = 0x82,
}

impl FrameType {
    /// 帧类型的线协议取值。
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// 已知帧类型 ⇒ 字节值；未知 ⇒ `None`。
    ///
    /// 未知类型是**解码失败**（线协议 §2），不是「原样透传的未知类型」。
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0x01 => Some(Self::Hello),
            0x02 => Some(Self::KeyEnvelope),
            0x03 => Some(Self::ConfigBundle),
            0x04 => Some(Self::ActionRequest),
            0x05 => Some(Self::ActionResult),
            0x81 => Some(Self::Event),
            0x82 => Some(Self::EventAck),
            _ => None,
        }
    }
}

/// 帧解码失败的原因。
///
/// ⛔ 这些错误文本**不得**包含载荷内容：载荷里可能有密钥信封。
/// 帧头里的 `type`/`len` 是公开的协议常量，可以安全出现在错误文本里。
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FrameError {
    /// 缓冲区不足以凑出完整的 8 字节帧头。
    #[error("帧头被截断：需要 {HEADER_LEN} 字节，实际 {actual} 字节")]
    TruncatedHeader { actual: usize },

    /// 帧头里的 `len` 超过 8 MiB 上限。
    #[error("帧长超限：声明 {declared} 字节，上限 {MAX_PAYLOAD_LEN} 字节")]
    PayloadTooLarge { declared: u32 },

    /// 帧头里的 `reserved` 三字节不全为 0。
    #[error("保留位非零")]
    ReservedNotZero,

    /// 帧头里的 `type` 不是已知帧类型。
    #[error("未知帧类型：0x{code:02x}")]
    UnknownType { code: u8 },

    /// 载荷字节数不足 `len` 声明的长度。
    #[error("载荷被截断：声明 {declared} 字节，实际 {actual} 字节")]
    TruncatedPayload { declared: usize, actual: usize },
}

/// 一个解码出来的帧。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// 帧类型。
    pub frame_type: FrameType,
    /// 载荷（不含帧头）。
    pub payload: Vec<u8>,
}

impl Frame {
    /// 构造一帧。
    #[must_use]
    pub fn new(frame_type: FrameType, payload: Vec<u8>) -> Self {
        Self {
            frame_type,
            payload,
        }
    }

    /// 编码成线上的字节（含 8 字节帧头）。
    ///
    /// payload 超过 8 MiB 上限时返回 `None`——**不截断**。
    #[must_use]
    pub fn encode(&self) -> Option<Vec<u8>> {
        let len = u32::try_from(self.payload.len()).ok()?;
        if len > MAX_PAYLOAD_LEN {
            return None;
        }
        let mut out = Vec::with_capacity(HEADER_LEN + self.payload.len());
        out.extend_from_slice(&len.to_be_bytes());
        out.push(self.frame_type.code());
        out.extend_from_slice(&[0u8; 3]);
        out.extend_from_slice(&self.payload);
        Some(out)
    }
}

/// 从缓冲区**开头**解出一帧。
///
/// # 语义
///
/// - 头不足 8 字节 ⇒ [`FrameError::TruncatedHeader`]；
/// - `len` 超 8 MiB ⇒ [`FrameError::PayloadTooLarge`]（**不等数据、不截断**）；
/// - `reserved` 非 0 ⇒ [`FrameError::ReservedNotZero`]；
/// - `type` 未知 ⇒ [`FrameError::UnknownType`]；
/// - 载荷不足 `len` ⇒ [`FrameError::TruncatedPayload`]。
///
/// 失败时**不返回任何部分明文**。
///
/// # 长度
///
/// 成功时返回 `(帧, 本帧总字节数)`；总字节数 = `8 + len`。
/// 调用方据此从缓冲区里切走这一帧、继续解下一帧。
pub fn decode_frame(buf: &[u8]) -> Result<(Frame, usize), FrameError> {
    if buf.len() < HEADER_LEN {
        return Err(FrameError::TruncatedHeader { actual: buf.len() });
    }

    // ⭐ 大端：len 是网络序。写成小端会让固定向量立刻对不上。
    let declared = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
    if declared > MAX_PAYLOAD_LEN {
        return Err(FrameError::PayloadTooLarge { declared });
    }

    let frame_type = buf[4];
    if buf[5] != 0 || buf[6] != 0 || buf[7] != 0 {
        return Err(FrameError::ReservedNotZero);
    }

    let code = frame_type;
    let kind = FrameType::from_code(code).ok_or(FrameError::UnknownType { code })?;

    let declared_len = declared as usize;
    let total = HEADER_LEN + declared_len;
    if buf.len() < total {
        return Err(FrameError::TruncatedPayload {
            declared: declared_len,
            actual: buf.len().saturating_sub(HEADER_LEN),
        });
    }

    let payload = buf[HEADER_LEN..total].to_vec();
    Ok((Frame::new(kind, payload), total))
}

/// 缓冲区里能解出的帧**数量**（尽力而为：遇到坏帧就停下）。
///
/// 只做「有多少个完整合法的帧」这一个统计，不返回任何载荷内容。
/// 用途：连接收尾时判断「还有没有没收完的尾包」。
#[must_use]
pub fn count_complete_frames(buf: &[u8]) -> usize {
    let mut count = 0;
    let mut offset = 0usize;
    while offset < buf.len() {
        match decode_frame(&buf[offset..]) {
            Ok((_, used)) => {
                count += 1;
                offset += used;
            }
            Err(_) => break,
        }
    }
    count
}

#[cfg(test)]
mod frame_tests {
    use super::*;

    /// 合法帧的共同填充字节（线协议 §2）。
    const FILL: u8 = 0x5A;

    /// 合法帧 6 组：`(len, 帧头 8 字节)`（线协议 §2，逐字节照抄）。
    const LEGAL_HEADERS: [(u32, [u8; 8]); 6] = [
        (0, [0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00]),
        (1, [0x00, 0x00, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00]),
        (3, [0x00, 0x00, 0x00, 0x03, 0x03, 0x00, 0x00, 0x00]),
        (255, [0x00, 0x00, 0x00, 0xff, 0x03, 0x00, 0x00, 0x00]),
        (65535, [0x00, 0x00, 0xff, 0xff, 0x03, 0x00, 0x00, 0x00]),
        (8_388_608, [0x00, 0x80, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00]),
    ];

    /// 非法帧：`len` 超上限（8 MiB + 1）。
    const IL_LEN_OVER: [u8; 8] = [0x00, 0x80, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00];
    /// 非法帧：`reserved` 非 0。
    const IL_RESERVED: [u8; 9] = [0x00, 0x00, 0x00, 0x01, 0x03, 0x00, 0x00, 0x01, 0x5A];
    /// 非法帧：未知 `type`。
    const IL_TYPE: [u8; 9] = [0x00, 0x00, 0x00, 0x01, 0x07, 0x00, 0x00, 0x00, 0x5A];
    /// 非法帧：头被截断（7 字节）。
    const IL_TRUNC_HDR: [u8; 7] = [0x00, 0x00, 0x00, 0x01, 0x03, 0x00, 0x00];
    /// 非法帧：载荷不足（声明 3，实给 1）。
    const IL_TRUNC_PAYLOAD: [u8; 9] = [0x00, 0x00, 0x00, 0x03, 0x03, 0x00, 0x00, 0x00, 0x5A];

    #[test]
    fn legal_frame_headers_match_the_wire_contract_byte_for_byte() {
        for (len, header) in LEGAL_HEADERS {
            let frame = Frame::new(FrameType::ConfigBundle, vec![FILL; len as usize]);
            let encoded = frame.encode().expect("合法帧应可编码");
            assert_eq!(
                &encoded[..HEADER_LEN],
                &header[..],
                "len={len} 的帧头必须与线协议逐字节相同"
            );
            assert_eq!(encoded.len(), HEADER_LEN + len as usize);
        }
    }

    #[test]
    fn legal_frames_round_trip() {
        for (len, _header) in LEGAL_HEADERS {
            // 8 MiB 的向量太占内存，帧头由上面那个测试单独验。
            if len as usize > 65535 {
                continue;
            }
            let payload = vec![FILL; len as usize];
            let frame = Frame::new(FrameType::ConfigBundle, payload.clone());
            let encoded = frame.encode().expect("合法帧应可编码");
            let (decoded, used) = decode_frame(&encoded).expect("合法帧应可解码");
            assert_eq!(decoded.frame_type, FrameType::ConfigBundle);
            assert_eq!(decoded.payload, payload, "往返后载荷必须逐字节相同");
            assert_eq!(used, encoded.len());
        }
    }

    #[test]
    fn empty_and_small_frames_round_trip() {
        for len in [0usize, 1, 3, 255, 65535] {
            let payload: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
            let frame = Frame::new(FrameType::Event, payload.clone());
            let encoded = frame.encode().expect("合法帧应可编码");
            let (decoded, _) = decode_frame(&encoded).expect("合法帧应可解码");
            assert_eq!(decoded.payload, payload, "len={len} 往返后必须逐字节相同");
        }
    }

    #[test]
    fn payload_len_over_the_limit_is_rejected() {
        // IL_LEN_OVER：len = 8 MiB + 1。
        let err = decode_frame(&IL_LEN_OVER).expect_err("len 超上限必须失败");
        assert_eq!(
            err,
            FrameError::PayloadTooLarge {
                declared: MAX_PAYLOAD_LEN + 1
            }
        );
    }

    #[test]
    fn max_payload_len_is_exactly_legal() {
        // VMAX：len 恰为上限 8 MiB ⇒ 合法。
        let (len, header) = LEGAL_HEADERS[5];
        assert_eq!(len, MAX_PAYLOAD_LEN);
        let mut frame_bytes = header.to_vec();
        frame_bytes.extend_from_slice(&[FILL; 16]);
        // 只给 16 字节载荷但声明 8 MiB ⇒ 载荷不足（不是超限）。
        let err = decode_frame(&frame_bytes).expect_err("载荷不足必须失败");
        assert_eq!(
            err,
            FrameError::TruncatedPayload {
                declared: MAX_PAYLOAD_LEN as usize,
                actual: 16
            }
        );
    }

    #[test]
    fn reserved_bytes_must_be_zero() {
        // IL_RESERVED：reserved 的最后一字节是 0x01。
        let err = decode_frame(&IL_RESERVED).expect_err("reserved 非 0 必须失败");
        assert_eq!(err, FrameError::ReservedNotZero);
    }

    #[test]
    fn unknown_frame_type_is_rejected() {
        // IL_TYPE：type = 0x07，不是已知帧类型。
        let err = decode_frame(&IL_TYPE).expect_err("未知 type 必须失败");
        assert_eq!(err, FrameError::UnknownType { code: 0x07 });
    }

    #[test]
    fn truncated_header_is_rejected() {
        // IL_TRUNC_HDR：只有 7 字节。
        let err = decode_frame(&IL_TRUNC_HDR).expect_err("头被截断必须失败");
        assert_eq!(err, FrameError::TruncatedHeader { actual: 7 });
    }

    #[test]
    fn every_truncated_header_length_is_rejected() {
        // 1..=7 字节都必须是「头被截断」，而不是别的错。
        for len in 1..HEADER_LEN {
            let buf = vec![0x00u8; len];
            let err = decode_frame(&buf).expect_err("头不全都必须失败");
            assert_eq!(
                err,
                FrameError::TruncatedHeader { actual: len },
                "{len} 字节的头必须报「头被截断」"
            );
        }
    }

    #[test]
    fn truncated_payload_is_rejected() {
        // IL_TRUNC_PAYLOAD：声明 3，实给 1。
        let err = decode_frame(&IL_TRUNC_PAYLOAD).expect_err("载荷不足必须失败");
        assert_eq!(
            err,
            FrameError::TruncatedPayload {
                declared: 3,
                actual: 1
            }
        );
    }

    #[test]
    fn frame_length_is_big_endian() {
        // ⭐ 手工构造期望字节数组：len=1、type=0x01 ⇒ `00 00 00 01 01 00 00 00`。
        // 写成小端会得到 `01 00 00 00 ...`，这里立刻对不上。
        let expected: [u8; 9] = [0x00, 0x00, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x5A];
        let frame = Frame::new(FrameType::Hello, vec![0x5A]);
        let encoded = frame.encode().expect("应可编码");
        assert_eq!(encoded, expected.to_vec(), "帧头必须是大端（网络序）");
    }

    #[test]
    fn every_frame_type_code_matches_the_wire_contract() {
        let expected = [
            (FrameType::Hello, 0x01u8),
            (FrameType::KeyEnvelope, 0x02),
            (FrameType::ConfigBundle, 0x03),
            (FrameType::ActionRequest, 0x04),
            (FrameType::ActionResult, 0x05),
            (FrameType::Event, 0x81),
            (FrameType::EventAck, 0x82),
        ];
        for (kind, code) in expected {
            assert_eq!(kind.code(), code, "帧类型码必须与线协议一致");
            assert_eq!(FrameType::from_code(code), Some(kind));
        }
        for unknown in [0x00u8, 0x06, 0x07, 0x80, 0x83, 0xff] {
            assert_eq!(
                FrameType::from_code(unknown),
                None,
                "0x{unknown:02x} 必须未知"
            );
        }
    }

    #[test]
    fn encoder_refuses_to_truncate_an_oversized_payload() {
        // 超过 8 MiB 的载荷不能被静默截断。
        let mut payload = vec![0u8; MAX_PAYLOAD_LEN as usize + 1];
        payload[MAX_PAYLOAD_LEN as usize] = 0x01;
        let frame = Frame::new(FrameType::Event, payload);
        assert!(frame.encode().is_none(), "超限载荷必须编码失败，不得截断");
    }

    #[test]
    fn decoder_reports_how_many_complete_frames_are_buffered() {
        let a = Frame::new(FrameType::Hello, vec![1, 2, 3])
            .encode()
            .expect("应可编码");
        let b = Frame::new(FrameType::Event, vec![])
            .encode()
            .expect("应可编码");
        let mut buf = a.clone();
        buf.extend_from_slice(&b);
        assert_eq!(count_complete_frames(&buf), 2);
        // 尾部再接半个头 ⇒ 只数到 2 个完整帧。
        buf.extend_from_slice(&[0x00, 0x00]);
        assert_eq!(count_complete_frames(&buf), 2);
    }
}
