//! 内核管道纯逻辑层的单元测试。
//!
//! ⭐ **固定测试向量逐字节抄自跨仓线协议**
//! `docs/kernel-wire-contract.md`（内核仓 `lengsuo-browser-dev/recipe`）。
//! 那是**唯一真相**：内核侧（任务 Q）用同一组常量，跨仓互验靠它。
//! ⛔ 不许在本文件里调用外部脚本生成期望值——那样两边都错也会「一致」。

use super::handshake::{
    parse_hello, Handshake, HandshakeError, HandshakeOutcome, HandshakeState, PROTOCOL_VERSION,
};
use super::keys::{
    self, derive_session_key, k_machine, machine_code, random_nonce, random_session_key,
    seal_envelope, EnvelopePlaintext, KeyEnvelope, GCM_NONCE_LEN, GCM_TAG_LEN, K_MACHINE_LEN,
    SESSION_KEY_LEN,
};
use super::{
    decode, decode_prefix, encode, Frame, FrameError, FRAME_HEADER_LEN, FRAME_TYPE_CONFIG_BUNDLE,
    FRAME_TYPE_EVENT, FRAME_TYPE_EVENT_ACK, FRAME_TYPE_HELLO, FRAME_TYPE_KEY_ENVELOPE,
    MAX_PAYLOAD_LEN,
};

// === 线协议固定向量（kernel-wire-contract.md §2） ===

/// 合法帧的共同填充字节。
const FILL: u8 = 0x5A;
/// 合法帧的固定帧类型（ConfigBundle）。
const VECTOR_FRAME_TYPE: u8 = FRAME_TYPE_CONFIG_BUNDLE;

/// 合法帧向量表：`(名称, len, 期望的 8 字节帧头)`。
///
/// 逐字节抄自线协议 §2「⭐ 固定测试向量」。
const LEGAL_VECTORS: &[(&str, u32, [u8; 8])] = &[
    ("V0", 0, [0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00]),
    ("V1", 1, [0x00, 0x00, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00]),
    ("V3", 3, [0x00, 0x00, 0x00, 0x03, 0x03, 0x00, 0x00, 0x00]),
    (
        "V255",
        255,
        [0x00, 0x00, 0x00, 0xff, 0x03, 0x00, 0x00, 0x00],
    ),
    (
        "V65535",
        65535,
        [0x00, 0x00, 0xff, 0xff, 0x03, 0x00, 0x00, 0x00],
    ),
    (
        "VMAX",
        8_388_608,
        [0x00, 0x80, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00],
    ),
];

/// 非法帧向量表：`(名称, 字节, 期望错误)`。
///
/// 逐字节抄自线协议 §2「非法帧」。
const ILLEGAL_VECTORS: &[(&str, &[u8], FrameError)] = &[
    (
        "IL_LEN_OVER",
        &[0x00, 0x80, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00],
        FrameError::PayloadTooLarge,
    ),
    (
        "IL_RESERVED",
        &[0x00, 0x00, 0x00, 0x01, 0x03, 0x00, 0x00, 0x01, 0x5a],
        FrameError::ReservedNotZero,
    ),
    (
        "IL_TYPE",
        &[0x00, 0x00, 0x00, 0x01, 0x07, 0x00, 0x00, 0x00, 0x5a],
        FrameError::UnknownType(0x07),
    ),
    (
        "IL_TRUNC_HDR",
        &[0x00, 0x00, 0x00, 0x01, 0x03, 0x00, 0x00],
        FrameError::HeaderTruncated,
    ),
    (
        "IL_TRUNC_PAYLOAD",
        &[0x00, 0x00, 0x00, 0x03, 0x03, 0x00, 0x00, 0x00, 0x5a],
        FrameError::PayloadTruncated,
    ),
];

// === 帧编解码 ===

#[test]
fn legal_frame_vectors_match_the_contract_byte_for_byte() {
    for (name, len, expected_header) in LEGAL_VECTORS {
        let payload = vec![FILL; *len as usize];
        let bytes = encode(VECTOR_FRAME_TYPE, &payload).expect("合法帧必须能编码");
        assert_eq!(
            &bytes[..FRAME_HEADER_LEN],
            &expected_header[..],
            "向量 {name} 的帧头与线协议不一致"
        );
        assert_eq!(bytes.len(), FRAME_HEADER_LEN + *len as usize);
    }
}

