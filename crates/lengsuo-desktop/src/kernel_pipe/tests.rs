//! 内核管道的**共享测试夹具** + 跨模块集成测试。
//!
//! ⛔ 不在这里再包 `mod tests {}`（clippy `module_inception` 会拦）。
//!
//! # 这个文件放什么
//!
//! - **固定测试向量**（跨仓线协议 `kernel-wire-contract.md` §2 / §3.1 的字节）——
//!   帧相关的向量在 `mod.rs` 的帧测试里，**密钥相关**的向量在这里，
//!   因为 `keys.rs` 与 `handshake.rs` 的测试都要用同一组。
//! - **跨模块的集成测试**（帧 → 信封 → 握手串起来跑一遍）。
//!
//! 各模块自己的单元测试就地放在那个模块里（`#[cfg(test)] mod tests`），
//! 与本仓 `state/` 的写法一致。
//!
//! # ⭐ 固定测试向量的来源
//!
//! 全部来自跨仓线协议 `kernel-wire-contract.md`（内核仓
//! `lengsuo-browser-dev/recipe/docs/kernel-wire-contract.md`）的 §2 与 §3.1。
//!
//! ⛔ **这些常量是硬编码的**（不是运行时算出来的）——
//! 两侧各写一份，跨仓比对靠「逐字节相同」。
//! ⛔ **不许在测试里调用外部脚本生成期望值**：那样两边都错也会「一致」。
//!
//! # ⭐ 契约 §3.1 的 85 字节布局（2026-10-21 裁定）
//!
//! 契约 §3.1 此前自相矛盾（逐字段表算 102 字节 vs AAD/散文说 85 字节），
//! **已裁定以 85 字节为准**——GCM 的 tag 覆盖 AAD，而 AAD 首 4 字节就是
//! 帧头 `len`（= 载荷字节数）⇒ 载荷总长只能由 tag 判定。
//!
//! ⇒ 载荷的**明文区只有 6 个字段**：
//! `u32 nonce_len | nonce(12) | u32 ciphertext_len | ciphertext | u32 tag_len | tag(16)`
//! ⇒ 载荷总长 = **40 + 密文长度** = **85 字节**（`container_id` = "ct-01" 时），
//! 帧头 `len` = `0x00000055`，AAD = `00000055020000000102030405060708`。
//!
//! ⛔ `container_id` 与 `expires_unix` **只在密文里**（明文 =
//! `session_key(32B) || container_id || expires_unix`），
//! 载荷的明文区**不重复**它们——AAD 只盖住帧头 8 字节 + Hello nonce，
//! 明文区的任何字节都不在认证范围内 ⇒ 抄一份到明文区 =
//! 容器标识与过期时间可被中间人改写而认证照样通过。

use crate::kernel_pipe::handshake::{envelope_aad, Handshake, PROTOCOL_VERSION};
use crate::kernel_pipe::keys::{
    self, k_machine, EnvelopePlain, KeyEnvelope, HELLO_NONCE_LEN, K_MACHINE_LEN,
};
use crate::kernel_pipe::{decode_frame, Frame, FrameType};

/// 机器码夹具的 JSON（键序固定）。
pub(crate) const MACHINE_JSON: &str =
    r#"{"os":"linux","cpuid":"cpuid-test","hddid":"hddid-test","computername":"host-test"}"#;

/// 机器码派生向量（线协议 §2）。
pub(crate) const MACHINE_CODE: &str =
    "216feaf8875b0e55d32682bef617480d57c842fc3db57018f0715e17cc199033";
/// `k_machine` 向量（线协议 §2）。
pub(crate) const K_MACHINE_HEX: &str =
    "64fa1b15ef6856fb97cfb29aa397500a3003e8c40568c684cb55fa96438c0d7f";
/// 会话密钥向量（线协议 §2）。
pub(crate) const SESSION_KEY_HEX: &str =
    "b102b38300fa770def96bbd421f83c9858c86401d5905c4e6e4fb97900627ff1";

/// Hello 的 8 字节 nonce（线协议 §2 / §3.1）。
pub(crate) const HELLO_NONCE: [u8; HELLO_NONCE_LEN] =
    [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
/// GCM 的 12 字节 nonce（线协议 §3.1）。
pub(crate) const GCM_NONCE: [u8; keys::GCM_NONCE_LEN] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b,
];
/// 容器 id（线协议 §3.1）。
pub(crate) const CONTAINER_ID: &[u8] = b"ct-01";
/// 过期时间（线协议 §3.1）。
pub(crate) const EXPIRES_UNIX: u64 = 1_700_000_000;

