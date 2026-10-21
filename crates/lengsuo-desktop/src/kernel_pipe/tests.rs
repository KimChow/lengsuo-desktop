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
//! # ⚠️ 一处契约内部的矛盾（已写进报告，未自行拍板）
//!
//! 契约 §3.1 说「载荷总长 = 85 字节 ⇒ 帧头 `len` = `0x00000055`」，
//! 但同一节给的逐字段布局算出来是 **102 字节**（密文 45 字节）。
//! 契约同时把 **AAD 钉死成** `00000055020000000102030405060708`
//! （即 `len` = `0x55`），且那条 AAD 配出的密文/认证标签与契约给出的
//! 密文、标签**逐字节相同**（已被两套独立实现验过）。
//!
//! ⇒ 本卡的处理：**AAD 用契约钉死的 `0x55`**（这是跨仓比对的那条字节），
//! **载荷按逐字段布局实算**（102 字节）。
//! 帧头的 `len` 字段写的是**载荷实际长度**（102 = `0x66`）——
//! 这是线协议 §2 的定义（`len` = payload 字节数）。
//! 详见 `docs/kernel-pipe-core-status.md` 的「语义冲突」一节。

use crate::kernel_pipe::handshake::{envelope_aad, Handshake};
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

/// 由固定向量拼出的**信封载荷**（线协议 §3.1 的逐字段布局）。
pub(crate) fn expected_envelope_payload() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&12u32.to_be_bytes());
    out.extend_from_slice(&GCM_NONCE);
    out.extend_from_slice(&(CONTAINER_ID.len() as u32).to_be_bytes());
    out.extend_from_slice(CONTAINER_ID);
    out.extend_from_slice(&(ENVELOPE_CIPHERTEXT.len() as u32).to_be_bytes());
    out.extend_from_slice(&ENVELOPE_CIPHERTEXT);
    out.extend_from_slice(&EXPIRES_UNIX.to_be_bytes());
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
mod key_tests {
    use super::*;
    use crate::kernel_pipe::keys::{
        derive_session_key, k_machine, machine_code, random_session_key, to_hex, EnvelopeError,
        MachineInputs, SessionKeySource, SESSION_KEY_LEN,
    };

    #[test]
    fn machine_code_matches_the_wire_contract_vector() {
        let inputs = MachineInputs {
            os: "linux".to_string(),
            cpuid: "cpuid-test".to_string(),
            hddid: "hddid-test".to_string(),
            computername: "host-test".to_string(),
        };
        assert_eq!(inputs.to_json(), MACHINE_JSON, "JSON 形状与键序必须一致");
        assert_eq!(
            machine_code(&inputs),
            MACHINE_CODE,
            "机器码必须与线协议逐字节相同"
        );
    }

    #[test]
    fn machine_code_uses_the_fixed_key_order() {
        // 键序变了 ⇒ 摘要变了。四个字段的顺序是协议的一部分。
        let a = MachineInputs {
            os: "linux".to_string(),
            cpuid: "c".to_string(),
            hddid: "h".to_string(),
            computername: "n".to_string(),
        };
        assert_eq!(machine_code(&a), machine_code(&a.clone()));
        let swapped = MachineInputs {
            os: "linux".to_string(),
            cpuid: "h".to_string(),
            hddid: "c".to_string(),
            computername: "n".to_string(),
        };
        assert_ne!(machine_code(&a), machine_code(&swapped));
    }

    #[test]
    fn k_machine_matches_the_wire_contract_vector() {
        assert_eq!(to_hex(&fixed_k_machine()), K_MACHINE_HEX);
    }

    #[test]
    fn session_key_derivation_matches_the_wire_contract_vector() {
        // 候选 B：HKDF-SHA256(ikm=k_machine, salt=hello_nonce, info=...)。
        let derived = derive_session_key(&fixed_k_machine(), &HELLO_NONCE).expect("应可派生");
        assert_eq!(to_hex(&derived), SESSION_KEY_HEX);
    }

    #[test]
    fn session_key_source_is_injectable_not_hard_coded() {
        // ⭐ 生成方式是「待主理人拍板」的 ⇒ 本模块只提供两种实现，由调用方注入。
        let random = random_session_key().expect("应可生成");
        let derived = derive_session_key(&fixed_k_machine(), &HELLO_NONCE).expect("应可派生");
        assert_eq!(random.len(), SESSION_KEY_LEN);
        assert_eq!(derived.len(), SESSION_KEY_LEN);
        // 两次随机不应相同（否则就是常量了）。
        let again = random_session_key().expect("应可生成");
        assert_ne!(random, again, "CSPRNG 两次必须不同");
        // 派生的两次必须相同（HKDF 是确定性的）。
        assert_eq!(
            derived,
            derive_session_key(&fixed_k_machine(), &HELLO_NONCE).expect("应可派生")
        );
        // 换 salt（hello nonce）必须换出不同的密钥。
        let other_nonce = [0x09u8; HELLO_NONCE_LEN];
        assert_ne!(
            derived,
            derive_session_key(&fixed_k_machine(), &other_nonce).expect("应可派生")
        );
        // 显式走一遍「按指定方式生成」的入口。
        let via_random =
            keys::make_session_key(SessionKeySource::Random, &fixed_k_machine(), &HELLO_NONCE)
                .expect("应可生成");
        assert_eq!(via_random.len(), SESSION_KEY_LEN);
        let via_hkdf =
            keys::make_session_key(SessionKeySource::Hkdf, &fixed_k_machine(), &HELLO_NONCE)
                .expect("应可派生");
        assert_eq!(via_hkdf, derived);
    }

    #[test]
    fn envelope_payload_length_is_big_endian_throughout() {
        // 载荷里每个整数都必须是大端：手工核对各字段。
        let payload = expected_envelope_payload();
        assert_eq!(
            &payload[0..4],
            &[0x00, 0x00, 0x00, 0x0c],
            "nonce_len 必须大端"
        );
        assert_eq!(&payload[4..16], &GCM_NONCE);
        assert_eq!(
            &payload[16..20],
            &[0x00, 0x00, 0x00, 0x05],
            "container_id_len 必须大端"
        );
        assert_eq!(&payload[20..25], CONTAINER_ID);
        assert_eq!(
            &payload[25..29],
            &[0x00, 0x00, 0x00, 0x2d],
            "ciphertext_len 必须大端"
        );
        assert_eq!(
            &payload[74..82],
            &[0x00, 0x00, 0x00, 0x00, 0x65, 0x53, 0xf1, 0x00]
        );
        assert_eq!(
            &payload[82..86],
            &[0x00, 0x00, 0x00, 0x10],
            "tag_len 必须大端"
        );
        assert_eq!(&payload[86..102], &ENVELOPE_TAG);
        assert_eq!(payload.len(), 102, "载荷总长按逐字段布局是 102 字节");
        assert_eq!(keys::envelope_payload_len(CONTAINER_ID.len()), 102);
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
        let payload = expected_envelope_payload();
        for cut in [1usize, 4, 16, 20, 25, 29, 50, 81, 85, 101] {
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
        payload[85] = 0x0f; // tag_len = 15
        assert_eq!(
            KeyEnvelope::decode(&payload).expect_err("tag_len 必须为 16"),
            EnvelopeError::BadTagLength
        );
    }

    #[test]
    fn machine_inputs_use_this_machine_real_values() {
        // ⚠️ 只断言「形状」与「非空」：取值是本机真实值，各机不同。
        let inputs = keys::collect_machine_inputs();
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
}
