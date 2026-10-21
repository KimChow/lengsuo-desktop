//! P2 卡的测试：端点映射、握手、增量解帧、畸形帧、事件 sink、断链、命令行。
//!
//! ⛔ 固定测试向量全部**硬编码**（线协议 §2 / §3.1）——不在测试里算期望值。
//! ⛔ 端到端测试有**超时**（CI 的 `timeout 120` 兜底，但本地也要不挂）。

use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::*;
use crate::kernel_pipe::handshake::{envelope_aad, Handshake, PROTOCOL_VERSION};
use crate::kernel_pipe::keys::{self, SessionKeySource};
use crate::kernel_pipe::launch;
use crate::kernel_pipe::tests::{
    fixed_k_machine, hex_bytes32, CONTAINER_ID, ENVELOPE_AAD, EXPIRES_UNIX, GCM_NONCE, HELLO_NONCE,
    K_MACHINE_HEX, SESSION_KEY_HEX,
};
use crate::kernel_pipe::{Frame, FrameType};

/// 造一个 `Hello` 帧（JSON `{version, nonce}`，nonce 用小写十六进制串）。
fn hello_frame(version: u32, nonce: &[u8; 8]) -> Frame {
    let nonce_hex: String = nonce.iter().map(|b| format!("{b:02x}")).collect();
    let payload = format!(r#"{{"version":{version},"nonce":"{nonce_hex}"}}"#);
    Frame::new(FrameType::Hello, payload.into_bytes())
}

/// 事件载荷（线协议 §4：JSON，带 `action`）。
fn event_payload(action: &str) -> Vec<u8> {
    format!(r#"{{"from":"kernel","action":"{action}","url":"https://example.invalid/"}}"#)
        .into_bytes()
}

// ---------------------------------------------------------------------------
// 端点映射
// ---------------------------------------------------------------------------

#[test]
fn two_frames_in_one_buffer_decode_into_two_frames() {
    // ⛔ 一次 read 未必拿到整帧 ⇒ 缓冲区里可能有好几帧。
    let a = hello_frame(PROTOCOL_VERSION, &HELLO_NONCE)
        .encode()
        .expect("应可编码");
    let b = Frame::new(FrameType::Event, event_payload("Heartbeat"))
        .encode()
        .expect("应可编码");
    let mut buf = a.clone();
    buf.extend_from_slice(&b);

    let mut decoder = FrameDecoder::new();
    decoder.push(&buf);

    let first = decoder.next_frame().expect("应可解");
    assert!(matches!(first, DecodeOutcome::Frame(_)));
    let second = decoder.next_frame().expect("应可解");
    assert!(matches!(second, DecodeOutcome::Frame(_)));
    // 第三帧 ⇒ 缓冲空了 ⇒ 「还没读够」而不是错误。
    assert_eq!(
        decoder.next_frame().expect("应可解"),
        DecodeOutcome::Incomplete
    );
    assert_eq!(decoder.buffered(), 0);
}

#[test]
fn a_frame_split_in_half_yields_incomplete_then_the_whole_frame() {
    // ⛔ 一次 read 未必拿到整帧 ⇒ 可能有半帧。
    let bytes = hello_frame(PROTOCOL_VERSION, &HELLO_NONCE)
        .encode()
        .expect("应可编码");
    let (first_half, second_half) = bytes.split_at(bytes.len() / 2);

    let mut decoder = FrameDecoder::new();
    decoder.push(first_half);
    // ⛔ 半帧是 `Incomplete`（**不是错误**）。
    assert_eq!(
        decoder.next_frame().expect("半帧不该是错误"),
        DecodeOutcome::Incomplete
    );
    assert!(decoder.buffered() > 0);

    decoder.push(second_half);
    match decoder.next_frame().expect("应可解") {
        DecodeOutcome::Frame(frame) => {
            assert_eq!(frame.frame_type, FrameType::Hello);
        }
        DecodeOutcome::Incomplete => panic!("补齐后必须解出整帧"),
    }
    assert_eq!(decoder.buffered(), 0);
}

#[test]
fn a_byte_at_a_time_still_decodes_to_the_whole_frame() {
    // 逐字节喂 ⇒ 每一步都是 Incomplete，最后一步解出整帧。
    let bytes = Frame::new(FrameType::Event, event_payload("PageEvent"))
        .encode()
        .expect("应可编码");
    let mut decoder = FrameDecoder::new();
    for (idx, byte) in bytes.iter().enumerate() {
        decoder.push(&[*byte]);
        let outcome = decoder.next_frame().expect("不该是错误");
        if idx + 1 < bytes.len() {
            assert_eq!(
                outcome,
                DecodeOutcome::Incomplete,
                "第 {idx} 字节应还没读够"
            );
        } else {
            assert!(matches!(outcome, DecodeOutcome::Frame(_)));
        }
    }
}

// ---------------------------------------------------------------------------
// 畸形帧
// ---------------------------------------------------------------------------

#[test]
fn malformed_frames_fail_instead_of_being_tolerated() {
    // 线协议 §2 的非法帧。⚠️ 「头被截断」与「载荷不足」在**流中间**只是
    // 「还没读够」（⇒ Incomplete），流**结束**时才是错误（另一个测试守住）。
    let cases: [(&str, Vec<u8>, FrameError); 3] = [
        (
            "len 超上限",
            vec![0x00, 0x80, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00],
            FrameError::PayloadTooLarge {
                declared: 8 * 1024 * 1024 + 1,
            },
        ),
        (
            "reserved 非 0",
            vec![0x00, 0x00, 0x00, 0x01, 0x03, 0x00, 0x00, 0x01, 0x5A],
            FrameError::ReservedNotZero,
        ),
        (
            "未知 type",
            vec![0x00, 0x00, 0x00, 0x01, 0x07, 0x00, 0x00, 0x00, 0x5A],
            FrameError::UnknownType { code: 0x07 },
        ),
    ];
    for (what, bytes, expected) in cases {
        let mut decoder = FrameDecoder::new();
        decoder.push(&bytes);
        let err = decoder
            .next_frame()
            .expect_err(&format!("{what} 必须失败，不得当成半帧"));
        assert_eq!(err, expected, "{what} 的错误类型不对");
    }
}

#[test]
fn a_truncated_frame_is_incomplete_while_the_stream_is_alive() {
    // ⛔ 流中间「头不全是 7 字节」/「声明 3、实给 1」= 还没读够；
    // 流**结束**时才是错误。
    for bytes in [
        &[0x00, 0x00, 0x00, 0x01, 0x03, 0x00, 0x00][..],
        &[0x00, 0x00, 0x00, 0x03, 0x03, 0x00, 0x00, 0x00, 0x5A][..],
    ] {
        let mut decoder = FrameDecoder::new();
        decoder.push(bytes);
        assert_eq!(
            decoder.next_frame().expect("不该是错误"),
            DecodeOutcome::Incomplete
        );
        // 流结束 ⇒ 残留半帧才是错误。
        assert!(decoder.finish().is_err(), "流结束时残留半帧必须报错");
    }
}

#[test]
fn a_malformed_frame_error_never_leaks_the_payload() {
    // ⛔ 解码失败不得返回部分明文，错误文本也不得含载荷内容。
    let secret = b"super-secret-payload";
    let mut bytes = vec![0x00, 0x00, 0x00, 0x01, 0x07, 0x00, 0x00, 0x00];
    bytes.extend_from_slice(secret);
    let mut decoder = FrameDecoder::new();
    decoder.push(&bytes);
    let err = decoder.next_frame().expect_err("未知 type 必须失败");
    let text = err.to_string();
    assert!(!text.contains("super-secret"), "错误文本不得含载荷：{text}");
}

// ---------------------------------------------------------------------------
// 握手（端到端：内核侧造帧 ⇒ 服务端回信封）
// ---------------------------------------------------------------------------

/// 造一个「内核侧」的对端：连上抽象命名空间端点，造帧、收发。
#[cfg(target_os = "linux")]
struct KernelPeer {
    stream: std::os::unix::net::UnixStream,
}

#[cfg(target_os = "linux")]
impl KernelPeer {
    fn connect(server: &KernelPipeServer) -> Self {
        use std::os::linux::net::SocketAddrExt;
        let addr = std::os::unix::net::SocketAddr::from_abstract_name(
            launch::abstract_name(server.name()).as_bytes(),
        )
        .expect("抽象名应合法");
        let stream = std::os::unix::net::UnixStream::connect_addr(&addr).expect("应能连上");
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .expect("应能设超时");
        stream
            .set_write_timeout(Some(Duration::from_secs(10)))
            .expect("应能设超时");
        Self { stream }
    }

    fn send(&mut self, frame: &Frame) {
        use std::io::Write;
        let bytes = frame.encode().expect("应可编码");
        self.stream.write_all(&bytes).expect("应能写");
    }

    fn recv_frame(&mut self) -> Frame {
        use std::io::Read;
        let mut header = [0u8; 8];
        self.stream.read_exact(&mut header).expect("应能读头");
        let len = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as usize;
        let mut payload = vec![0u8; len];
        self.stream.read_exact(&mut payload).expect("应能读载荷");
        assert_eq!(header[4], FrameType::KeyEnvelope.code());
        Frame::new(FrameType::KeyEnvelope, payload)
    }

    /// 收任意一帧（**不**断言类型）：握手后连着收两帧时要用。
    fn recv_any_frame(&mut self) -> (u8, Vec<u8>, [u8; 8]) {
        use std::io::Read;
        let mut header = [0u8; 8];
        self.stream.read_exact(&mut header).expect("应能读头");
        let len = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as usize;
        let mut payload = vec![0u8; len];
        self.stream.read_exact(&mut payload).expect("应能读载荷");
        (header[4], payload, header)
    }
}

/// 造一份配置（固定 `k_machine` + 固定 GCM nonce ⇒ 字节可比对）。
#[cfg(target_os = "linux")]
fn fixed_config() -> PipeServerConfig {
    let mut config = PipeServerConfig::with_k_machine(
        SessionKeySource::Hkdf,
        CONTAINER_ID,
        EXPIRES_UNIX,
        fixed_k_machine(),
    );
    config.gcm_nonce = Some(GCM_NONCE);
    config
}

/// 端到端：内核发 `Hello` ⇒ 服务端回的 `KeyEnvelope` **逐字节**等于
/// P1 的 `build_key_envelope_frame` 算出的字节。
#[cfg(target_os = "linux")]
#[test]
fn the_handshake_replies_with_the_byte_for_byte_expected_envelope() {
    let name = format!("t-handshake-{}", std::process::id());
    let server = KernelPipeServer::bind(&name, fixed_config()).expect("应能绑定");
    let mut peer = KernelPeer::connect(&server);

    let mut conn = server
        .accept(Some(Duration::from_secs(5)))
        .expect("应能接受");

    // 内核侧：发 Hello。
    peer.send(&hello_frame(PROTOCOL_VERSION, &HELLO_NONCE));

    // 服务端侧：握手。
    conn.perform_handshake().expect("握手应成功");
    assert!(conn.is_ready());

    // 内核侧：收 KeyEnvelope。
    let got = peer.recv_frame();

    // 期望字节：P1 的握手状态机自己造一帧。
    let mut expected_hs = Handshake::new(fixed_k_machine());
    expected_hs
        .on_frame(&hello_frame(PROTOCOL_VERSION, &HELLO_NONCE))
        .expect("Hello 应被接受");
    let expected = expected_hs
        .build_key_envelope_frame(
            &hex_bytes32(SESSION_KEY_HEX),
            CONTAINER_ID,
            EXPIRES_UNIX,
            GCM_NONCE,
        )
        .expect("应能造信封")
        .encode()
        .expect("应可编码");

    assert_eq!(
        got.payload,
        expected[8..].to_vec(),
        "回的 KeyEnvelope 载荷必须与 P1 算出的逐字节相同"
    );
    // 帧头也逐字节相同（len 大端、type = 0x02、reserved = 0）。
    assert_eq!(
        &expected[..8],
        &[0x00, 0x00, 0x00, 0x55, 0x02, 0x00, 0x00, 0x00]
    );
    // AAD 与线协议钉死的那 16 字节一致（载荷 85 字节 ⇒ len = 0x55）。
    assert_eq!(envelope_aad(85, &HELLO_NONCE), ENVELOPE_AAD);
    // 解出来的会话密钥就是注入的那一个（候选 B = HKDF，与固定向量相同）。
    let aad = envelope_aad(got.payload.len(), &HELLO_NONCE);
    let envelope = keys::KeyEnvelope::decode(&got.payload).expect("应可解码");
    let plain = keys::open_envelope(&fixed_k_machine(), &aad, &envelope).expect("应能解封");
    assert_eq!(keys::to_hex(&plain.session_key), SESSION_KEY_HEX);
    assert_eq!(plain.container_id, CONTAINER_ID.to_vec());
    assert_eq!(plain.expires_unix, EXPIRES_UNIX);
}

#[cfg(target_os = "linux")]
#[test]
fn the_handshake_is_followed_by_the_config_bundle_frame() {
    // ⭐ 必做项 0：握手后**立刻**发一帧 `ConfigBundle`（0x03）。
    // ⛔ 不发这一帧，内核会卡在等 `ConfigBundle`：连接不报错，
    // 但内核不进入事件阶段 ⇒ 整条通道卡住。
    let name = format!("t-configbundle-{}", std::process::id());
    let server = KernelPipeServer::bind(&name, fixed_config()).expect("应能绑定");
    let mut peer = KernelPeer::connect(&server);
    let mut conn = server
        .accept(Some(Duration::from_secs(5)))
        .expect("应能接受");

    peer.send(&hello_frame(PROTOCOL_VERSION, &HELLO_NONCE));
    conn.perform_handshake().expect("握手应成功");
    assert!(conn.is_ready());

    // 第一帧：KeyEnvelope（0x02）。
    let (kind, _payload, _header) = peer.recv_any_frame();
    assert_eq!(kind, 0x02, "第一帧必须是密钥信封");

    // 第二帧：ConfigBundle（0x03）—— ⛔ 这一帧不许缺。
    let (kind, payload, header) = peer.recv_any_frame();
    assert_eq!(kind, 0x03, "第二帧必须是 ConfigBundle（0x03）");

    // 帧头 8 字节：reserved 三字节全 0、len 是**大端**的载荷字节数。
    assert_eq!(&header[5..8], &[0x00, 0x00, 0x00], "reserved 必须全 0");
    let declared = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
    assert_eq!(
        declared as usize,
        payload.len(),
        "len 必须是载荷字节数（大端）"
    );

    // 载荷是**合法 JSON**，且与跨仓固定向量逐字节相同。
    let expected = upload::config_bundle_payload(&upload::ProfileSyncRules::default());
    assert_eq!(
        payload,
        expected.as_bytes(),
        "ConfigBundle 载荷必须与固定向量逐字节相同"
    );
    let parsed: serde_json::Value = serde_json::from_slice(&payload).expect("应是合法 JSON");
    assert!(
        parsed.get("profile_sync").is_some(),
        "必须有 profile_sync 段"
    );

    // ⛔ 载荷里不许有密钥、会话密钥、`data_url`（拍板 3A）。
    let text = String::from_utf8_lossy(&payload);
    assert!(!text.contains("data_url"), "不许有 data_url：{text}");
    assert!(!text.contains("session_key"), "不许有会话密钥：{text}");
    assert!(!text.contains("k_machine"), "不许有机器密钥：{text}");
    assert!(
        !text.contains(SESSION_KEY_HEX),
        "不许有会话密钥字节：{text}"
    );
    assert!(!text.contains(K_MACHINE_HEX), "不许有机器密钥字节：{text}");
}

#[cfg(target_os = "linux")]
#[test]
fn a_version_mismatch_disconnects_and_never_leaks_key_material() {
    let name = format!("t-version-{}", std::process::id());
    let server = KernelPipeServer::bind(&name, fixed_config()).expect("应能绑定");
    let mut peer = KernelPeer::connect(&server);
    let mut conn = server
        .accept(Some(Duration::from_secs(5)))
        .expect("应能接受");

    // 内核侧：发一个版本不对的 Hello。
    peer.send(&hello_frame(PROTOCOL_VERSION + 1, &HELLO_NONCE));

    let err = conn.perform_handshake().expect_err("版本不匹配必须失败");
    assert!(matches!(err, PipeError::Handshake(_)), "实际是 {err:?}");
    // ⛔ 断链：连接被标记为断，且握手没完成。
    assert!(conn.is_broken(), "版本不匹配必须断链");
    assert!(!conn.is_ready(), "版本不匹配绝不进 Ready");

    // ⛔ 错误文本不得含密钥 / k_machine / 信封字节。
    let text = err.to_string();
    assert!(
        !text.contains(SESSION_KEY_HEX),
        "错误文本含会话密钥：{text}"
    );
    assert!(
        !text.contains(K_MACHINE_HEX),
        "错误文本含 k_machine：{text}"
    );
    let session_bytes = hex_bytes32(SESSION_KEY_HEX);
    for byte in session_bytes {
        let needle = format!("{byte:02x}");
        assert!(
            !text.contains(&needle),
            "错误文本含密钥字节 {needle}：{text}"
        );
    }

    // ⛔ fail-closed：绝不回一个「无密钥」的降级响应。
    // （对端此时读到的应是 EOF，而不是一帧 KeyEnvelope。）
    use std::io::Read;
    let mut buf = [0u8; 1];
    let read = peer.stream.read(&mut buf).expect("读应成功");
    assert_eq!(read, 0, "版本不匹配后不得回任何帧（应断链）");
}

#[cfg(target_os = "linux")]
#[test]
fn a_malformed_hello_disconnects_the_peer() {
    let name = format!("t-badhello-{}", std::process::id());
    let server = KernelPipeServer::bind(&name, fixed_config()).expect("应能绑定");
    let mut peer = KernelPeer::connect(&server);
    let mut conn = server
        .accept(Some(Duration::from_secs(5)))
        .expect("应能接受");

    // ⛔ 载荷不是 JSON（但**帧本身**是合法的）⇒ 握手层失败。
    peer.send(&Frame::new(FrameType::Hello, b"not-json-at-all".to_vec()));

    let err = conn.perform_handshake().expect_err("非法 Hello 必须失败");
    assert!(matches!(err, PipeError::Handshake(_)), "实际是 {err:?}");
    assert!(conn.is_broken(), "非法 Hello 必须断链");
    assert!(!conn.is_ready());
}

// ---------------------------------------------------------------------------
// 事件 sink
// ---------------------------------------------------------------------------

/// 收集日志行的 sink（用来断言「日志里没有载荷内容」）。
#[derive(Clone, Default)]
struct LogCollector {
    lines: Arc<Mutex<Vec<String>>>,
}

impl LogCollector {
    fn sink(&self) -> Box<dyn EventSink> {
        let lines = Arc::clone(&self.lines);
        Box::new(CountingEventSink::with_log(move |line| {
            lines.lock().expect("锁").push(line.to_string());
        }))
    }

    fn text(&self) -> String {
        self.lines.lock().expect("锁").join("\n")
    }
}

#[cfg(target_os = "linux")]
#[test]
fn the_event_sink_only_counts_and_never_logs_the_payload() {
    let name = format!("t-event-{}", std::process::id());
    let server = KernelPipeServer::bind(&name, fixed_config()).expect("应能绑定");
    let mut peer = KernelPeer::connect(&server);
    let mut conn = server
        .accept(Some(Duration::from_secs(5)))
        .expect("应能接受");

    let collector = LogCollector::default();
    conn.set_event_sink(collector.sink());

    peer.send(&hello_frame(PROTOCOL_VERSION, &HELLO_NONCE));
    conn.perform_handshake().expect("握手应成功");

    // 短读超时 ⇒ `pump` 读到没有更多数据就收工（测试不等生产默认值）。
    conn.set_read_timeout(Some(Duration::from_millis(300)))
        .expect("应能设超时");

    // 载荷里带一个**特征串**——日志里不许出现它。
    let secret = "SECRET-CANVAS-PAYLOAD-9f3a";
    let payload = format!(
        r#"{{"from":"kernel","action":"CanvasSample","profile":"p1","width":1,"length":2,"url":"https://example.invalid/","hash":"h","data_url":"{secret}"}}"#
    );
    peer.send(&Frame::new(FrameType::Event, payload.into_bytes()));

    conn.pump().expect("收发应成功");

    let stats = conn.stats();
    assert_eq!(stats.events_received, 1, "应收到一条事件");
    assert_eq!(stats.events_dropped, 0);

    // ⛔ 日志里只有 action 名与字节数，**没有载荷内容**。
    let log = collector.text();
    assert!(log.contains("CanvasSample"), "日志应记 action 名：{log}");
    assert!(!log.contains(secret), "日志不得含载荷内容：{log}");
    assert!(!log.contains("data_url"), "日志不得含载荷字段：{log}");
}

#[cfg(target_os = "linux")]
#[test]
fn an_event_with_a_missing_action_is_dropped_and_counted() {
    let name = format!("t-badevent-{}", std::process::id());
    let server = KernelPipeServer::bind(&name, fixed_config()).expect("应能绑定");
    let mut peer = KernelPeer::connect(&server);
    let mut conn = server
        .accept(Some(Duration::from_secs(5)))
        .expect("应能接受");

    let collector = LogCollector::default();
    conn.set_event_sink(collector.sink());

    peer.send(&hello_frame(PROTOCOL_VERSION, &HELLO_NONCE));
    conn.perform_handshake().expect("握手应成功");

    // 短读超时 ⇒ `pump` 读到没有更多数据就收工。
    conn.set_read_timeout(Some(Duration::from_millis(300)))
        .expect("应能设超时");

    // ⛔ 必填字段缺失 ⇒ 丢弃 + 计数，**不发送**。
    peer.send(&Frame::new(
        FrameType::Event,
        b"{\"from\":\"kernel\"}".to_vec(),
    ));
    conn.pump().expect("收发应成功");

    let stats = conn.stats();
    assert_eq!(stats.events_received, 0, "非法事件不该被收下");
    assert_eq!(stats.events_dropped, 1, "非法事件必须被计数");
    assert!(collector.text().is_empty(), "非法事件不该进日志");
}

// ---------------------------------------------------------------------------
// 断链后不阻塞
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
#[test]
fn emitting_after_the_peer_closed_returns_at_once_and_drops_the_frame() {
    let name = format!("t-broken-{}", std::process::id());
    let server = KernelPipeServer::bind(&name, fixed_config()).expect("应能绑定");
    let mut peer = KernelPeer::connect(&server);
    let mut conn = server
        .accept(Some(Duration::from_secs(5)))
        .expect("应能接受");

    peer.send(&hello_frame(PROTOCOL_VERSION, &HELLO_NONCE));
    conn.perform_handshake().expect("握手应成功");

    // 对端把收到的信封读干净再关（不留未读数据 ⇒ 不会触发 RST）。
    let _ = peer.recv_frame();
    drop(peer);

    // ⛔ `emit` 必须**立刻**返回（不等待），且帧被丢弃 + 计数。
    let started = std::time::Instant::now();
    let ok = conn.emit(&Frame::new(FrameType::EventAck, b"{\"seq\":1}".to_vec()));
    let elapsed = started.elapsed();
    assert!(!ok, "对端已关 ⇒ 发不出去");
    assert!(
        elapsed < Duration::from_secs(2),
        "emit 不得等待（实际等了 {elapsed:?}）"
    );
    assert_eq!(conn.stats().frames_dropped, 1, "发不出去的帧必须计数");
    assert!(conn.is_broken(), "发失败后连接应标记为断");
}

#[cfg(target_os = "linux")]
#[test]
fn reading_from_a_closed_peer_reports_the_close_without_hanging() {
    let name = format!("t-closed-{}", std::process::id());
    let server = KernelPipeServer::bind(&name, fixed_config()).expect("应能绑定");
    let mut peer = KernelPeer::connect(&server);
    let mut conn = server
        .accept(Some(Duration::from_secs(5)))
        .expect("应能接受");

    peer.send(&hello_frame(PROTOCOL_VERSION, &HELLO_NONCE));
    conn.perform_handshake().expect("握手应成功");
    // 对端把收到的信封读干净再关（不留未读数据 ⇒ 不会触发 RST）。
    let _ = peer.recv_frame();
    drop(peer);

    let started = std::time::Instant::now();
    // 读到 EOF ⇒ `Ok(None)`（不是错误），并把连接标记为断。
    let frame = conn.read_frame().expect("读 EOF 不该是错误");
    assert!(frame.is_none(), "对端关了 ⇒ 没有帧");
    assert!(conn.is_closed(), "应识别出对端已关闭");
    assert!(conn.is_broken());
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "读 EOF 不得挂住"
    );
}

// ---------------------------------------------------------------------------
// 命令行
// ---------------------------------------------------------------------------

#[test]
fn the_session_key_source_is_injectable_not_hard_coded() {
    // ⛔ 待主理人拍板 ⇒ 本卡**不替主理人选**。
    let random = keys::make_session_key(SessionKeySource::Random, &fixed_k_machine(), &HELLO_NONCE)
        .expect("应可生成");
    let hkdf = keys::make_session_key(SessionKeySource::Hkdf, &fixed_k_machine(), &HELLO_NONCE)
        .expect("应可派生");
    assert_ne!(random, hkdf, "两种候选必须给出不同的密钥");
    assert_eq!(
        keys::to_hex(&hkdf),
        SESSION_KEY_HEX,
        "HKDF 必须对上固定向量"
    );
    // 默认值只是「当前先用哪个」的落点 ⇒ 改一行即可换。
    assert_eq!(DEFAULT_SESSION_KEY_SOURCE, SessionKeySource::Random);
}
