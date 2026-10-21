//! 内核管道的 **IO 层**：管道服务端（监听 + 接受连接 + 收发循环）。
//!
//! # 线协议
//!
//! 端点映射与握手流程都逐字节对齐跨仓契约 `kernel-wire-contract.md` §1：
//!
//! | 平台 | 端点 |
//! |---|---|
//! | Windows | 命名管道 `\\.\pipe\lengsuo-kernel-<name>` |
//! | Linux / POSIX | **抽象命名空间** unix 域套接字 `\0lengsuo-kernel-<name>` |
//!
//! ⛔ 前缀 `lengsuo-kernel-` 两侧必须逐字节一致；⛔ 不许用文件系统路径
//! （抽象命名空间不留 socket 文件 ⇒ 不用清理、不受目录权限影响）。
//!
//! # 分层
//!
//! - 纯逻辑（帧编解码 / 密钥 / 握手状态机）在 [`super`]（P1 卡）——本模块**只接线**。
//! - 本模块做：增量解帧、收发循环、事件路由。
//! - 端点映射 / 管道名 / 传输层 / 服务端配置在 [`crate::kernel_pipe::launch`]。
//!
//! # ⛔ 安全边界
//!
//! - 会话密钥只在内存里；离开作用域即清零。
//! - 错误文本**不含密钥或信封内容**。
//! - 事件日志**只记 action 名与字节数**，⛔ 不记载荷内容。
//! - 管道断 ⇒ 事件丢弃 + 计数，**绝不阻塞**调用方。
//!
//! # 平台
//!
//! ⛔ 本卡只实现 **Linux** 的传输层（抽象命名空间）。
//! Windows 的**端点映射**（字符串）两侧已经钉死并有测试，
//! 但**命名管道服务端**需要 `CreateNamedPipeW`——本仓禁 `unsafe`、
//! 且不许加依赖 ⇒ 传输层留到下一张卡（见报告 §未完成）。

use std::io;
use std::time::{Duration, Instant};

use zeroize::Zeroize;

use crate::kernel_pipe::handshake::{Handshake, HandshakeError, HandshakeState};
use crate::kernel_pipe::keys::{self, SessionKeySource, GCM_NONCE_LEN};
use crate::kernel_pipe::launch::{
    self, imp::Listener, imp::Transport, Endpoint, PipeError, PipeServerConfig,
};
use crate::kernel_pipe::{decode_frame, Frame, FrameError, FrameType};

#[cfg(test)]
#[path = "io_tests.rs"]
mod io_tests;

/// 本卡选的**默认**会话密钥生成方式。
///
/// ⛔ **待主理人拍板**（线协议 §1 列出候选 A / B 互相矛盾）。
/// 本卡**不替主理人选**：这个常量只是「当前先用哪个」的落点，
/// 改成 [`SessionKeySource::Hkdf`] 只需改这一行。
///
/// ⭐ **不影响线上字节**：信封总是携带 `session_key`，接收方用信封里解出来的值。
pub const DEFAULT_SESSION_KEY_SOURCE: SessionKeySource = SessionKeySource::Random;

/// 默认的读超时。
const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(30);
/// 默认的写超时——保证 `emit` **不会无限期**卡住。
const DEFAULT_WRITE_TIMEOUT: Duration = Duration::from_secs(5);

// ---------------------------------------------------------------------------
// 增量解帧
// ---------------------------------------------------------------------------

/// 一次解帧的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeOutcome {
    /// 解出一帧。
    Frame(Frame),
    /// 缓冲区里的字节还不够凑一帧——**不是错误**，等更多字节。
    Incomplete,
}

/// 增量解帧器：把「读到的字节」变成「一帧一帧的帧」。
///
/// ⛔ **不假设「一次 read = 一帧」**：管道是字节流，一次 read 可能带来
/// 半帧，也可能带来好几帧 ⇒ 缓冲区里可能同时有「半帧」和「好几帧」。
#[derive(Debug, Default)]
pub struct FrameDecoder {
    buf: Vec<u8>,
}