#[test]
fn legal_frames_round_trip() {
    for (name, len, _) in LEGAL_VECTORS {
        let payload = vec![FILL; *len as usize];
        let bytes = encode(VECTOR_FRAME_TYPE, &payload).expect("合法帧必须能编码");
        let frame = decode(&bytes).expect("合法帧必须能解码");
        assert_eq!(frame.frame_type(), VECTOR_FRAME_TYPE, "向量 {name}");
        assert_eq!(frame.payload(), payload.as_slice(), "向量 {name}");
        assert_eq!(frame.encode(), bytes, "向量 {name} 往返后字节必须相同");
    }
}

#[test]
fn len_is_big_endian() {
    // ⭐ 字节序自检（线协议 §2）：len=1 必须是 `00 00 00 01`。
    // 写成小端（`01 00 00 00`）这一条立刻失败。
    let bytes = encode(VECTOR_FRAME_TYPE, &[FILL]).expect("编码");
    assert_eq!(
        &bytes[..4],
        &[0x00, 0x00, 0x00, 0x01],
        "len 字段必须是大端（网络序）"
    );
    assert_ne!(&bytes[..4], &[0x01, 0x00, 0x00, 0x00]);
}

#[test]
fn empty_and_small_payloads_round_trip() {
    for len in [0usize, 1, 3, 255, 65535] {
        let payload = vec![FILL; len];
        let bytes = encode(VECTOR_FRAME_TYPE, &payload).expect("编码");
        let frame = decode(&bytes).expect("解码");
        assert_eq!(frame.payload().len(), len);
        assert_eq!(frame.payload(), payload.as_slice());
    }
}

#[test]
fn illegal_frame_vectors_are_rejected() {
    for (name, bytes, expected) in ILLEGAL_VECTORS {
        let actual = decode(bytes).expect_err("非法帧必须解码失败");
        assert_eq!(actual, *expected, "向量 {name} 的错误类型不对");
    }
}

#[test]
fn truncated_headers_of_every_length_are_rejected() {
    // 1..7 字节的头都是截断。
    for len in 1..FRAME_HEADER_LEN {
        let bytes: Vec<u8> = (0..len).map(|i| i as u8).collect();
        assert_eq!(
            decode(&bytes).expect_err("截断的头必须失败"),
            FrameError::HeaderTruncated,
            "头长 {len} 字节应当判为截断"
        );
    }
}

#[test]
fn payload_over_the_limit_is_rejected_not_truncated() {
    // 声明 8 MiB + 1 ⇒ 失败；**不得**截断成 8 MiB 返回。
    let bytes = encode(FRAME_TYPE_CONFIG_BUNDLE, &vec![FILL; MAX_PAYLOAD_LEN + 1]);
    assert_eq!(
        bytes.expect_err("超限的 payload 必须编码失败"),
        FrameError::PayloadTooLarge
    );

    // 线上的帧：len 字段超限 ⇒ 解码失败，且不得返回部分明文。
    let mut header = [0u8; FRAME_HEADER_LEN];
    header[..4].copy_from_slice(&((MAX_PAYLOAD_LEN as u32) + 1).to_be_bytes());
    header[4] = FRAME_TYPE_CONFIG_BUNDLE;
    assert_eq!(
        decode(&header).expect_err("超限的 len 必须解码失败"),
        FrameError::PayloadTooLarge
    );
}

#[test]
fn payload_at_the_limit_is_legal() {
    // 8 MiB 恰为上限，**合法**（线协议 §2）。
    let bytes = encode(FRAME_TYPE_CONFIG_BUNDLE, &vec![FILL; MAX_PAYLOAD_LEN]).expect("上限合法");
    let frame = decode(&bytes).expect("上限帧必须能解码");
    assert_eq!(frame.payload().len(), MAX_PAYLOAD_LEN);
}

