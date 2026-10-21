//! 机器码派生 + 会话密钥信封（线协议 §1 / §3）。
//!
//! 关键语义（`docs/kernel-wire-contract.md` §1）：
//! - `machine_code` = 采集 `{os, cpuid, hddid, computername}` → **固定 JSON 键序**
//!   序列化 → **UTF-16LE**（无 BOM）→ SHA-256 → 十六进制小写。
//!   ⛔ **形状**对齐（同样四个字段、同样键序），**取值必须是本机真实值**
//!   —— 不复制任何一方的常量字符串。
//! - `k_machine = SHA-256(machine_code 的 ASCII 字节)`。
//! - 会话密钥 32 字节、随机、**只存内存**；⛔ 不写盘、不进环境变量、
//!   不进命令行、不进日志。
//! - `session_key = HKDF-SHA256(ikm=k_machine, salt=nonce, info="lengsuo-kernel-session-v1", L=32)`。
//!
//! ⛔ 错误文本**不得**包含密钥或信封内容。

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use hkdf::Hkdf;
use sha2::{Digest, Sha256};
use thiserror::Error;
use zeroize::Zeroizing;

/// 会话密钥长度（线协议 §1：32 字节）。
pub const SESSION_KEY_LEN: usize = 32;

/// 机器码派生密钥长度（SHA-256 输出）。
pub const K_MACHINE_LEN: usize = 32;

/// 握手 nonce 长度（线协议 §1：8 字节）。
pub const NONCE_LEN: usize = 8;

/// GCM 认证标签长度。
pub const GCM_TAG_LEN: usize = 16;

/// GCM nonce 长度（GCM 规范的 96 位）。
pub const GCM_NONCE_LEN: usize = 12;

/// HKDF 的 `info` 串（线协议 §1，逐字节对齐）。
pub const SESSION_KEY_INFO: &[u8] = b"lengsuo-kernel-session-v1";

/// 密钥派生 / 信封错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum KeyError {
    /// 取随机数失败（CSPRNG 不可用）。
    #[error("随机数源不可用")]
    RandomUnavailable,
    /// HKDF 派生长度非法。
    #[error("会话密钥派生失败")]
    DerivationFailed,
    /// 密钥长度不是 32 字节。
    #[error("密钥长度非法")]
    InvalidKeyLength,
    /// 信封结构非法（长度字段自相矛盾或被截断）。
    #[error("密钥信封结构非法")]
    MalformedEnvelope,
    /// 认证失败：密钥不对、或密文/标签被改过。
    ///
    /// ⛔ 失败时**不得**区分「密钥错」与「密文被改」——那会给攻击者一个 oracle。
    #[error("密钥信封认证失败")]
    AuthenticationFailed,
}

/// 机器码的原始取值（四个字段，**取值来自本机**）。
///
/// 字段名与顺序即线协议 §1 的一部分，改动会让跨仓派生出不同的 `k_machine`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineInputs {
    /// 操作系统标识。
    pub os: String,
    /// CPU 标识。
    pub cpuid: String,
    /// 磁盘标识。
    pub hddid: String,
    /// 计算机名。
    pub computername: String,
}

impl MachineInputs {
    /// 按**固定键序**序列化成机器码 JSON。
    ///
    /// 键序固定为 `os, cpuid, hddid, computername`（线协议 §1）。
    /// ⛔ 用手写拼接而非 `serde_json`：serde 的 map 顺序依赖实现，
    /// 而这里的键序是**跨仓协议的一部分**，必须逐字节可控。
    #[must_use]
    pub fn to_json(&self) -> String {
        format!(
            "{{\"os\":{},\"cpuid\":{},\"hddid\":{},\"computername\":{}}}",
            json_string(&self.os),
            json_string(&self.cpuid),
            json_string(&self.hddid),
            json_string(&self.computername),
        )
    }
}