impl FrameDecoder {
    /// 空解帧器。
    #[must_use]
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    /// 把新读到的字节塞进缓冲区。
    pub fn push(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// 解出下一帧。
    ///
    /// - 字节不够 ⇒ [`DecodeOutcome::Incomplete`]（**不是错误**）；
    /// - 畸形帧 ⇒ `Err`（协议坏了，必须断链）。
    pub fn next_frame(&mut self) -> Result<DecodeOutcome, FrameError> {
        match decode_frame(&self.buf) {
            Ok((frame, used)) => {
                self.buf.drain(..used);
                Ok(DecodeOutcome::Frame(frame))
            }
            // ⛔ 「头/载荷不完整」在流里就是「还没读够」⇒ 等更多字节，
            // 不当错误。真正的畸形（len 超限 / reserved 非 0 / 未知 type）才是错误。
            Err(FrameError::TruncatedHeader { .. } | FrameError::TruncatedPayload { .. }) => {
                Ok(DecodeOutcome::Incomplete)
            }
            Err(err) => Err(err),
        }
    }

    /// 缓冲区里还没解出来的字节数。
    #[must_use]
    pub fn buffered(&self) -> usize {
        self.buf.len()
    }

    /// 流结束时的收尾：残留半帧 ⇒ 报截断错误。
    ///
    /// 「头被截断」只有在**流结束**时才能判定——流中间它只是「还没读够」。
    /// ⚠️ 缓冲区**空**时是**干净的收尾**（对端正常关闭），不是截断。
    pub fn finish(&self) -> Result<(), FrameError> {
        if self.buf.is_empty() {
            return Ok(());
        }
        decode_frame(&self.buf).map(|_| ())
    }
}

// ---------------------------------------------------------------------------
// 上行事件
// ---------------------------------------------------------------------------

/// 一条上行事件的**元信息**。
///
/// ⛔ **不含载荷内容**——只有 action 名与字节数（线协议 §4：日志只记这两样）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventMeta {
    /// action 名（如 `CanvasSample` / `Heartbeat`）。
    pub action: String,
    /// 载荷字节数。
    pub payload_len: usize,
}

/// 上行事件的去处。
pub trait EventSink {
    /// 收到一条上行事件。
    fn on_event(&mut self, event: &EventMeta);
}

/// 默认的事件去处：**只计数 + 打日志**。
///
/// ⛔ 日志里只有 action 名与字节数——**不记载荷内容**。
pub struct CountingEventSink {
    count: u64,
    last_action: Option<String>,
    log: Box<dyn Fn(&str) + Send + Sync>,
}

impl std::fmt::Debug for CountingEventSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CountingEventSink")
            .field("count", &self.count)
            .field("last_action", &self.last_action)
            .finish_non_exhaustive()
    }
}

impl Default for CountingEventSink {
    fn default() -> Self {
        Self::new()
    }
}

impl CountingEventSink {
    /// 默认实现：日志走 stderr。
    #[must_use]
    pub fn new() -> Self {
        Self::with_log(|line| eprintln!("{line}"))
    }

    /// 自定义日志去向（测试用收集器）。
    pub fn with_log(log: impl Fn(&str) + Send + Sync + 'static) -> Self {
        Self {
            count: 0,
            last_action: None,
            log: Box::new(log),
        }
    }

    /// 收到的事件数。
    #[must_use]
    pub fn count(&self) -> u64 {
        self.count
    }

    /// 最近一次事件的 action 名。
    #[must_use]
    pub fn last_action(&self) -> Option<&str> {
        self.last_action.as_deref()
    }
}

impl EventSink for CountingEventSink {
    fn on_event(&mut self, event: &EventMeta) {
        self.count += 1;
        self.last_action = Some(event.action.clone());
        (self.log)(&format!(
            "[lengsuo-desktop] 内核事件：action={}，字节数={}",
            event.action, event.payload_len
        ));
    }
}