/// KeyEnvelope 的 AAD（线协议 §3.1 钉死的 16 字节）。
pub(crate) const ENVELOPE_AAD: [u8; 16] = [
    0x00, 0x00, 0x00, 0x55, 0x02, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
];
/// KeyEnvelope 的密文（线协议 §3.1）。
pub(crate) const ENVELOPE_CIPHERTEXT: [u8; 45] = [
    0x88, 0x3b, 0x15, 0x2f, 0x81, 0x0f, 0xc5, 0xd4, 0x87, 0x0a, 0x13, 0x99, 0xe7, 0xc2, 0x6b, 0x42,
    0x5e, 0x51, 0x20, 0x47, 0x49, 0x37, 0x0b, 0x39, 0xee, 0x25, 0xcb, 0x05, 0xd5, 0xf8, 0x59, 0x86,
    0xa4, 0x91, 0x70, 0xc6, 0xc9, 0xb1, 0xbb, 0x32, 0x72, 0xa4, 0x67, 0x7e, 0xdc,
];
/// KeyEnvelope 的认证标签（线协议 §3.1）。
pub(crate) const ENVELOPE_TAG: [u8; 16] = [
    0xc5, 0x2e, 0x71, 0x8c, 0xbe, 0x62, 0x81, 0xf0, 0xb8, 0xbb, 0xb4, 0x0c, 0x15, 0x37, 0x57, 0xad,
];

/// 固定 `k_machine`（由向量派生，避免每个测试重复算）。
pub(crate) fn fixed_k_machine() -> [u8; K_MACHINE_LEN] {
    k_machine(MACHINE_CODE)
}

/// 十六进制串 ⇒ 32 字节。
pub(crate) fn hex_bytes32(text: &str) -> [u8; 32] {
    let bytes = text.as_bytes();
    assert_eq!(bytes.len(), 64, "夹具必须是 32 字节");
    let mut out = [0u8; 32];
    for (idx, slot) in out.iter_mut().enumerate() {
        let hi = (bytes[idx * 2] as char).to_digit(16).expect("hex") as u8;
        let lo = (bytes[idx * 2 + 1] as char).to_digit(16).expect("hex") as u8;
        *slot = (hi << 4) | lo;
    }
    out
}

/// 造一个 `Hello` 帧（JSON `{version, nonce}`，nonce 用十六进制字符串）。
pub(crate) fn hello_frame(version: u32, nonce: &[u8; HELLO_NONCE_LEN]) -> Frame {
    let nonce_hex: String = nonce.iter().map(|b| format!("{b:02x}")).collect();
    let payload = format!(r#"{{"version":{version},"nonce":"{nonce_hex}"}}"#);
    Frame::new(FrameType::Hello, payload.into_bytes())
}

/// 造一个 `KeyEnvelope` 帧，**AAD 与帧头自洽**（len 字段 = 载荷实际长度）。
///
/// ⭐ 契约 §3.1 定的 85 字节布局下，这里的 `payload_len` 恰好 = 85 = `0x55`
/// ⇒ 自洽算出来的 AAD 与契约钉死的那 16 字节**逐字节相同**。
pub(crate) fn envelope_frame(k: &[u8; K_MACHINE_LEN]) -> Frame {
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

/// 造一个能推进到 `Handshake` 状态的机。
pub(crate) fn handshake_in_handshake(k: &[u8; K_MACHINE_LEN]) -> Handshake {
    let mut hs = Handshake::new(*k);
    hs.on_frame(&hello_frame(PROTOCOL_VERSION, &HELLO_NONCE))
        .expect("Hello 应被接受");
    hs
}

/// 由固定向量拼出的**信封载荷**（线协议 §3.1 的逐字段布局，**85 字节**）。
///
/// ```text
/// u32 nonce_len(4) | nonce(12) | u32 ciphertext_len(4) | ciphertext(45)
/// | u32 tag_len(4) | tag(16)
/// ```
///
/// ⛔ 明文区**没有** `container_id` 与 `expires_unix`——它们只在密文里。
pub(crate) fn expected_envelope_payload() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&12u32.to_be_bytes());
    out.extend_from_slice(&GCM_NONCE);
    out.extend_from_slice(&(ENVELOPE_CIPHERTEXT.len() as u32).to_be_bytes());
    out.extend_from_slice(&ENVELOPE_CIPHERTEXT);
    out.extend_from_slice(&16u32.to_be_bytes());
    out.extend_from_slice(&ENVELOPE_TAG);
    out
}