/// 采集**本机真实**的机器码输入（线协议 §1）。
///
/// ⛔ **形状**对齐跨仓线协议（同样四个字段、同样键序），
/// 但**取值必须是本机真实值**——不复制任何一方的常量字符串。
/// ⛔ 采集不到的字段退化成**空串**（不是占位常量串）：形状仍是四字段、
/// 键序不变 ⇒ 跨仓派生的 `k_machine` 仍能对齐。
///
/// ⚠️ 本函数**只读**系统信息：不写盘、不发网络请求、不写日志。
#[must_use]
pub fn collect_machine_inputs() -> MachineInputs {
    MachineInputs {
        os: std::env::consts::OS.to_string(),
        cpuid: collect_cpuid(),
        hddid: collect_hddid(),
        computername: collect_computername(),
    }
}

/// 读一个文本文件，去掉首尾空白；读不到或为空则返回 `None`。
///
/// ⚠️ 只有 Linux / macOS 走文件读取（Windows 走 API），故按平台 gate，
/// 否则在 Windows 上会触发 `dead_code` 警告（clippy 带 `-D warnings` ⇒ 整卡失败）。
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn read_trimmed(path: &str) -> Option<String> {
    let raw = std::fs::read_to_string(path).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// CPU 标识。
#[cfg(target_os = "linux")]
fn collect_cpuid() -> String {
    let Some(info) = read_trimmed("/proc/cpuinfo") else {
        return String::new();
    };
    let mut vendor = String::new();
    let mut model = String::new();
    for line in info.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "vendor_id" if vendor.is_empty() => vendor = value.to_string(),
            "model name" if model.is_empty() => model = value.to_string(),
            _ => {}
        }
    }
    match (vendor.is_empty(), model.is_empty()) {
        (true, true) => String::new(),
        (true, false) => model,
        (false, true) => vendor,
        (false, false) => format!("{vendor} {model}"),
    }
}

/// CPU 标识（Windows x86_64：CPUID 指令取厂商串 + brand string）。
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
fn collect_cpuid() -> String {
    use std::arch::x86_64::__cpuid;
    // 叶子 0：最高叶子号 + 厂商字符串（EBX, EDX, ECX）。
    let leaf0 = __cpuid(0);
    let mut vendor_bytes = [0u8; 12];
    vendor_bytes[..4].copy_from_slice(&leaf0.ebx.to_le_bytes());
    vendor_bytes[4..8].copy_from_slice(&leaf0.edx.to_le_bytes());
    vendor_bytes[8..].copy_from_slice(&leaf0.ecx.to_le_bytes());
    let vendor = String::from_utf8_lossy(&vendor_bytes).trim().to_string();

    if leaf0.eax < 0x8000_0000 {
        return vendor;
    }
    // 叶子 0x80000002..=0x80000004：brand string 的 48 个字节。
    let mut brand = String::new();
    for leaf in 0x8000_0002u32..=0x8000_0004u32 {
        let result = __cpuid(leaf);
        for word in [result.eax, result.ebx, result.ecx, result.edx] {
            brand.push_str(&String::from_utf8_lossy(&word.to_le_bytes()));
        }
    }
    let brand = brand.trim().to_string();
    match (vendor.is_empty(), brand.is_empty()) {
        (true, true) => String::new(),
        (true, false) => brand,
        (false, true) => vendor,
        (false, false) => format!("{vendor} {brand}"),
    }
}

/// CPU 标识（Windows 非 x86_64：退化为空串）。
#[cfg(all(target_os = "windows", not(target_arch = "x86_64")))]
fn collect_cpuid() -> String {
    String::new()
}

/// CPU 标识（macOS：CPU 品牌串）。
#[cfg(target_os = "macos")]
fn collect_cpuid() -> String {
    read_trimmed("/sys/devices/system/cpu/cpu0/cache/index0/size").unwrap_or_default()
}

/// CPU 标识（其它平台：退化为空串）。
#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
fn collect_cpuid() -> String {
    String::new()
}