/// 从事件载荷里取出 action 名与字节数。
///
/// ⛔ 必填字段缺失 / 类型不符 ⇒ `None`（调用方丢弃 + 计数）。
fn parse_event_meta(payload: &[u8]) -> Option<EventMeta> {
    let value: serde_json::Value = serde_json::from_slice(payload).ok()?;
    let action = value.get("action")?.as_str()?;
    Some(EventMeta {
        action: action.to_string(),
        payload_len: payload.len(),
    })
}

// ---------------------------------------------------------------------------
// 监听端点
// ---------------------------------------------------------------------------

/// 管道服务端：绑定端点，等内核连上来。
pub struct KernelPipeServer {
    name: String,
    config: PipeServerConfig,
    listener: Listener,
}

impl std::fmt::Debug for KernelPipeServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KernelPipeServer")
            .field("name", &self.name)
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl KernelPipeServer {
    /// 绑定端点（不 accept）。
    ///
    /// ⛔ 绑定失败（名字被占用等）⇒ 报错，**不留半开的端点**
    /// （抽象命名空间的端点随监听 fd 关闭而消失 ⇒ 不用删文件）。
    pub fn bind(name: &str, config: PipeServerConfig) -> Result<Self, PipeError> {
        launch::validate_pipe_name(name)?;
        let listener = Listener::bind(&launch::abstract_name(name)).map_err(|err| {
            if err.kind() == io::ErrorKind::Unsupported {
                PipeError::UnsupportedPlatform
            } else {
                PipeError::Io(err)
            }
        })?;
        Ok(Self {
            name: name.to_string(),
            config,
            listener,
        })
    }

    /// 管道名。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 端点（与内核侧看到的**逐字节**一致）。
    #[must_use]
    pub fn endpoint(&self) -> Endpoint {
        Endpoint::for_pipe_name(&self.name)
    }

    /// 接受一个内核连接。
    ///
    /// `timeout` = 等待连接的超时（`None` = 一直等）。
    pub fn accept(&self, timeout: Option<Duration>) -> Result<Connection, PipeError> {
        let deadline = timeout.map(|t| Instant::now() + t);
        loop {
            match self.listener.accept()? {
                Some(transport) => return Ok(Connection::new(transport, self.config.clone())),
                None => {
                    if let Some(deadline) = deadline {
                        if Instant::now() >= deadline {
                            return Err(PipeError::Io(io::Error::new(
                                io::ErrorKind::TimedOut,
                                "等待内核连接超时",
                            )));
                        }
                    }
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 连接
// ---------------------------------------------------------------------------

/// 连接的计数（失败语义：丢弃 + 计数，**绝不阻塞**）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConnStats {
    /// 收到并交给 sink 的事件数。
    pub events_received: u64,
    /// 收到但被丢弃的事件数（载荷非法）。
    pub events_dropped: u64,
    /// 发不出去而被丢弃的帧数。
    pub frames_dropped: u64,
    /// 收到但不是事件的帧数。
    pub frames_ignored: u64,
}

/// 一条已建立的内核连接。
pub struct Connection {
    transport: Transport,
    decoder: FrameDecoder,
    handshake: Handshake,
    config: PipeServerConfig,
    sink: Box<dyn EventSink>,
    stats: ConnStats,
    /// 握手是否已完成。
    ready: bool,
    /// 管道是否已断（对端关闭 / 写过失败）。
    broken: bool,
    /// 对端是否已正常关闭。
    closed: bool,
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("ready", &self.ready)
            .field("broken", &self.broken)
            .field("closed", &self.closed)
            .field("stats", &self.stats)
            .finish_non_exhaustive()
    }
}

impl Connection {
    /// 新建连接（未握手）。
    fn new(transport: Transport, config: PipeServerConfig) -> Self {
        let mut this = Self {
            transport,
            decoder: FrameDecoder::new(),
            handshake: Handshake::new(config.k_machine),
            config,
            sink: Box::new(CountingEventSink::new()),
            stats: ConnStats::default(),
            ready: false,
            broken: false,
            closed: false,
        };
        // ⛔ 写超时 ⇒ `emit` 不会无限期卡住；读超时 ⇒ 收发循环能定期醒来。
        let _ = this.transport.set_read_timeout(Some(DEFAULT_READ_TIMEOUT));
        let _ = this
            .transport
            .set_write_timeout(Some(DEFAULT_WRITE_TIMEOUT));
        this
    }

    /// 换掉上行事件的去处。
    pub fn set_event_sink(&mut self, sink: Box<dyn EventSink>) {
        self.sink = sink;
    }

    /// 设置读超时（`None` = 一直等）。
    pub fn set_read_timeout(&mut self, timeout: Option<Duration>) -> Result<(), PipeError> {
        self.transport.set_read_timeout(timeout)?;
        Ok(())
    }

    /// 计数。
    #[must_use]
    pub fn stats(&self) -> ConnStats {
        self.stats
    }

    /// 握手是否完成。
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// 管道是否已断。
    #[must_use]
    pub fn is_broken(&self) -> bool {
        self.broken
    }

    /// 对端是否已关闭。
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// 握手：收 `Hello` ⇒ 回 `KeyEnvelope`。
    ///
    /// ⛔ **fail-closed**：版本不匹配 / Hello 非法 ⇒ **断链 + 报错**，
    /// **绝不降级成「无密钥继续跑」**。
    /// ⛔ 错误文本**不含密钥或信封内容**。
    pub fn perform_handshake(&mut self) -> Result<(), PipeError> {
        let hello = match self.read_frame() {
            Ok(Some(frame)) => frame,
            Ok(None) => {
                self.close();
                return Err(PipeError::Io(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "等待 Hello 超时",
                )));
            }
            Err(err) => {
                self.close();
                return Err(err);
            }
        };

        // 交给 P1 的握手状态机：版本检查 + 取 Hello 的 8 字节 nonce。
        // ⛔ 状态机失败 ⇒ 已进 Failed（fail-closed）。
        if let Err(err) = self.handshake.on_frame(&hello) {
            self.close();
            return Err(PipeError::Handshake(err));
        }
        let hello_nonce = match self.handshake.state() {
            HandshakeState::Handshake { hello_nonce } => *hello_nonce,
            _ => {
                self.close();
                return Err(PipeError::Handshake(HandshakeError::AlreadyFinished));
            }
        };

        // 会话密钥：⛔ 生成方式**由调用方注入**（待主理人拍板），本模块不选。
        let frame = {
            let mut session_key = match keys::make_session_key(
                self.config.session_key_source,
                &self.config.k_machine,
                &hello_nonce,
            ) {
                Ok(key) => key,
                Err(err) => {
                    self.close();
                    return Err(PipeError::Envelope(err));
                }
            };
            // GCM nonce：每次信封独立随机（线协议 §3.1）。
            let gcm_nonce = match self.config.gcm_nonce {
                Some(fixed) => fixed,
                None => match random_nonce() {
                    Ok(nonce) => nonce,
                    Err(err) => {
                        session_key.zeroize();
                        self.close();
                        return Err(err);
                    }
                },
            };
            let built = self.handshake.build_key_envelope_frame(
                &session_key,
                &self.config.container_id,
                self.config.expires_unix,
                gcm_nonce,
            );
            // ⛔ 会话密钥只在内存里 ⇒ 用完立刻清零。
            session_key.zeroize();
            built
        };
        let frame = match frame {
            Ok(frame) => frame,
            Err(err) => {
                self.close();
                return Err(PipeError::Handshake(err));
            }
        };

        if !self.emit(&frame) {
            self.close();
            return Err(PipeError::Io(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "发送密钥信封失败",
            )));
        }
        self.ready = true;
        Ok(())
    }

    /// 读一帧（**增量**解帧）。
    ///
    /// - `Ok(Some(frame))` = 读到了完整的一帧；
    /// - `Ok(None)` = 超时 / 对端关闭，还没凑够一帧（**不是错误**）；
    /// - `Err` = 协议坏了 / IO 失败 ⇒ 调用方必须断链。
    pub fn read_frame(&mut self) -> Result<Option<Frame>, PipeError> {
        loop {
            match self.decoder.next_frame() {
                Ok(DecodeOutcome::Frame(frame)) => return Ok(Some(frame)),
                Ok(DecodeOutcome::Incomplete) => {}
                Err(err) => {
                    self.broken = true;
                    return Err(PipeError::Frame(err));
                }
            }

            let mut chunk = [0u8; 4096];
            match self.transport.read(&mut chunk) {
                Ok(0) => {
                    // 对端关闭。
                    return self.on_peer_gone();
                }
                Ok(n) => self.decoder.push(&chunk[..n]),
                Err(err)
                    if matches!(
                        err.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                    ) =>
                {
                    return Ok(None)
                }
                // ⛔ 对端「带未读数据关闭」⇒ 内核发 RST ⇒ ECONNRESET。
                // 这也是「对端没了」，不是协议错误。
                Err(err)
                    if matches!(
                        err.kind(),
                        io::ErrorKind::ConnectionReset | io::ErrorKind::ConnectionAborted
                    ) =>
                {
                    return self.on_peer_gone();
                }
                Err(err) => {
                    self.broken = true;
                    return Err(PipeError::Io(err));
                }
            }
        }
    }

    /// 收发循环：读一帧、分发一帧，直到超时或断链。
    ///
    /// 返回这一轮处理了几帧。
    pub fn pump(&mut self) -> Result<usize, PipeError> {
        let mut handled = 0usize;
        while let Some(frame) = self.read_frame()? {
            self.dispatch(frame);
            handled += 1;
        }
        Ok(handled)
    }

    /// 发一帧给内核。
    ///
    /// ⛔ **绝不阻塞调用方**：管道断了 ⇒ 立刻返回 `false`（丢弃 + 计数）。
    /// ⛔ 载荷编码失败（超过 8 MiB）⇒ 同样丢弃 + 计数，**不截断**。
    pub fn emit(&mut self, frame: &Frame) -> bool {
        if self.broken {
            self.stats.frames_dropped += 1;
            return false;
        }
        let Some(bytes) = frame.encode() else {
            self.stats.frames_dropped += 1;
            return false;
        };
        match self.transport.write_all(&bytes) {
            Ok(()) => true,
            Err(_) => {
                self.broken = true;
                self.stats.frames_dropped += 1;
                false
            }
        }
    }

    /// 对端没了（EOF 或 RST）：标记断链；残留半帧 ⇒ 截断错误。
    ///
    /// 「头被截断」只有在**流结束**时才能判定——流中间它只是「还没读够」。
    fn on_peer_gone(&mut self) -> Result<Option<Frame>, PipeError> {
        self.closed = true;
        self.broken = true;
        match self.decoder.finish() {
            Ok(()) => Ok(None),
            Err(err) => Err(PipeError::Frame(err)),
        }
    }

    /// 把一帧派给去处。
    fn dispatch(&mut self, frame: Frame) {
        match frame.frame_type {
            FrameType::Event => match parse_event_meta(&frame.payload) {
                Some(meta) => {
                    self.stats.events_received += 1;
                    self.sink.on_event(&meta);
                }
                // ⛔ 必填字段缺失 / 类型不符 ⇒ 丢弃 + 计数（线协议 §4）。
                None => self.stats.events_dropped += 1,
            },
            _ => self.stats.frames_ignored += 1,
        }
    }

    /// 断链（幂等）。
    fn close(&mut self) {
        self.broken = true;
        let _ = self.transport.shutdown();
    }
}

/// 随机的 12 字节 GCM nonce（线协议 §3.1：每次信封独立随机）。
fn random_nonce() -> Result<[u8; GCM_NONCE_LEN], PipeError> {
    let mut nonce = [0u8; GCM_NONCE_LEN];
    getrandom::getrandom(&mut nonce).map_err(|_| PipeError::Random)?;
    Ok(nonce)
}