#[test]
fn unknown_frame_types_are_rejected() {
    for bad_type in [0x00u8, 0x06, 0x07, 0x80, 0x83, 0xff] {
        let mut bytes = vec![0x00, 0x00, 0x00, 0x00, bad_type, 0x00, 0x00, 0x00];
        bytes.push(0x5a);
        assert_eq!(
            decode(&bytes).expect_err("未知 type 必须解码失败"),
            FrameError::UnknownType(bad_type)
        );
        assert_eq!(
            encode(bad_type, &[]).expect_err("未知 type 必须编码失败"),
            FrameError::UnknownType(bad_type)
        );
    }
}

#[test]
fn all_seven_known_frame_types_round_trip() {
    for frame_type in [
        FRAME_TYPE_HELLO,
        FRAME_TYPE_KEY_ENVELOPE,
        FRAME_TYPE_CONFIG_BUNDLE,
        0x04,
        0x05,
        FRAME_TYPE_EVENT,
        FRAME_TYPE_EVENT_ACK,
    ] {
        let bytes = encode(frame_type, b"{}").expect("已知 type 必须能编码");
        let frame = decode(&bytes).expect("已知 type 必须能解码");
        assert_eq!(frame.frame_type(), frame_type);
    }
}

#[test]
fn trailing_bytes_after_a_frame_are_rejected() {
    let mut bytes = encode(VECTOR_FRAME_TYPE, &[FILL]).expect("编码");
    bytes.push(0x00);
    assert_eq!(
        decode(&bytes).expect_err("尾部多余字节必须失败"),
        FrameError::TrailingBytes
    );
}

#[test]
fn decode_prefix_returns_the_consumed_length() {
    let first = encode(VECTOR_FRAME_TYPE, &[FILL, FILL]).expect("编码");
    let second = encode(FRAME_TYPE_EVENT, b"{}").expect("编码");
    let mut buffer = first.clone();
    buffer.extend_from_slice(&second);

    let (frame, consumed) = decode_prefix(&buffer).expect("第一帧应能解出");
    assert_eq!(frame.payload(), &[FILL, FILL]);
    assert_eq!(consumed, first.len());
    assert_eq!(&buffer[consumed..], &second[..]);
}

// === 机器码 / 密钥派生 ===

/// 线协议 §2「⭐ 密钥派生固定测试向量」的输入。
fn contract_machine_inputs() -> keys::MachineInputs {
    keys::MachineInputs {
        os: "linux".to_string(),
        cpuid: "cpuid-test".to_string(),
        hddid: "hddid-test".to_string(),
        computername: "host-test".to_string(),
    }
}