/// 磁盘标识。
#[cfg(target_os = "linux")]
fn collect_hddid() -> String {
    // 取第一块带序列号的块设备（跳过 loop / ram 设备）。
    let Ok(entries) = std::fs::read_dir("/sys/block") else {
        return String::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| !name.starts_with("loop") && !name.starts_with("ram"))
        .collect();
    names.sort();
    for name in names {
        if let Some(serial) = read_trimmed(&format!("/sys/block/{name}/device/serial")) {
            return format!("{name}:{serial}");
        }
    }
    String::new()
}

/// 磁盘标识（Windows：系统卷的创建时间 + 大小）。
///
/// ⚠️ `volume_serial_number` / `file_index` 在标准库里仍是
/// **nightly-only**（`windows_by_handle`），stable 工具链取不到。
/// ⇒ 这里只用 **stable** 的 `creation_time` + `file_size`，
/// 同样随机器而异、跨重启稳定。
#[cfg(target_os = "windows")]
fn collect_hddid() -> String {
    use std::os::windows::fs::MetadataExt;
    let Ok(metadata) = std::fs::metadata("C:\\") else {
        return String::new();
    };
    format!("C:{}:{}", metadata.creation_time(), metadata.file_size())
}

/// 磁盘标识（macOS：设备 UUID）。
#[cfg(target_os = "macos")]
fn collect_hddid() -> String {
    read_trimmed("/private/var/db/dbus/uuid").unwrap_or_default()
}

/// 磁盘标识（其它平台：退化为空串）。
#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
fn collect_hddid() -> String {
    String::new()
}

/// 计算机名。
#[cfg(target_os = "linux")]
fn collect_computername() -> String {
    read_trimmed("/proc/sys/kernel/hostname").unwrap_or_default()
}

/// 计算机名（Windows：环境变量 `COMPUTERNAME`）。
#[cfg(target_os = "windows")]
fn collect_computername() -> String {
    std::env::var("COMPUTERNAME").unwrap_or_default()
}

/// 计算机名（macOS：`/etc/hostname`）。
///
/// ⛔ 本 crate 是 `#![forbid(unsafe_code)]`，所以**不**走 `gethostname` 的
/// libc FFI 绑定（那需要 `unsafe`）——改读同一份系统信息文件。
#[cfg(target_os = "macos")]
fn collect_computername() -> String {
    read_trimmed("/etc/hostname").unwrap_or_default()
}

/// 计算机名（其它平台：环境变量 `HOSTNAME`）。
#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
fn collect_computername() -> String {
    std::env::var("HOSTNAME").unwrap_or_default()
}