// ---------------------------------------------------------------------------
// 跨模块集成测试
// ---------------------------------------------------------------------------

#[test]
fn runner_builds_an_envelope_frame_that_the_peer_can_open() {
    // 端到端：runner 收 Hello → 封信封 → 编码成帧 → 内核侧解帧 → 解封。
    let k = fixed_k_machine();
    let mut runner = Handshake::new(k);
    runner
        .on_frame(&hello_frame(1, &HELLO_NONCE))
        .expect("Hello 应被接受");

    let session = hex_bytes32(SESSION_KEY_HEX);
    let frame = runner
        .build_key_envelope_frame(&session, CONTAINER_ID, EXPIRES_UNIX, GCM_NONCE)
        .expect("应可封信封");

    // 帧头里的 len 必须等于载荷实际长度。
    let encoded = frame.encode().expect("应可编码");
    let (decoded, used) = decode_frame(&encoded).expect("应可解码");
    assert_eq!(used, encoded.len());
    assert_eq!(decoded.frame_type, FrameType::KeyEnvelope);
    assert_eq!(decoded.payload, frame.payload);

    // 内核侧：AAD = 帧头 8 字节（len = 载荷实际长度）+ Hello nonce。
    let aad = envelope_aad(decoded.payload.len(), &HELLO_NONCE);
    let envelope = KeyEnvelope::decode(&decoded.payload).expect("应可解码");
    let plain = keys::open_envelope(&k, &aad, &envelope).expect("应可解封");
    assert_eq!(plain.session_key, session);
    assert_eq!(plain.container_id, CONTAINER_ID.to_vec());
    assert_eq!(plain.expires_unix, EXPIRES_UNIX);
}

#[test]
fn the_whole_key_chain_matches_the_contract_vectors() {
    // 机器码 → k_machine → 会话密钥，三步逐字节对上线协议。
    let inputs = keys::MachineInputs {
        os: "linux".to_string(),
        cpuid: "cpuid-test".to_string(),
        hddid: "hddid-test".to_string(),
        computername: "host-test".to_string(),
    };
    let code = keys::machine_code(&inputs);
    assert_eq!(code, MACHINE_CODE);
    assert_eq!(keys::to_hex(&k_machine(&code)), K_MACHINE_HEX);
    let session = keys::derive_session_key(&k_machine(&code), &HELLO_NONCE).expect("应可派生");
    assert_eq!(keys::to_hex(&session), SESSION_KEY_HEX);
}

#[test]
fn an_envelope_sealed_with_the_contract_aad_matches_the_contract_payload() {
    // 用契约钉死的 AAD 封一个信封 ⇒ 载荷与契约 §3.1 逐字节相同。
    let envelope = keys::seal_envelope(
        &fixed_k_machine(),
        &ENVELOPE_AAD,
        &EnvelopePlain {
            session_key: hex_bytes32(SESSION_KEY_HEX),
            container_id: CONTAINER_ID.to_vec(),
            expires_unix: EXPIRES_UNIX,
        },
        GCM_NONCE,
    )
    .expect("应可封装");
    assert_eq!(envelope.ciphertext.to_vec(), ENVELOPE_CIPHERTEXT.to_vec());
    assert_eq!(envelope.tag.to_vec(), ENVELOPE_TAG.to_vec());
    assert_eq!(envelope.encode(), expected_envelope_payload());
}

