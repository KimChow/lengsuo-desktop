//! 管道名 → OS 端点 → 启动内核进程。
//!
//! # 线协议
//!
//! 线协议 §1 第 1、2 步：
//!
//! 1. runner 生成管道名（随机，不含敏感信息）；
//! 2. runner 启动内核，命令行**追加** `--lengsuo-kernel-pipe=<name>`。
//!
//! ⛔ **命令行只带管道名**：不带密钥、不带配置内容、不带任何指纹值。
//!
//! # 端点映射（线协议 §1「管道名 → OS 端点的映射」）
//!
//! | 平台 | 端点 |
//! |---|---|
//! | Windows | 命名管道 `\\.\pipe\lengsuo-kernel-<name>` |
//! | Linux / POSIX | **抽象命名空间** unix 域套接字 `\0lengsuo-kernel-<name>` |
//!
//! ⛔ 前缀 `lengsuo-kernel-` 两侧必须**逐字节**一致；
//! ⛔ **不许用文件系统路径**（两侧运行目录不同会错位）。
//!
//! # 本模块放什么
//!
//! - **端点映射**（[`Endpoint`]）与管道名的生成/校验；
//! - **传输层**（[`imp`]）——平台相关的 socket 抽象；
//! - **服务端配置**（[`PipeServerConfig`]）与错误类型（[`PipeError`]）；
//! - **启动内核**（[`LaunchConfig`] / [`launch`]）。
//!
//! # 边界
//!
//! - ⛔ 不实现 profile 打包上传（任务 R）。
//! - ⛔ 不实现平台 HTTP 上行。
//! - ⛔ 不决定内核该不该启动、也不解释退出码语义（那在上层）。

use std::io;
use std::process::Command;

use getrandom::getrandom;
use zeroize::Zeroize;

use crate::kernel_pipe::handshake::HandshakeError;
use crate::kernel_pipe::keys::{
    self, EnvelopeError, SessionKeySource, GCM_NONCE_LEN, K_MACHINE_LEN,
};
use crate::kernel_pipe::FrameError;

/// 端点前缀（线协议 §1）。
///
/// ⛔ 两侧必须**逐字节**一致（内核侧是 `kKernelPipeEndpointPrefix`）。
pub const ENDPOINT_PREFIX: &str = "lengsuo-kernel-";

/// 内核命令行里的管道参数前缀（线协议 §1：命令行**只带管道名**）。
pub const PIPE_ARG_PREFIX: &str = "--lengsuo-kernel-pipe=";

/// 管道名的字符集：只允许字母、数字、`-`、`_`。
///
/// ⛔ 管道名会进**命令行**与**端点名** ⇒ 不许有分隔符、空格或控制字符。
const NAME_CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_";

/// 管道名的长度上限（字节）。
///
/// 抽象命名空间的名字要塞进 `sun_path`（108 字节）⇒ 加上前缀仍有富余。
pub const MAX_PIPE_NAME_LEN: usize = 64;

/// 管道名的随机源字节数（十六进制 ⇒ 32 个字符）。
const PIPE_NAME_RANDOM_BYTES: usize = 16;

// ---------------------------------------------------------------------------
// 端点映射
// ---------------------------------------------------------------------------

/// 端点形态（线协议 §1「管道名 → OS 端点的映射」）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Endpoint {
    /// Windows 命名管道：`\\.\pipe\lengsuo-kernel-<name>`。
    WindowsNamedPipe {
        /// 管道名（不含前缀）。
        name: String,
    },
    /// POSIX 抽象命名空间 unix 域套接字：`\0lengsuo-kernel-<name>`。
    AbstractUnix {
        /// 管道名（不含前缀）。
        name: String,
    },
}

impl Endpoint {
    /// 按**当前平台**选端点形态。
    #[must_use]
    pub fn for_pipe_name(name: &str) -> Self {
        if cfg!(windows) {
            Self::WindowsNamedPipe {
                name: name.to_string(),
            }
        } else {
            Self::AbstractUnix {
                name: name.to_string(),
            }
        }
    }

    /// 管道名（不含前缀）。
    #[must_use]
    pub fn pipe_name(&self) -> &str {
        match self {
            Self::WindowsNamedPipe { name } | Self::AbstractUnix { name } => name,
        }
    }