/// 把一个字符串编码成 JSON 字符串字面量（含引号与转义）。
fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// 机器码 = `SHA-256(UTF-16LE(JSON))` 的**十六进制小写**（线协议 §1）。
#[must_use]
pub fn machine_code(inputs: &MachineInputs) -> String {
    let json = inputs.to_json();
    let utf16: Vec<u8> = json
        .encode_utf16()
        .flat_map(|unit| unit.to_le_bytes())
        .collect();
    let digest = Sha256::digest(&utf16);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// `k_machine = SHA-256(machine_code 的 ASCII 字节)`（线协议 §1）。
///
/// 入参是 [`machine_code`] 的输出（64 个十六进制字符），
/// **不是**原始 JSON——两者派生出完全不同的密钥，不能混。
#[must_use]
pub fn k_machine(machine_code_hex: &str) -> Zeroizing<[u8; K_MACHINE_LEN]> {
    let digest = Sha256::digest(machine_code_hex.as_bytes());
    let mut key = Zeroizing::new([0u8; K_MACHINE_LEN]);
    key.copy_from_slice(&digest);
    key
}

/// 派生会话密钥：`HKDF-SHA256(ikm=k_machine, salt=nonce, info, L=32)`。
///
/// ⛔ 参数位置必须与线协议 §1 一致：`ikm` = `k_machine`、`salt` = **nonce（非空）**、
/// `info` = [`SESSION_KEY_INFO`]、`L` = 32。salt 留空或与 ikm 对调都会派生出不同的值。
pub fn derive_session_key(
    k_machine: &[u8; K_MACHINE_LEN],
    nonce: &[u8; NONCE_LEN],
) -> Result<Zeroizing<[u8; SESSION_KEY_LEN]>, KeyError> {
    let hk = Hkdf::<Sha256>::new(Some(nonce), k_machine);
    let mut okm = Zeroizing::new([0u8; SESSION_KEY_LEN]);
    hk.expand(SESSION_KEY_INFO, okm.as_mut())
        .map_err(|_| KeyError::DerivationFailed)?;
    Ok(okm)
}

/// 生成 32 字节随机会话密钥（CSPRNG）。
pub fn random_session_key() -> Result<Zeroizing<[u8; SESSION_KEY_LEN]>, KeyError> {
    let mut key = Zeroizing::new([0u8; SESSION_KEY_LEN]);
    getrandom::getrandom(key.as_mut()).map_err(|_| KeyError::RandomUnavailable)?;
    Ok(key)
}

/// 生成 8 字节随机 nonce。
pub fn random_nonce() -> Result<Zeroizing<[u8; NONCE_LEN]>, KeyError> {
    let mut nonce = Zeroizing::new([0u8; NONCE_LEN]);
    getrandom::getrandom(nonce.as_mut()).map_err(|_| KeyError::RandomUnavailable)?;
    Ok(nonce)
}

/// 会话密钥信封的**明文**部分。
///
/// 线协议 §3 / 任务书：信封加密的是 `{session_key, container_id, expires_unix}`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvelopePlaintext {
    /// 容器 id（平台下发）。
    pub container_id: String,
    /// 会话密钥 32 字节。
    pub session_key: [u8; SESSION_KEY_LEN],
    /// 过期时间（Unix 秒）。
    pub expires_unix: u64,
}

/// 密钥信封的**线格式**（线协议 §3）。
///
/// ```text
/// u32 container_id_len + container_id 字节
/// u32 key_len          + key 字节
/// u64 expires_unix
/// u32 tag_len          + GCM tag 字节
/// ```
///
/// - `key` 段 = **GCM nonce(12B) || 密文**。密文是
///   `{session_key, container_id, expires_unix}` 的 AES-256-GCM 密文
///   （会话密钥本体**永不明文过线**）。
/// - GCM 标签**独立成段**（`tag_len` 显式给出长度），不与密文连成一段。
/// - 外层的 `container_id` / `expires_unix` 是明文副本；解密时与密文内的
///   权威副本逐字节比对，对不上即认证失败（否则外层副本就成了可篡改的路由信息）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEnvelope {
    /// GCM nonce（12 字节）。
    nonce: [u8; GCM_NONCE_LEN],
    /// 密文（不含 nonce、不含标签）。
    ciphertext: Vec<u8>,
    /// GCM 认证标签。
    tag: [u8; GCM_TAG_LEN],
    /// 容器 id 明文副本。
    container_id: String,
    /// 过期时间明文副本。
    expires_unix: u64,
}

/// 信封的固定前缀长度：`container_id_len`(4) + `key_len`(4) + `expires_unix`(8) + `tag_len`(4)。
const ENVELOPE_PREFIX_LEN: usize = 4 + 4 + 8 + 4;

impl KeyEnvelope {
    /// 容器 id。
    #[must_use]
    pub fn container_id(&self) -> &str {
        &self.container_id
    }

    /// `key` 段长度（GCM nonce + 密文）。
    #[must_use]
    pub fn key_len(&self) -> u32 {
        (GCM_NONCE_LEN + self.ciphertext.len()) as u32
    }

    /// 过期时间（Unix 秒）。
    #[must_use]
    pub fn expires_unix(&self) -> u64 {
        self.expires_unix
    }

    /// GCM 标签长度。
    #[must_use]
    pub fn tag_len(&self) -> u32 {
        self.tag.len() as u32
    }

    /// 密文（不含 nonce、不含标签）。
    #[must_use]
    pub fn ciphertext(&self) -> &[u8] {
        &self.ciphertext
    }