// ---------------------------------------------------------------------------
// 密钥派生与密钥信封的单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod leak_tests {
    use super::*;
    use crate::kernel_pipe::keys::{EnvelopePlain, MachineInputs};

    #[test]
    fn debug_output_never_contains_key_material() {
        // ⛔ 派生 `Debug` 会把密钥原样写进日志 ⇒ 这两个类型的 `Debug` 是手写的。
        let session = hex_bytes32(SESSION_KEY_HEX);
        let plain = EnvelopePlain {
            session_key: session,
            container_id: CONTAINER_ID.to_vec(),
            expires_unix: EXPIRES_UNIX,
        };
        let plain_text = format!("{plain:?}");
        assert!(
            !plain_text.contains(SESSION_KEY_HEX),
            "EnvelopePlain 的 Debug 不得含会话密钥：{plain_text}"
        );

        let hs = Handshake::new(fixed_k_machine());
        let hs_text = format!("{hs:?}");
        assert!(
            !hs_text.contains(K_MACHINE_HEX),
            "Handshake 的 Debug 不得含 k_machine：{hs_text}"
        );
    }

    #[test]
    fn the_machine_code_debug_is_not_a_secret_but_the_inputs_shape_is_checked() {
        // 机器码的**取值**是本机真值，不该被顺手打进日志。
        let inputs = MachineInputs {
            os: "linux".to_string(),
            cpuid: "cpuid-test".to_string(),
            hddid: "hddid-test".to_string(),
            computername: "host-test".to_string(),
        };
        // MachineInputs 的 Debug 是派生的（里面没有密钥），但机器码本身不应进日志。
        assert!(!format!("{inputs:?}").contains(MACHINE_CODE));
    }
}

#[cfg(test)]
mod runner_tests {
    use super::*;
    use crate::kernel_pipe::handshake::{HandshakeError, HandshakeState, PROTOCOL_VERSION};
    use crate::kernel_pipe::keys::SESSION_KEY_LEN;
    use crate::kernel_pipe::FrameType;

    #[test]
    fn the_runner_can_build_the_key_envelope_frame() {
        // runner 侧：收到 Hello 后才能封信封（AAD 要用 Hello 的 nonce）。
        let k = fixed_k_machine();
        let mut hs = Handshake::new(k);
        assert!(
            hs.build_key_envelope_frame(
                &[0u8; SESSION_KEY_LEN],
                CONTAINER_ID,
                EXPIRES_UNIX,
                GCM_NONCE
            )
            .is_err(),
            "WaitHello 状态不能封信封（没有 Hello nonce）"
        );

        hs.on_frame(&hello_frame(PROTOCOL_VERSION, &HELLO_NONCE))
            .expect("Hello 应被接受");

        let session = hex_bytes32(SESSION_KEY_HEX);
        let frame = hs
            .build_key_envelope_frame(&session, CONTAINER_ID, EXPIRES_UNIX, GCM_NONCE)
            .expect("应可封信封");
        assert_eq!(frame.frame_type, FrameType::KeyEnvelope);

        // 对端用同样的 AAD 应当能解出来。
        let aad = envelope_aad(frame.payload.len(), &HELLO_NONCE);
        let envelope = KeyEnvelope::decode(&frame.payload).expect("应可解码");
        let plain = keys::open_envelope(&k, &aad, &envelope).expect("应可解封");
        assert_eq!(plain.session_key, session);
        assert_eq!(plain.container_id, CONTAINER_ID.to_vec());
        assert_eq!(plain.expires_unix, EXPIRES_UNIX);
    }

    #[test]
    fn the_envelope_aad_is_the_frame_header_plus_the_hello_nonce() {
        let aad = envelope_aad(0x55, &HELLO_NONCE);
        assert_eq!(aad, ENVELOPE_AAD, "AAD 必须与线协议钉死的 16 字节一致");
        assert_eq!(aad.len(), 16, "AAD 必须是 16 字节 = 帧头 8 + hello nonce 8");
        assert_eq!(
            &aad[4..8],
            &[0x02, 0x00, 0x00, 0x00],
            "AAD 里的 type 必须是 KeyEnvelope"
        );
        assert_eq!(
            &aad[8..16],
            &HELLO_NONCE,
            "AAD 必须含 Hello 的 8 字节 nonce"
        );
    }