    /// OS 层面的端点字符串（与内核侧看到的**逐字节**一致）。
    #[must_use]
    pub fn path(&self) -> String {
        match self {
            Self::WindowsNamedPipe { name } => windows_endpoint(name),
            Self::AbstractUnix { name } => posix_endpoint(name),
        }
    }
}

/// Windows 端点：`\\.\pipe\lengsuo-kernel-<name>`。
#[must_use]
pub fn windows_endpoint(name: &str) -> String {
    format!(r"\\.\pipe\{ENDPOINT_PREFIX}{name}")
}

/// POSIX 端点：抽象命名空间 `\0lengsuo-kernel-<name>`。
///
/// ⛔ 前导的 `\0` 是**命名空间的标记**（不是路径分隔符）⇒ 端点**不在文件系统里**。
/// 传给内核的 `sun_path` 字节是「NUL + 名字」，NUL 不算进名字本身。
#[must_use]
pub fn posix_endpoint(name: &str) -> String {
    format!("\0{ENDPOINT_PREFIX}{name}")
}

/// 抽象命名空间里的**名字**（不含前导 NUL——NUL 由 OS 语义隐含）。
#[must_use]
pub fn abstract_name(name: &str) -> String {
    format!("{ENDPOINT_PREFIX}{name}")
}

// ---------------------------------------------------------------------------
// 传输层（平台相关）
// ---------------------------------------------------------------------------

/// 传输层：Linux = unix 域套接字（**抽象命名空间**）。
#[cfg(target_os = "linux")]
pub(crate) mod imp {
    use std::io::{self, Read, Write};
    use std::time::Duration;

    /// 监听端点。
    pub(crate) struct Listener {
        inner: std::os::unix::net::UnixListener,
    }

    impl Listener {
        /// 绑定抽象命名空间端点（非阻塞，便于轮询超时）。
        pub(crate) fn bind(abstract_name: &str) -> io::Result<Self> {
            use std::os::linux::net::SocketAddrExt;
            let addr = std::os::unix::net::SocketAddr::from_abstract_name(abstract_name.as_bytes())
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
            let inner = std::os::unix::net::UnixListener::bind_addr(&addr)?;
            inner.set_nonblocking(true)?;
            Ok(Self { inner })
        }

        /// 接受一个连接；**没有连接时**返回 `Ok(None)`（不阻塞）。
        pub(crate) fn accept(&self) -> io::Result<Option<Transport>> {
            match self.inner.accept() {
                Ok((stream, _addr)) => {
                    // Linux 的 accept 不保证继承 O_NONBLOCK ⇒ 显式写死。
                    stream.set_nonblocking(false)?;
                    Ok(Some(Transport { inner: stream }))
                }
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => Ok(None),
                Err(err) => Err(err),
            }
        }
    }

    /// 已接受的连接。
    pub(crate) struct Transport {
        inner: std::os::unix::net::UnixStream,
    }

    impl Transport {
        pub(crate) fn set_read_timeout(&mut self, timeout: Option<Duration>) -> io::Result<()> {
            self.inner.set_read_timeout(timeout)
        }

        pub(crate) fn set_write_timeout(&mut self, timeout: Option<Duration>) -> io::Result<()> {
            self.inner.set_write_timeout(timeout)
        }

        pub(crate) fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.inner.read(buf)
        }

        pub(crate) fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
            self.inner.write_all(bytes)
        }

        pub(crate) fn shutdown(&self) -> io::Result<()> {
            self.inner.shutdown(std::net::Shutdown::Both)
        }
    }
}

/// 传输层占位：非 Linux 平台本卡**未实现**。
#[cfg(not(target_os = "linux"))]
pub(crate) mod imp {
    use std::io;
    use std::time::Duration;

    const UNSUPPORTED: &str = "本卡只实现了 Linux 抽象命名空间的传输层";

    fn unsupported() -> io::Error {
        io::Error::new(io::ErrorKind::Unsupported, UNSUPPORTED)
    }

    /// 监听端点占位。
    pub(crate) struct Listener;

    impl Listener {
        pub(crate) fn bind(_abstract_name: &str) -> io::Result<Self> {
            Err(unsupported())
        }

        pub(crate) fn accept(&self) -> io::Result<Option<Transport>> {
            Err(unsupported())
        }
    }

    /// 连接占位。
    pub(crate) struct Transport;