    /// 编码成线协议 §3 的线格式。
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let id = self.container_id.as_bytes();
        let mut out = Vec::with_capacity(
            ENVELOPE_PREFIX_LEN + id.len() + GCM_NONCE_LEN + self.ciphertext.len() + GCM_TAG_LEN,
        );
        out.extend_from_slice(&(id.len() as u32).to_be_bytes());
        out.extend_from_slice(id);
        out.extend_from_slice(&self.key_len().to_be_bytes());
        out.extend_from_slice(&self.nonce);
        out.extend_from_slice(&self.ciphertext);
        out.extend_from_slice(&self.expires_unix.to_be_bytes());
        out.extend_from_slice(&(self.tag.len() as u32).to_be_bytes());
        out.extend_from_slice(&self.tag);
        out
    }

    /// 解码线格式的密钥信封。
    ///
    /// ⛔ 任何长度不自洽、字段越界的输入一律失败（fail-closed），
    /// **不得**返回部分明文。
    pub fn decode(bytes: &[u8]) -> Result<Self, KeyError> {
        let mut reader = Reader::new(bytes);
        let id_len = reader.u32()? as usize;
        let container_id = reader.utf8(id_len)?;
        let key_len = reader.u32()? as usize;
        if key_len <= GCM_NONCE_LEN {
            return Err(KeyError::MalformedEnvelope);
        }
        // `key` 段 = GCM nonce || 密文。
        let key_section = reader.take(key_len)?;
        let expires_unix = reader.u64()?;
        let tag_len = reader.u32()? as usize;
        if tag_len != GCM_TAG_LEN {
            return Err(KeyError::MalformedEnvelope);
        }
        // ⛔ 信封必须**恰好**用完：多出的字节说明编码与解码不一致。
        let tag = reader.take(tag_len)?;
        if !reader.is_exhausted() {
            return Err(KeyError::MalformedEnvelope);
        }

        let mut nonce = [0u8; GCM_NONCE_LEN];
        nonce.copy_from_slice(&key_section[..GCM_NONCE_LEN]);
        let ciphertext = key_section[GCM_NONCE_LEN..].to_vec();
        let mut tag_bytes = [0u8; GCM_TAG_LEN];
        tag_bytes.copy_from_slice(tag);

        Ok(Self {
            nonce,
            ciphertext,
            tag: tag_bytes,
            container_id,
            expires_unix,
        })
    }
}