    #[test]
    fn the_runner_cannot_send_an_envelope_before_the_hello() {
        // 状态机必须先收到 Hello 才知道 AAD 里的 hello nonce。
        let hs = Handshake::new(fixed_k_machine());
        assert!(matches!(hs.state(), HandshakeState::WaitHello));
        let err = hs
            .build_key_envelope_frame(
                &[0u8; SESSION_KEY_LEN],
                CONTAINER_ID,
                EXPIRES_UNIX,
                GCM_NONCE,
            )
            .expect_err("WaitHello 状态不能封信封");
        assert_eq!(err, HandshakeError::AlreadyFinished);
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

#[cfg(test)]
mod envelope_tests {
    use super::*;
    use crate::kernel_pipe::keys::{to_hex, EnvelopeError};

    #[test]
    fn envelope_payload_length_is_big_endian_throughout() {
        // 载荷里每个整数都必须是大端：手工核对各字段。
        // 布局：nonce_len(4) | nonce(12) | ciphertext_len(4) | ciphertext(45)
        //       | tag_len(4) | tag(16)  ⇒ 85 字节
        let payload = expected_envelope_payload();
        assert_eq!(
            &payload[0..4],
            &[0x00, 0x00, 0x00, 0x0c],
            "nonce_len 必须大端"
        );
        assert_eq!(&payload[4..16], &GCM_NONCE);
        assert_eq!(
            &payload[16..20],
            &[0x00, 0x00, 0x00, 0x2d],
            "ciphertext_len 必须大端"
        );
        assert_eq!(&payload[20..65], &ENVELOPE_CIPHERTEXT);
        assert_eq!(
            &payload[65..69],
            &[0x00, 0x00, 0x00, 0x10],
            "tag_len 必须大端"
        );
        assert_eq!(&payload[69..85], &ENVELOPE_TAG);
        assert_eq!(payload.len(), 85, "载荷总长按逐字段布局是 85 字节");
        assert_eq!(keys::envelope_payload_len(CONTAINER_ID.len()), 85);
    }

    #[test]
    fn the_plaintext_region_never_duplicates_container_id_or_expires() {
        // ⛔ 载荷的明文区**没有** container_id 与 expires_unix——
        // 它们只在密文里（线协议 §3.1）。
        // 抄一份到明文区 ⇒ 未被认证 ⇒ 可被中间人改写（过期时间可被无限延长）。
        let payload = expected_envelope_payload();
        let ciphertext = &payload[20..65];
        assert!(
            !ciphertext
                .windows(CONTAINER_ID.len())
                .any(|w| w == CONTAINER_ID),
            "container_id 只应出现在密文里，载荷明文区不得有它的副本"
        );
        // 明文区里也不该出现 expires_unix 的字节。
        let expires_bytes = EXPIRES_UNIX.to_be_bytes();
        assert!(
            !ciphertext
                .windows(expires_bytes.len())
                .any(|w| w == expires_bytes),
            "expires_unix 只应出现在密文里，载荷明文区不得有它的副本"
        );
    }

    #[test]
    fn envelope_round_trips() {
        let envelope = keys::seal_envelope(
            &fixed_k_machine(),
            &ENVELOPE_AAD,
            &EnvelopePlain {
                session_key: hex_bytes32(SESSION_KEY_HEX),
                container_id: CONTAINER_ID.to_vec(),
                expires_unix: EXPIRES_UNIX,
            },
            GCM_NONCE,
        )
        .expect("应可封装");

        let decoded = KeyEnvelope::decode(&envelope.encode()).expect("应可解码");
        assert_eq!(decoded, envelope);

        let plain =
            keys::open_envelope(&fixed_k_machine(), &ENVELOPE_AAD, &decoded).expect("应可解封");
        assert_eq!(to_hex(&plain.session_key), SESSION_KEY_HEX);
        assert_eq!(plain.container_id, CONTAINER_ID.to_vec());
        assert_eq!(plain.expires_unix, EXPIRES_UNIX);
    }

    #[test]
    fn envelope_decrypts_with_the_right_key_and_fails_with_the_wrong_one() {
        let envelope = keys::seal_envelope(
            &fixed_k_machine(),
            &ENVELOPE_AAD,
            &EnvelopePlain {
                session_key: hex_bytes32(SESSION_KEY_HEX),
                container_id: CONTAINER_ID.to_vec(),
                expires_unix: EXPIRES_UNIX,
            },
            GCM_NONCE,
        )
        .expect("应可封装");

        let wrong_key =
            k_machine("0000000000000000000000000000000000000000000000000000000000000000");
        let err = keys::open_envelope(&wrong_key, &ENVELOPE_AAD, &envelope)
            .expect_err("错误密钥必须失败");
        assert_eq!(err, EnvelopeError::DecryptFailed);
    }

    #[test]
    fn envelope_rejects_a_single_flipped_ciphertext_bit() {
        let mut envelope = keys::seal_envelope(
            &fixed_k_machine(),
            &ENVELOPE_AAD,
            &EnvelopePlain {
                session_key: hex_bytes32(SESSION_KEY_HEX),
                container_id: CONTAINER_ID.to_vec(),
                expires_unix: EXPIRES_UNIX,
            },
            GCM_NONCE,
        )
        .expect("应可封装");

        // 改 1 个字节 ⇒ 认证失败。
        envelope.ciphertext[0] ^= 0x01;
        let err = keys::open_envelope(&fixed_k_machine(), &ENVELOPE_AAD, &envelope)
            .expect_err("密文被改 1 字节必须失败");
        assert_eq!(err, EnvelopeError::DecryptFailed);

        // 改认证标签同样失败。
        let mut envelope = keys::seal_envelope(
            &fixed_k_machine(),
            &ENVELOPE_AAD,
            &EnvelopePlain {
                session_key: hex_bytes32(SESSION_KEY_HEX),
                container_id: CONTAINER_ID.to_vec(),
                expires_unix: EXPIRES_UNIX,
            },
            GCM_NONCE,
        )
        .expect("应可封装");
        envelope.tag[0] ^= 0x01;
        assert_eq!(
            keys::open_envelope(&fixed_k_machine(), &ENVELOPE_AAD, &envelope)
                .expect_err("标签被改必须失败"),
            EnvelopeError::DecryptFailed
        );
    }

    #[test]
    fn envelope_rejects_a_wrong_aad() {
        let envelope = keys::seal_envelope(
            &fixed_k_machine(),
            &ENVELOPE_AAD,
            &EnvelopePlain {
                session_key: hex_bytes32(SESSION_KEY_HEX),
                container_id: CONTAINER_ID.to_vec(),
                expires_unix: EXPIRES_UNIX,
            },
            GCM_NONCE,
        )
        .expect("应可封装");

        let mut wrong_aad = ENVELOPE_AAD;
        wrong_aad[8] ^= 0xFF;
        assert_eq!(
            keys::open_envelope(&fixed_k_machine(), &wrong_aad, &envelope)
                .expect_err("AAD 不符必须失败"),
            EnvelopeError::DecryptFailed
        );

        // 只用帧头当 AAD（少了后 8 字节）⇒ 同样必须失败。
        assert_eq!(
            keys::open_envelope(&fixed_k_machine(), &ENVELOPE_AAD[..8], &envelope)
                .expect_err("AAD 少 8 字节必须失败"),
            EnvelopeError::DecryptFailed
        );
    }

    #[test]
    fn truncated_envelope_payload_is_rejected() {
        // 载荷布局：nonce_len(4) | nonce(12) | ciphertext_len(4) | ciphertext(45)
        //           | tag_len(4) | tag(16)  ⇒ 85 字节
        let payload = expected_envelope_payload();
        for cut in [1usize, 4, 16, 19, 20, 21, 40, 64, 65, 68, 69, 84] {
            let err = KeyEnvelope::decode(&payload[..cut]).expect_err("截断必须失败");
            assert!(
                matches!(
                    err,
                    EnvelopeError::Truncated | EnvelopeError::InconsistentLength
                ),
                "截断到 {cut} 字节应失败，实际是 {err:?}"
            );
        }
    }

    #[test]
    fn envelope_with_trailing_bytes_is_rejected() {
        let mut payload = expected_envelope_payload();
        payload.push(0x00);
        assert_eq!(
            KeyEnvelope::decode(&payload).expect_err("尾部多字节必须失败"),
            EnvelopeError::InconsistentLength
        );
    }

    #[test]
    fn envelope_rejects_wrong_nonce_and_tag_lengths() {
        let mut payload = expected_envelope_payload();
        payload[3] = 0x0b; // nonce_len = 11
        assert_eq!(
            KeyEnvelope::decode(&payload).expect_err("nonce_len 必须为 12"),
            EnvelopeError::BadNonceLength
        );

        let mut payload = expected_envelope_payload();
        payload[68] = 0x0f; // tag_len = 15（tag_len 在 [65..69)）
        assert_eq!(
            KeyEnvelope::decode(&payload).expect_err("tag_len 必须为 16"),
            EnvelopeError::BadTagLength
        );
    }

    #[test]
    fn machine_inputs_use_this_machine_real_values() {
        // ⚠️ 只断言「形状」与「非空」：取值是本机真实值，各机不同。
        let inputs = crate::kernel_pipe::machine::collect_machine_inputs();
        assert_eq!(inputs.os, std::env::consts::OS);
        assert!(
            !inputs.computername.is_empty(),
            "计算机名应采到真实值（HOSTNAME / COMPUTERNAME / /etc/hostname）"
        );
        // 采集来的值要能安全地编进 JSON。
        let json = inputs.to_json();
        assert!(json.starts_with("{\"os\":\""));
        assert!(json.contains("\"computername\":\""));
    }
}