    impl Transport {
        pub(crate) fn set_read_timeout(&mut self, _timeout: Option<Duration>) -> io::Result<()> {
            Ok(())
        }

        pub(crate) fn set_write_timeout(&mut self, _timeout: Option<Duration>) -> io::Result<()> {
            Ok(())
        }

        pub(crate) fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(unsupported())
        }

        pub(crate) fn write_all(&mut self, _bytes: &[u8]) -> io::Result<()> {
            Err(unsupported())
        }

        pub(crate) fn shutdown(&self) -> io::Result<()> {
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// 错误
// ---------------------------------------------------------------------------

/// 管道服务端的失败原因。
///
/// ⛔ 这些文本**不得**包含会话密钥或信封内容——只说「哪一步失败」。
#[derive(Debug, thiserror::Error)]
pub enum PipeError {
    /// 管道名不合法（空、太长、含非法字符）。
    #[error("管道名不合法")]
    BadPipeName,
    /// 当前平台没有实现传输层（本卡只实现 Linux 抽象命名空间）。
    #[error("当前平台未实现管道传输层")]
    UnsupportedPlatform,
    /// IO 失败。
    #[error("管道 IO 失败：{0}")]
    Io(#[from] io::Error),
    /// 握手失败（版本不匹配 / Hello 非法 / 信封造不出来）。
    #[error("握手失败：{0}")]
    Handshake(#[from] HandshakeError),
    /// 帧解码失败（畸形帧）。
    #[error("帧解码失败：{0}")]
    Frame(#[from] FrameError),
    /// 信封失败。
    #[error("密钥信封失败：{0}")]
    Envelope(#[from] EnvelopeError),
    /// 随机源失败。
    #[error("取随机数失败")]
    Random,
}

// ---------------------------------------------------------------------------
// 服务端配置
// ---------------------------------------------------------------------------

/// 管道服务端的配置。
#[derive(Clone)]
pub struct PipeServerConfig {
    /// 会话密钥的生成方式（⛔ 待主理人拍板 ⇒ **由调用方注入**）。
    pub session_key_source: SessionKeySource,
    /// 容器 id（进信封**明文**）。
    pub container_id: Vec<u8>,
    /// 会话密钥的过期时间（Unix 秒）——内核据此 fail-closed。
    pub expires_unix: u64,
    /// `k_machine`（机器码派生密钥）——信封用它封，会话密钥用它派生。
    pub k_machine: [u8; K_MACHINE_LEN],
    /// 密钥信封的 GCM nonce。
    ///
    /// `None` = **每次信封独立随机**（线协议 §3.1，生产路径）。
    /// `Some` = 固定 nonce（只给固定测试向量比对用；GCM 下复用 nonce 是禁忌）。
    pub gcm_nonce: Option<[u8; GCM_NONCE_LEN]>,
}

impl std::fmt::Debug for PipeServerConfig {
    /// ⛔ 不打印 `k_machine`。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PipeServerConfig")
            .field("session_key_source", &self.session_key_source)
            .field("container_id_len", &self.container_id.len())
            .field("expires_unix", &self.expires_unix)
            .field("k_machine", &"<redacted>")
            .finish()
    }
}

impl Drop for PipeServerConfig {
    fn drop(&mut self) {
        // ⛔ 密钥材料离开作用域即清零。
        self.k_machine.zeroize();
    }
}

impl PipeServerConfig {
    /// 用**本机真实值**派生 `k_machine`。
    ///
    /// ⚠️ 机器码的**取值**是本机真值（两侧必须采到同一组值才能互认）。
    #[must_use]
    pub fn from_machine(
        session_key_source: SessionKeySource,
        container_id: impl Into<Vec<u8>>,
        expires_unix: u64,
    ) -> Self {
        let machine_code = keys::machine_code(&super::machine::collect_machine_inputs());
        Self {
            session_key_source,
            container_id: container_id.into(),
            expires_unix,
            k_machine: keys::k_machine(&machine_code),
            gcm_nonce: None,
        }
    }

    /// 直接给定 `k_machine`（测试用固定向量）。
    #[must_use]
    pub fn with_k_machine(
        session_key_source: SessionKeySource,
        container_id: impl Into<Vec<u8>>,
        expires_unix: u64,
        k_machine: [u8; K_MACHINE_LEN],
    ) -> Self {
        Self {
            session_key_source,
            container_id: container_id.into(),
            expires_unix,
            k_machine,
            gcm_nonce: None,
        }
    }
}

// ---------------------------------------------------------------------------
// 管道名
// ---------------------------------------------------------------------------

/// 生成一个随机的管道名。
///
/// ⛔ **随机**（CSPRNG）且**不含敏感信息**（线协议 §1）。
/// 形状：小写十六进制 ⇒ 只含安全字符，能直接进命令行与端点名。
pub fn random_pipe_name() -> Result<String, PipeError> {
    let mut bytes = [0u8; PIPE_NAME_RANDOM_BYTES];
    getrandom(&mut bytes).map_err(|_| PipeError::Random)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// 校验管道名：非空、不太长、只含安全字符。
///
/// ⛔ 管道名会进**命令行**与**端点名** ⇒ 不许有分隔符、空格或控制字符。
pub fn validate_pipe_name(name: &str) -> Result<(), PipeError> {
    if name.is_empty() || name.len() > MAX_PIPE_NAME_LEN {
        return Err(PipeError::BadPipeName);
    }
    if !name.bytes().all(|b| NAME_CHARS.contains(&b)) {
        return Err(PipeError::BadPipeName);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 命令行
// ---------------------------------------------------------------------------

/// 拼内核的命令行参数：只追加 `--lengsuo-kernel-pipe=<name>`。
///
/// ⛔ **只带管道名**：不带密钥、不带容器 id、不带任何指纹值。
#[must_use]
pub fn pipe_arg(name: &str) -> String {
    format!("{PIPE_ARG_PREFIX}{name}")
}

/// 内核启动器的配置。
#[derive(Debug, Clone)]
pub struct LaunchConfig {
    /// 内核可执行文件路径。
    pub program: String,
    /// 内核的**原有**参数（逐字传给内核，本卡不改）。
    pub base_args: Vec<String>,
    /// 管道名（由 [`random_pipe_name`] 生成，或由调用方指定）。
    pub pipe_name: String,
}

impl LaunchConfig {
    /// 用**随机管道名**造一份配置。
    pub fn new(program: impl Into<String>, base_args: Vec<String>) -> Result<Self, PipeError> {
        Ok(Self {
            program: program.into(),
            base_args,
            pipe_name: random_pipe_name()?,
        })
    }

    /// 用**指定**管道名造一份配置（测试用）。
    #[must_use]
    pub fn with_pipe_name(
        program: impl Into<String>,
        base_args: Vec<String>,
        pipe_name: impl Into<String>,
    ) -> Self {
        Self {
            program: program.into(),
            base_args,
            pipe_name: pipe_name.into(),
        }
    }

    /// 拼出完整 argv（**逐字节**可断言）。
    ///
    /// ⛔ 最后一个参数恒为 `--lengsuo-kernel-pipe=<name>`，
    /// ⛔ 其它参数**原样**保留（不追加任何密钥/指纹）。
    #[must_use]
    pub fn argv(&self) -> Vec<String> {
        let mut argv = self.base_args.clone();
        argv.push(pipe_arg(&self.pipe_name));
        argv
    }
}

// ---------------------------------------------------------------------------
// 启动
// ---------------------------------------------------------------------------

/// 启动内核的失败原因。
///
/// ⛔ 错误文本**不含密钥或信封内容**。
#[derive(Debug, thiserror::Error)]
pub enum LaunchError {
    /// 拉起进程失败（文件不存在 / 权限不足 / …）。
    #[error("启动内核失败（程序 {program}）：{source}")]
    Spawn {
        /// 内核可执行文件路径。
        program: String,
        /// 底层 IO 错误。
        source: std::io::Error,
    },

    /// 内核**立刻退出**（没跑起来）。
    ///
    /// ⛔ 这时必须清理端点——**不留半开的管道**。
    #[error("内核进程立刻退出：{status}")]
    ExitedImmediately {
        /// 退出状态。
        status: String,
    },
}

/// 启动内核进程。
///
/// ⛔ **只追加管道参数**——不设环境变量里的密钥、不传配置内容。
/// ⛔ 启动失败 ⇒ 报错并**清理端点**（调用方负责关掉监听端点；
/// 本卡不留下半开的管道——抽象命名空间的端点随监听 fd 关闭而消失）。
///
/// 返回子进程句柄与**实际用的** argv（便于日志/测试核对）。
pub fn launch(config: &LaunchConfig) -> Result<(std::process::Child, Vec<String>), LaunchError> {
    let argv = config.argv();
    let child = Command::new(&config.program)
        .args(&argv)
        .spawn()
        .map_err(|source| LaunchError::Spawn {
            program: config.program.clone(),
            source,
        })?;
    Ok((child, argv))
}

/// 确认内核**没有立刻退出**：给它一小段时间，还活着就返回 `Ok`。
///
/// ⛔ 立刻退出 ⇒ 报 [`LaunchError::ExitedImmediately`]（调用方据此清理端点）。
/// ⚠️ 不解释退出码——退出码语义不在本卡范围。
pub fn check_started(
    child: &mut std::process::Child,
    grace: std::time::Duration,
) -> Result<(), LaunchError> {
    if let Some(status) = child.try_wait().map_err(|source| LaunchError::Spawn {
        program: "<已启动的内核>".to_string(),
        source,
    })? {
        return Err(LaunchError::ExitedImmediately {
            status: status.to_string(),
        });
    }
    std::thread::sleep(grace);
    match child.try_wait() {
        Ok(Some(status)) => Err(LaunchError::ExitedImmediately {
            status: status.to_string(),
        }),
        Ok(None) => Ok(()),
        Err(source) => Err(LaunchError::Spawn {
            program: "<已启动的内核>".to_string(),
            source,
        }),
    }
}

#[cfg(test)]
mod launch_tests {
    use super::*;
    use crate::kernel_pipe::tests::{K_MACHINE_HEX, SESSION_KEY_HEX};

    #[test]
    fn the_endpoint_prefix_is_byte_for_byte_the_contract_one() {
        // ⛔ 两侧必须逐字节一致（内核侧 `kKernelPipeEndpointPrefix`）。
        assert_eq!(ENDPOINT_PREFIX, "lengsuo-kernel-");
        assert_eq!(
            ENDPOINT_PREFIX.as_bytes(),
            b"lengsuo-kernel-",
            "前缀必须逐字节相同"
        );
        assert_eq!(ENDPOINT_PREFIX.len(), 15);
    }

    #[test]
    fn the_windows_endpoint_is_a_named_pipe_under_the_prefix() {
        let endpoint = windows_endpoint("abc123");
        assert_eq!(endpoint, r"\\.\pipe\lengsuo-kernel-abc123");
        // ⛔ 必须是命名管道形态：不是文件路径。
        assert!(endpoint.starts_with(r"\\.\pipe\"), "必须是命名管道");
        assert!(!endpoint.contains('/'), "命名管道不是文件路径");
        assert_eq!(
            endpoint.strip_prefix(r"\\.\pipe\").expect("应有前缀"),
            "lengsuo-kernel-abc123",
            "去掉命名管道前缀后必须正好是端点前缀 + 管道名"
        );
    }

    #[test]
    fn the_posix_endpoint_is_an_abstract_namespace_name_not_a_path() {
        let endpoint = posix_endpoint("abc123");
        // ⛔ 抽象命名空间：前导 NUL。
        assert!(endpoint.starts_with('\0'), "POSIX 端点必须有 \\0 前缀");
        assert_eq!(endpoint, "\0lengsuo-kernel-abc123");
        // ⛔ **不是文件路径**：不许有 `/`，也不许有路径分隔符。
        assert!(!endpoint.contains('/'), "抽象命名空间不是文件路径");
        assert!(!endpoint.contains('/'), "不许出现路径分隔符");
        // ⛔ 前缀必须与 Windows 侧**逐字节**一致（同一个常量）。
        assert_eq!(
            endpoint.trim_start_matches('\0'),
            windows_endpoint("abc123")
                .strip_prefix(r"\\.\pipe\")
                .expect("应有前缀"),
            "两侧的端点前缀必须逐字节相同"
        );
    }

    #[test]
    fn the_abstract_name_carries_no_leading_nul() {
        // NUL 由 OS 语义隐含（`sun_path[0] = 0`），不算进名字本身。
        assert_eq!(abstract_name("abc123"), "lengsuo-kernel-abc123");
        assert!(!abstract_name("abc123").starts_with('\0'));
    }

    #[test]
    fn the_endpoint_picks_the_shape_of_the_current_platform() {
        let endpoint = Endpoint::for_pipe_name("abc123");
        if cfg!(windows) {
            assert_eq!(
                endpoint,
                Endpoint::WindowsNamedPipe {
                    name: "abc123".into()
                }
            );
            assert!(endpoint.path().starts_with(r"\\.\pipe\"));
        } else {
            assert_eq!(
                endpoint,
                Endpoint::AbstractUnix {
                    name: "abc123".into()
                }
            );
            assert!(endpoint.path().starts_with('\0'));
        }
        assert_eq!(endpoint.pipe_name(), "abc123");
    }

    #[test]
    fn a_pipe_name_with_unsafe_characters_is_rejected() {
        // ⛔ 管道名会进命令行与端点名 ⇒ 不许有分隔符、空格、控制字符。
        for bad in ["", "a b", "a/b", "a\0b", "a\nb", "a;b", "../etc/passwd"] {
            assert!(
                validate_pipe_name(bad).is_err(),
                "管道名 {bad:?} 必须被拒绝"
            );
        }
        assert!(validate_pipe_name("abc-123_XYZ").is_ok());
        assert!(
            validate_pipe_name(&"a".repeat(MAX_PIPE_NAME_LEN + 1)).is_err(),
            "超长管道名必须被拒绝"
        );
    }

    #[test]
    fn a_random_pipe_name_is_safe_and_different_each_time() {
        let a = random_pipe_name().expect("应可生成");
        let b = random_pipe_name().expect("应可生成");
        assert_ne!(a, b, "两次随机必须不同");
        assert!(validate_pipe_name(&a).is_ok(), "随机名必须通过校验");
        // ⛔ 管道名只含小写十六进制 ⇒ 绝不带敏感信息，也不带路径分隔符。
        assert!(a
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
    }

    // ---------------------------------------------------------------------------
    // 增量解帧
    // ---------------------------------------------------------------------------

    #[test]
    fn the_command_line_carries_only_the_pipe_name() {
        // ⛔ 命令行**只带管道名**：不带密钥、不带容器 id、不带任何指纹值。
        let config = LaunchConfig::with_pipe_name(
            "/usr/bin/lengsuo-browser",
            vec![
                "--headless".to_string(),
                "--user-data-dir=/tmp/x".to_string(),
            ],
            "abc123",
        );
        let argv = config.argv();
        assert_eq!(argv.len(), 3, "原有参数 + 一个管道参数");
        assert_eq!(argv[0], "--headless");
        assert_eq!(argv[1], "--user-data-dir=/tmp/x");
        assert_eq!(argv[2], "--lengsuo-kernel-pipe=abc123");

        // ⛔ 逐字检查：不含密钥 / 容器 id / 指纹值。
        let joined = argv.join(" ");
        assert!(!joined.contains(SESSION_KEY_HEX), "命令行不得含会话密钥");
        assert!(!joined.contains(K_MACHINE_HEX), "命令行不得含 k_machine");
        assert!(!joined.contains("ct-01"), "命令行不得含容器 id");
        assert!(!joined.contains("64fa1b15"), "命令行不得含指纹值");
        // ⛔ 只带管道名本身。
        assert!(joined.contains("abc123"), "命令行必须带管道名");
    }

    #[test]
    fn the_pipe_arg_is_exactly_the_contract_shape() {
        assert_eq!(pipe_arg("abc123"), "--lengsuo-kernel-pipe=abc123");
        assert_eq!(
            PIPE_ARG_PREFIX, "--lengsuo-kernel-pipe=",
            "参数前缀必须与线协议一致"
        );
    }

    #[test]
    fn launching_a_missing_program_fails_with_a_clear_error() {
        let config = LaunchConfig::with_pipe_name(
            "/nonexistent/lengsuo-browser-does-not-exist",
            vec![],
            "abc123",
        );
        let err = launch(&config).expect_err("不存在的程序必须启动失败");
        let text = err.to_string();
        assert!(text.contains("启动内核失败"), "错误文本：{text}");
        // ⛔ 错误文本不得含密钥 / 指纹。
        assert!(!text.contains(SESSION_KEY_HEX));
        assert!(!text.contains(K_MACHINE_HEX));
    }

    // ---------------------------------------------------------------------------
    // 会话密钥来源（可注入）
    // ---------------------------------------------------------------------------
}