/// 顺序字节读取器（大端），越界一律失败。
struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Reader<'a> {
    /// 新建读取器。
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    /// 取 `len` 个字节（越界即失败）。
    fn take(&mut self, len: usize) -> Result<&'a [u8], KeyError> {
        let end = self
            .cursor
            .checked_add(len)
            .ok_or(KeyError::MalformedEnvelope)?;
        if end > self.bytes.len() {
            return Err(KeyError::MalformedEnvelope);
        }
        let slice = &self.bytes[self.cursor..end];
        self.cursor = end;
        Ok(slice)
    }

    /// 取一个 u32（大端）。
    fn u32(&mut self) -> Result<u32, KeyError> {
        let bytes = self.take(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// 取一个 u64（大端）。
    fn u64(&mut self) -> Result<u64, KeyError> {
        let bytes = self.take(8)?;
        let mut buf = [0u8; 8];
        buf.copy_from_slice(bytes);
        Ok(u64::from_be_bytes(buf))
    }

    /// 取 `len` 个字节并当作 UTF-8 字符串。
    fn utf8(&mut self, len: usize) -> Result<String, KeyError> {
        let bytes = self.take(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| KeyError::MalformedEnvelope)
    }

    /// 是否已读完。
    fn is_exhausted(&self) -> bool {
        self.cursor == self.bytes.len()
    }
}

/// 造一个密钥信封：用 `k_machine` 加密 `{session_key, container_id, expires_unix}`。
///
/// ⛔ 加密用的 GCM nonce 是**每次调用新生成**的 12 字节随机数
/// （GCM 规范要求同一密钥下 nonce 绝不重复）。
pub fn seal_envelope(
    k_machine: &[u8; K_MACHINE_LEN],
    plaintext: &EnvelopePlaintext,
) -> Result<KeyEnvelope, KeyError> {
    let mut nonce = [0u8; GCM_NONCE_LEN];
    getrandom::getrandom(&mut nonce).map_err(|_| KeyError::RandomUnavailable)?;

    // 明文布局：container_id_len + id + key_len(固定 32) + key + expires_unix。
    let id = plaintext.container_id.as_bytes();
    let mut body = Vec::with_capacity(4 + id.len() + 4 + SESSION_KEY_LEN + 8);
    body.extend_from_slice(&(id.len() as u32).to_be_bytes());
    body.extend_from_slice(id);
    body.extend_from_slice(&(SESSION_KEY_LEN as u32).to_be_bytes());
    body.extend_from_slice(&plaintext.session_key);
    body.extend_from_slice(&plaintext.expires_unix.to_be_bytes());

    let cipher = Aes256Gcm::new_from_slice(k_machine).map_err(|_| KeyError::InvalidKeyLength)?;
    // `encrypt` 产出 `密文 || 标签`（GCM 的后缀标签约定），这里把标签切出来独立成段。
    let sealed = cipher
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &body,
                aad: b"",
            },
        )
        .map_err(|_| KeyError::AuthenticationFailed)?;
    if sealed.len() <= GCM_TAG_LEN {
        return Err(KeyError::AuthenticationFailed);
    }
    let tag_bytes = sealed[sealed.len() - GCM_TAG_LEN..].to_vec();
    let ciphertext = sealed[..sealed.len() - GCM_TAG_LEN].to_vec();
    let mut tag = [0u8; GCM_TAG_LEN];
    tag.copy_from_slice(&tag_bytes);

    Ok(KeyEnvelope {
        nonce,
        ciphertext,
        tag,
        container_id: plaintext.container_id.clone(),
        expires_unix: plaintext.expires_unix,
    })
}

/// 拆开一个密钥信封：用 `k_machine` 解出 `{session_key, container_id, expires_unix}`。
///
/// ⛔ **失败即失败**：密钥不对 / 密文被改 / 标签被改，一律
/// [`KeyError::AuthenticationFailed`]，**绝不降级成「无密钥」**。
pub fn open_envelope(
    k_machine: &[u8; K_MACHINE_LEN],
    envelope: &KeyEnvelope,
) -> Result<EnvelopePlaintext, KeyError> {
    let cipher = Aes256Gcm::new_from_slice(k_machine).map_err(|_| KeyError::InvalidKeyLength)?;
    // 重组 `密文 || 标签` 交回 GCM（后缀标签约定）。
    let mut sealed = envelope.ciphertext.clone();
    sealed.extend_from_slice(&envelope.tag);
    let plain = cipher
        .decrypt(
            Nonce::from_slice(&envelope.nonce),
            Payload {
                msg: &sealed,
                aad: b"",
            },
        )
        .map_err(|_| KeyError::AuthenticationFailed)?;

    // 明文布局与 `seal_envelope` 对称：container_id_len + id + key_len + key + expires_unix。
    let mut reader = Reader::new(&plain);
    let id_len = reader.u32()? as usize;
    let container_id = reader.utf8(id_len)?;
    let key_len = reader.u32()? as usize;
    if key_len != SESSION_KEY_LEN {
        return Err(KeyError::MalformedEnvelope);
    }
    let key_bytes = reader.take(SESSION_KEY_LEN)?;
    let mut session_key = [0u8; SESSION_KEY_LEN];
    session_key.copy_from_slice(key_bytes);
    let expires_unix = reader.u64()?;

    // 明文里的 container_id / expires_unix 与信封外层的明文副本必须一致：
    // 外层副本被改过 ⇒ 认证失败（否则会变成一条可篡改的路由信息）。
    if container_id != envelope.container_id || expires_unix != envelope.expires_unix {
        return Err(KeyError::AuthenticationFailed);
    }

    Ok(EnvelopePlaintext {
        container_id,
        session_key,
        expires_unix,
    })
}