#[test]
fn collected_machine_inputs_use_real_local_values() {
    // ⛔ 取值必须来自本机（不是常量占位串），否则 k_machine 在所有机器上都一样。
    let collected = keys::collect_machine_inputs();
    assert_eq!(collected.os, std::env::consts::OS, "os 必须取本机真值");
    assert!(!collected.os.is_empty());
    // JSON 形状仍是四字段、键序不变。
    let json = collected.to_json();
    assert!(json.starts_with(r#"{"os":"#), "键序：os 必须最前");
    assert!(json.contains(r#","cpuid":"#), "键序：cpuid 必须在 os 之后");
    assert!(
        json.contains(r#","hddid":"#),
        "键序：hddid 必须在 cpuid 之后"
    );
    assert!(
        json.contains(r#","computername":"#),
        "键序：computername 必须在最后"
    );
    assert!(json.ends_with('}'));
    // 派生出的机器码必须是 64 位十六进制。
    let code = machine_code(&collected);
    assert_eq!(code.len(), 64);
    assert!(code.chars().all(|c| c.is_ascii_hexdigit()));
    assert!(
        code.chars().all(|c| !c.is_ascii_uppercase()),
        "十六进制必须小写"
    );
}

#[test]
fn machine_code_differs_for_different_inputs() {
    let a = machine_code(&contract_machine_inputs());
    let mut other = contract_machine_inputs();
    other.hddid = "another-disk".to_string();
    let b = machine_code(&other);
    assert_ne!(a, b, "输入不同 ⇒ 机器码必须不同");
}

#[test]
fn machine_code_matches_the_contract_vector() {
    // 线协议：machine_code = 216feaf8...9033
    assert_eq!(
        machine_code(&contract_machine_inputs()),
        "216feaf8875b0e55d32682bef617480d57c842fc3db57018f0715e17cc199033"
    );
}

#[test]
fn machine_code_json_shape_and_key_order_are_fixed() {
    assert_eq!(
        contract_machine_inputs().to_json(),
        r#"{"os":"linux","cpuid":"cpuid-test","hddid":"hddid-test","computername":"host-test"}"#
    );
}

#[test]
fn k_machine_matches_the_contract_vector() {
    // 线协议：k_machine = 64fa1b15...0d7f
    let derived = k_machine("216feaf8875b0e55d32682bef617480d57c842fc3db57018f0715e17cc199033");
    assert_eq!(
        hex(derived.as_slice()),
        "64fa1b15ef6856fb97cfb29aa397500a3003e8c40568c684cb55fa96438c0d7f"
    );
}

#[test]
fn session_key_matches_the_contract_vector() {
    // 线协议：salt = 8 字节 nonce 01..08，info = "lengsuo-kernel-session-v1"
    let k_machine_bytes =
        k_machine("216feaf8875b0e55d32682bef617480d57c842fc3db57018f0715e17cc199033");
    let nonce = [1u8, 2, 3, 4, 5, 6, 7, 8];
    let session = derive_session_key(&k_machine_bytes, &nonce).expect("派生");
    assert_eq!(
        hex(session.as_slice()),
        "b102b38300fa770def96bbd421f83c9858c86401d5905c4e6e4fb97900627ff1"
    );
}

#[test]
fn session_key_derivation_depends_on_the_nonce() {
    let k = k_machine("216feaf8875b0e55d32682bef617480d57c842fc3db57018f0715e17cc199033");
    let a = derive_session_key(&k, &[1u8, 2, 3, 4, 5, 6, 7, 8]).expect("派生");
    let b = derive_session_key(&k, &[8u8, 7, 6, 5, 4, 3, 2, 1]).expect("派生");
    assert_ne!(a.as_slice(), b.as_slice(), "salt 变了，派生值必须变");
}

#[test]
fn random_session_key_is_32_bytes_and_differs_between_calls() {
    let a = random_session_key().expect("随机源");
    let b = random_session_key().expect("随机源");
    assert_eq!(a.len(), SESSION_KEY_LEN);
    assert_eq!(b.len(), SESSION_KEY_LEN);
    assert_ne!(a.as_slice(), b.as_slice(), "两次随机会话密钥不得相同");
}

#[test]
fn random_nonce_is_8_bytes_and_differs_between_calls() {
    let a = random_nonce().expect("随机源");
    let b = random_nonce().expect("随机源");
    assert_eq!(a.len(), 8);
    assert_ne!(a.as_slice(), b.as_slice());
}

// === 密钥信封 ===

/// 造一个测试用的信封明文。
fn sample_plaintext() -> EnvelopePlaintext {
    EnvelopePlaintext {
        container_id: "container-001".to_string(),
        session_key: [0x42; SESSION_KEY_LEN],
        expires_unix: 1_900_000_000,
    }
}

#[test]
fn envelope_round_trips() {
    let k = k_machine("216feaf8875b0e55d32682bef617480d57c842fc3db57018f0715e17cc199033");
    let plain = sample_plaintext();
    let envelope = seal_envelope(&k, &plain).expect("封");
    let opened = keys::open_envelope(&k, &envelope).expect("开");
    assert_eq!(opened, plain);
}

#[test]
fn envelope_survives_the_wire_format() {
    let k = k_machine("216feaf8875b0e55d32682bef617480d57c842fc3db57018f0715e17cc199033");
    let plain = sample_plaintext();
    let envelope = seal_envelope(&k, &plain).expect("封");
    let bytes = envelope.encode();

    let decoded = KeyEnvelope::decode(&bytes).expect("解码信封");
    assert_eq!(decoded.container_id(), plain.container_id);
    assert_eq!(decoded.expires_unix(), plain.expires_unix);
    assert_eq!(decoded.tag_len(), 16);
    let opened = keys::open_envelope(&k, &decoded).expect("开");
    assert_eq!(opened, plain);
}

#[test]
fn envelope_never_carries_the_session_key_in_cleartext() {
    let k = k_machine("216feaf8875b0e55d32682bef617480d57c842fc3db57018f0715e17cc199033");
    let plain = sample_plaintext();
    let bytes = seal_envelope(&k, &plain).expect("封").encode();
    // 会话密钥本体不得出现在信封的任何位置。
    assert!(
        !bytes
            .windows(SESSION_KEY_LEN)
            .any(|w| w == plain.session_key),
        "信封里不得出现明文会话密钥"
    );
}

#[test]
fn wrong_key_fails_to_open() {
    let k = k_machine("216feaf8875b0e55d32682bef617480d57c842fc3db57018f0715e17cc199033");
    let other = k_machine("0000000000000000000000000000000000000000000000000000000000000000");
    let envelope = seal_envelope(&k, &sample_plaintext()).expect("封");
    assert_eq!(
        keys::open_envelope(&other, &envelope).expect_err("错误密钥必须失败"),
        keys::KeyError::AuthenticationFailed
    );
}

#[test]
fn flipping_one_ciphertext_byte_fails_to_open() {
    let k = k_machine("216feaf8875b0e55d32682bef617480d57c842fc3db57018f0715e17cc199033");
    let envelope = seal_envelope(&k, &sample_plaintext()).expect("封");
    let mut bytes = envelope.encode();
    // 改**密文**里的一个字节（不动任何长度字段 ⇒ 结构仍合法）。
    let id_len = "container-001".len();
    let cipher_start = 4 + id_len + 4 + GCM_NONCE_LEN;
    bytes[cipher_start] ^= 0x01;

    let decoded = KeyEnvelope::decode(&bytes).expect("结构仍合法");
    assert_eq!(
        keys::open_envelope(&k, &decoded).expect_err("密文改 1 字节必须失败"),
        keys::KeyError::AuthenticationFailed
    );
}

#[test]
fn flipping_one_tag_byte_fails_to_open() {
    let k = k_machine("216feaf8875b0e55d32682bef617480d57c842fc3db57018f0715e17cc199033");
    let envelope = seal_envelope(&k, &sample_plaintext()).expect("封");
    let mut bytes = envelope.encode();
    // 改**标签**里的一个字节（标签是最后 16 字节）。
    let tag_start = bytes.len() - GCM_TAG_LEN;
    bytes[tag_start] ^= 0x80;

    let decoded = KeyEnvelope::decode(&bytes).expect("结构仍合法");
    assert_eq!(
        keys::open_envelope(&k, &decoded).expect_err("标签改 1 字节必须失败"),
        keys::KeyError::AuthenticationFailed
    );
}

#[test]
fn truncated_envelopes_are_rejected() {
    let k = k_machine("216feaf8875b0e55d32682bef617480d57c842fc3db57018f0715e17cc199033");
    let bytes = seal_envelope(&k, &sample_plaintext()).expect("封").encode();
    for len in 0..bytes.len() {
        assert!(
            KeyEnvelope::decode(&bytes[..len]).is_err(),
            "截断到 {len} 字节的信封必须被拒"
        );
    }
}

#[test]
fn envelope_errors_never_leak_the_key_bytes() {
    let k = k_machine("216feaf8875b0e55d32682bef617480d57c842fc3db57018f0715e17cc199033");
    let envelope = seal_envelope(&k, &sample_plaintext()).expect("封");
    let other = k_machine("0000000000000000000000000000000000000000000000000000000000000000");
    let err = keys::open_envelope(&other, &envelope).expect_err("错误密钥必须失败");
    let text = err.to_string();
    assert!(!text.contains(&hex(&envelope.ciphertext()[..16])));
    assert!(!text.contains(&hex(&sample_plaintext().session_key)));
    assert!(!text.contains("42"), "错误文本不得含密钥字节");
}

// === 握手状态机 ===

/// 造一个 `Hello` 帧。
fn hello_frame(version: u32, nonce: [u8; 8]) -> Frame {
    let payload = format!("{{\"version\":{version},\"nonce\":\"{}\"}}", hex(&nonce));
    Frame::new(FRAME_TYPE_HELLO, payload.into_bytes()).expect("Hello 帧")
}

/// 造一个 `KeyEnvelope` 帧。
fn envelope_frame(k: &[u8; K_MACHINE_LEN], plain: &EnvelopePlaintext) -> Frame {
    let envelope = seal_envelope(k, plain).expect("封");
    Frame::new(FRAME_TYPE_KEY_ENVELOPE, envelope.encode()).expect("信封帧")
}

#[test]
fn hello_payload_parses() {
    let hello = parse_hello(br#"{"version":1,"nonce":"0102030405060708"}"#).expect("解析");
    assert_eq!(hello.version, 1);
    assert_eq!(hello.nonce, [1, 2, 3, 4, 5, 6, 7, 8]);
}

#[test]
fn hello_nonce_accepts_both_json_encodings() {
    // 线协议没写死 nonce 的 JSON 形式 ⇒ 两种写法都要能解析，且字节含义一致。
    let as_hex = parse_hello(br#"{"version":1,"nonce":"0102030405060708"}"#).expect("十六进制");
    let as_array = parse_hello(br#"{"version":1,"nonce":[1,2,3,4,5,6,7,8]}"#).expect("数组");
    assert_eq!(as_hex.nonce, as_array.nonce);
    assert_eq!(as_array.nonce, [1, 2, 3, 4, 5, 6, 7, 8]);
    // 大写十六进制同样接受。
    let upper = parse_hello(br#"{"version":1,"nonce":"01020304050607AB"}"#).expect("大写");
    assert_eq!(upper.nonce[7], 0xab);
}

#[test]
fn malformed_hello_payloads_are_rejected() {
    for payload in [
        &b"not json"[..],
        br#"{"version":1}"#,
        br#"{"nonce":"0102030405060708"}"#,
        br#"{"version":"x","nonce":"0102030405060708"}"#,
        br#"{"version":1,"nonce":"zz02030405060708"}"#,
        br#"{"version":1,"nonce":"0102"}"#,
    ] {
        assert_eq!(
            parse_hello(payload).expect_err("非法 Hello 载荷必须失败"),
            HandshakeError::MalformedHello
        );
    }
}

#[test]
fn handshake_reaches_ready_on_the_happy_path() {
    let k = k_machine("216feaf8875b0e55d32682bef617480d57c842fc3db57018f0715e17cc199033");
    let mut hs = Handshake::new(&k);
    assert_eq!(hs.state(), HandshakeState::WaitHello);

    let outcome = hs.on_frame(&hello_frame(PROTOCOL_VERSION, [1, 2, 3, 4, 5, 6, 7, 8]));
    assert_eq!(
        outcome,
        HandshakeOutcome::Transitioned(HandshakeState::Handshake)
    );
    assert_eq!(hs.state(), HandshakeState::Handshake);

    let plain = sample_plaintext();
    let outcome = hs.on_frame(&envelope_frame(&k, &plain));
    match outcome {
        HandshakeOutcome::Ready(key) => assert_eq!(key.as_slice(), &plain.session_key),
        other => panic!("应当握手成功，实际 {other:?}"),
    }
    assert_eq!(hs.state(), HandshakeState::Ready);
    assert!(hs.is_ready());
}

#[test]
fn version_mismatch_fails_the_handshake() {
    let k = k_machine("216feaf8875b0e55d32682bef617480d57c842fc3db57018f0715e17cc199033");
    let mut hs = Handshake::new(&k);
    let outcome = hs.on_frame(&hello_frame(PROTOCOL_VERSION + 1, [0; 8]));
    assert_eq!(
        outcome,
        HandshakeOutcome::Failed(HandshakeError::VersionMismatch)
    );
    assert_eq!(hs.state(), HandshakeState::Failed);
    assert!(!hs.is_ready());
}

#[test]
fn a_failed_handshake_never_downgrades_to_ready() {
    let k = k_machine("216feaf8875b0e55d32682bef617480d57c842fc3db57018f0715e17cc199033");
    let mut hs = Handshake::new(&k);
    assert!(hs.on_frame(&hello_frame(999, [0; 8])).is_failed());
    // 失败后再喂一个合法信封：仍必须是失败，**绝不**变成 Ready。
    let outcome = hs.on_frame(&envelope_frame(&k, &sample_plaintext()));
    assert!(outcome.is_failed());
    assert_eq!(hs.state(), HandshakeState::Failed);
}

#[test]
fn decryption_failure_fails_the_handshake() {
    let k = k_machine("216feaf8875b0e55d32682bef617480d57c842fc3db57018f0715e17cc199033");
    let other = k_machine("0000000000000000000000000000000000000000000000000000000000000000");
    let mut hs = Handshake::new(&k);
    hs.on_frame(&hello_frame(PROTOCOL_VERSION, [1, 2, 3, 4, 5, 6, 7, 8]));
    // 用另一把机器码密钥封的信封 ⇒ 解不开。
    let outcome = hs.on_frame(&envelope_frame(&other, &sample_plaintext()));
    assert_eq!(
        outcome,
        HandshakeOutcome::Failed(HandshakeError::EnvelopeRejected)
    );
    assert_eq!(hs.state(), HandshakeState::Failed);
}

#[test]
fn a_tampered_envelope_fails_the_handshake() {
    let k = k_machine("216feaf8875b0e55d32682bef617480d57c842fc3db57018f0715e17cc199033");
    let mut hs = Handshake::new(&k);
    hs.on_frame(&hello_frame(PROTOCOL_VERSION, [1, 2, 3, 4, 5, 6, 7, 8]));

    let mut bytes = seal_envelope(&k, &sample_plaintext()).expect("封").encode();
    let id_len = "container-001".len();
    let cipher_start = 4 + id_len + 4 + GCM_NONCE_LEN;
    bytes[cipher_start] ^= 0x01;
    let frame = Frame::new(FRAME_TYPE_KEY_ENVELOPE, bytes).expect("帧");
    let outcome = hs.on_frame(&frame);
    assert_eq!(
        outcome,
        HandshakeOutcome::Failed(HandshakeError::EnvelopeRejected)
    );
    assert_eq!(hs.state(), HandshakeState::Failed);
}

#[test]
fn out_of_order_frames_fail_the_handshake() {
    let k = k_machine("216feaf8875b0e55d32682bef617480d57c842fc3db57018f0715e17cc199033");
    let mut hs = Handshake::new(&k);
    // 等 Hello 时来了信封 ⇒ 失败。
    let outcome = hs.on_frame(&envelope_frame(&k, &sample_plaintext()));
    assert_eq!(
        outcome,
        HandshakeOutcome::Failed(HandshakeError::UnexpectedFrame)
    );
}

#[test]
fn handshake_failure_messages_never_contain_key_material() {
    let k = k_machine("216feaf8875b0e55d32682bef617480d57c842fc3db57018f0715e17cc199033");
    let other = k_machine("0000000000000000000000000000000000000000000000000000000000000000");
    let plain = sample_plaintext();

    let mut hs = Handshake::new(&k);
    hs.on_frame(&hello_frame(PROTOCOL_VERSION, [1, 2, 3, 4, 5, 6, 7, 8]));
    let outcome = hs.on_frame(&envelope_frame(&other, &plain));
    let err = outcome.error().expect("应当失败");
    let text = err.to_string();
    assert!(!text.contains(&hex(&plain.session_key)), "错误文本含密钥");
    assert!(
        !text.contains(&hex(k.as_slice())),
        "错误文本含机器码派生密钥"
    );
    assert!(!text.contains("42"), "错误文本含密钥字节");
}

#[test]
fn a_ready_handshake_rejects_further_frames() {
    let k = k_machine("216feaf8875b0e55d32682bef617480d57c842fc3db57018f0715e17cc199033");
    let mut hs = Handshake::new(&k);
    hs.on_frame(&hello_frame(PROTOCOL_VERSION, [1, 2, 3, 4, 5, 6, 7, 8]));
    hs.on_frame(&envelope_frame(&k, &sample_plaintext()));
    assert_eq!(hs.state(), HandshakeState::Ready);
    let outcome = hs.on_frame(&hello_frame(PROTOCOL_VERSION, [1, 2, 3, 4, 5, 6, 7, 8]));
    assert!(outcome.is_failed());
}

// === 工具 ===

/// 字节数组的十六进制（小写）。
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
