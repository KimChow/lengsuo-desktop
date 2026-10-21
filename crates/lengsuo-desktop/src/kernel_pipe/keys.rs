//! 密钥材料：机器码派生、会话密钥、密钥信封（KeyEnvelope）。
//!
//! # 线协议
//!
//! 全部对齐跨仓契约 `kernel-wire-contract.md` §1 / §3.1。
//!
//! # ⛔ 安全边界
//!
//! - **机器码**只在本机采集真实值，**不复制任何一方的常量字符串**。
//! - **会话密钥只存内存**：⛔ 不写盘、不进环境变量、不进命令行、不进日志。
//! - **错误信息不含密钥或信封内容**（见 [`EnvelopeError`]）。
//! - 密钥材料离开作用域时清零（[`zeroize`]）。
//!
//! # ⭐ 会话密钥的生成方式是「可注入」的
//!
//! 线协议 §1 把「CSPRNG 随机」与「HKDF 派生」列为**待主理人拍板**的候选。
//! 本模块**不替主理人选**：只提供派生函数 [`derive_session_key`]（候选 B）
//! 与随机生成 [`random_session_key`]（候选 A），**由调用方注入**。
//! 这不影响线上字节——信封总是携带 `session_key`，接收方用信封里解出来的值。

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use hkdf::Hkdf;
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use crate::kernel_pipe::FrameError;

/// 会话密钥长度（字节）。
pub const SESSION_KEY_LEN: usize = 32;

/// GCM nonce 长度（字节）。
///
/// ⛔ **12 字节**——GCM 标准 nonce 长度，也是 `aes-gcm` crate 唯一接受的。
/// 线协议 §3.1 明令「⛔ 不是 8 字节」。
pub const GCM_NONCE_LEN: usize = 12;

/// GCM 认证标签长度（字节）。
pub const GCM_TAG_LEN: usize = 16;

/// Hello 帧里的 nonce 长度（字节）。
///
/// ⚠️ 这个 8 字节 nonce 与 GCM 的 12 字节 nonce 是**两回事**：
/// 前者进 AAD，后者进加密。线协议 §3.1 要求 AAD = 帧头 8 字节 + 这 8 字节。
pub const HELLO_NONCE_LEN: usize = 8;

/// HKDF 的 `info` 串（线协议 §1）。
pub const SESSION_KEY_INFO: &[u8] = b"lengsuo-kernel-session-v1";

/// `k_machine` 长度（字节）——SHA-256 输出。
pub const K_MACHINE_LEN: usize = 32;

/// 机器码来源的四个字段（线协议 §1）。
///
/// ⚠️ **形状**（同样的四个字段、同样的键序）与线协议一致，
/// 但**取值必须是本机真实值**。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineInputs {
    /// 操作系统标识（如 `linux` / `windows`）。
    pub os: String,
    /// CPU 标识。
    pub cpuid: String,
    /// 磁盘标识。
    pub hddid: String,
    /// 计算机名。
    pub computername: String,
}

impl MachineInputs {
    /// 按线协议 §1 的**固定 JSON 键序**序列化。
    ///
    /// ⭐ 键序是协议的一部分：`os` → `cpuid` → `hddid` → `computername`。
    /// 顺序变了 ⇒ 摘要变了 ⇒ 两侧对不上。
    /// 紧凑写法（无空格、无转义冗余），与线协议给的夹具逐字节一致。
    #[must_use]
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"os":"{}","cpuid":"{}","hddid":"{}","computername":"{}"}}"#,
            json_escape(&self.os),
            json_escape(&self.cpuid),
            json_escape(&self.hddid),
            json_escape(&self.computername),
        )
    }
}

/// 极简 JSON 字符串转义（只处理控制字符与引号反斜杠）。
///
/// 机器码字段是本机采集的标识串，正常不含控制字符；
/// 但采集源不可控 ⇒ 转义一次，免得拼出非法 JSON。
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for ch in s.chars() {
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
    out
}

/// 机器码 = `SHA-256(UTF-16LE(固定键序 JSON))` 的**小写十六进制**。
///
/// ⭐ 编码是 **UTF-16LE 且无 BOM**（线协议 §1 + §2 固定向量）。
#[must_use]
pub fn machine_code(inputs: &MachineInputs) -> String {
    let json = inputs.to_json();
    let utf16 = json
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<u8>>();
    let digest = Sha256::digest(&utf16);
    to_hex(&digest)
}

/// `k_machine = SHA-256(machine_code 的 ASCII 字节)`。
///
/// ⚠️ 是 **ASCII 字节**（即十六进制字符串本身），不是摘要的原始字节。
#[must_use]
pub fn k_machine(machine_code: &str) -> [u8; K_MACHINE_LEN] {
    let digest = Sha256::digest(machine_code.as_bytes());
    let mut out = [0u8; K_MACHINE_LEN];
    out.copy_from_slice(&digest);
    out
}

/// 小写十六进制编码。
#[must_use]
pub fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// 密钥信封（KeyEnvelope）的载荷布局错误。
///
/// ⛔ 错误文本**不得**包含信封内容或密钥——只说「哪一步失败」。
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EnvelopeError {
    /// 载荷比声明的长度短。
    #[error("信封载荷被截断")]
    Truncated,

    /// 载荷里有字段长度不自洽（`len` 字段与实际剩余字节对不上）。
    #[error("信封字段长度不自洽")]
    InconsistentLength,

    /// `nonce_len` 不是 12。
    #[error("GCM nonce 长度非法（应为 12）")]
    BadNonceLength,

    /// `tag_len` 不是 16。
    #[error("GCM 认证标签长度非法（应为 16）")]
    BadTagLength,

    /// 解密失败：密钥不对、密文被改、或 AAD 不匹配。
    ///
    /// 三者在密码学上不可区分，**故意不细分**——细分会泄露信息。
    #[error("信封解密失败（认证不通过）")]
    DecryptFailed,

    /// 明文长度不对（解出来了但不是预期的 `session_key || container_id || expires`）。
    #[error("信封明文长度非法")]
    BadPlaintext,

    /// 随机源失败。
    #[error("取随机数失败")]
    Random,
}

/// 密钥信封的**明文**部分（线协议 §3.1）。
///
/// 明文 = `session_key(32B) || container_id || expires_unix(u64 大端)`。
///
/// ⛔ **`Debug` 是手写的**：会话密钥**不打印**（只打印长度）。
/// 派生 `Debug` 会把 32 字节密钥原样写进日志 ⇒ 违反「⛔ 密钥不进日志」。
#[derive(Clone, PartialEq, Eq)]
pub struct EnvelopePlain {
    /// 会话密钥（32 字节）。
    pub session_key: [u8; SESSION_KEY_LEN],
    /// 容器 id（UTF-8 字节）。
    pub container_id: Vec<u8>,
    /// 过期时间（Unix 秒）。
    pub expires_unix: u64,
}

impl std::fmt::Debug for EnvelopePlain {
    /// ⛔ **不打印会话密钥**，也不打印容器 id ——只打印形状。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnvelopePlain")
            .field("session_key", &"<redacted>")
            .field("container_id_len", &self.container_id.len())
            .field("expires_unix", &self.expires_unix)
            .finish()
    }
}

/// 密钥信封的**载荷**（线协议 §3.1 逐字段布局）。
///
/// ```text
/// 偏移   长度    字段
/// 0      4       nonce_len      u32 大端，固定 12
/// 4      12      nonce          GCM nonce（每次信封独立随机）
/// 16     4       ciphertext_len u32 大端
/// 20     M       ciphertext     GCM 密文
/// 20+M   4       tag_len        u32 大端，固定 16
/// 24+M   16      tag            GCM 认证标签
/// ```
///
/// ⛔ **载荷的明文区只有 6 个字段**：`container_id` 与 `expires_unix`
/// **只出现在密文里面**，接收方解密后从明文取。
/// ⛔ **不要把它们再抄一份到载荷的明文头部**——AAD 只盖住帧头 8 字节 +
/// Hello nonce，**明文区的任何字节都不在认证范围内** ⇒ 抄一份到明文区 =
/// 容器标识与过期时间可被中间人改写而认证照样通过（过期时间可被无限延长）。
///
/// **所有整数一律大端。**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEnvelope {
    /// GCM nonce（12 字节）。
    pub nonce: [u8; GCM_NONCE_LEN],
    /// GCM 密文（明文 = `session_key(32B) || container_id || expires_unix`）。
    pub ciphertext: Vec<u8>,
    /// GCM 认证标签（16 字节）。
    pub tag: [u8; GCM_TAG_LEN],
}

impl KeyEnvelope {
    /// 按线协议 §3.1 的布局编码成载荷字节。
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(ENVELOPE_FIXED_LEN + self.ciphertext.len());
        out.extend_from_slice(&(self.nonce.len() as u32).to_be_bytes());
        out.extend_from_slice(&self.nonce);
        out.extend_from_slice(&(self.ciphertext.len() as u32).to_be_bytes());
        out.extend_from_slice(&self.ciphertext);
        out.extend_from_slice(&(self.tag.len() as u32).to_be_bytes());
        out.extend_from_slice(&self.tag);
        out
    }

    /// 从载荷字节解码。
    ///
    /// 任何长度不自洽都失败，**不返回部分明文**。
    pub fn decode(payload: &[u8]) -> Result<Self, EnvelopeError> {
        let mut cursor = Cursor::new(payload);

        let nonce_len = cursor.take_u32()? as usize;
        if nonce_len != GCM_NONCE_LEN {
            return Err(EnvelopeError::BadNonceLength);
        }
        let nonce_bytes = cursor.take(nonce_len)?;
        let mut nonce = [0u8; GCM_NONCE_LEN];
        nonce.copy_from_slice(nonce_bytes);

        let ciphertext_len = cursor.take_u32()? as usize;
        let ciphertext = cursor.take(ciphertext_len)?.to_vec();

        let tag_len = cursor.take_u32()? as usize;
        if tag_len != GCM_TAG_LEN {
            return Err(EnvelopeError::BadTagLength);
        }
        let tag_bytes = cursor.take(tag_len)?;
        let mut tag = [0u8; GCM_TAG_LEN];
        tag.copy_from_slice(tag_bytes);

        // 载荷尾部有多余字节 ⇒ 布局不自洽（否则两侧会各读各的）。
        if !cursor.is_exhausted() {
            return Err(EnvelopeError::InconsistentLength);
        }

        Ok(Self {
            nonce,
            ciphertext,
            tag,
        })
    }
}

/// 密钥信封载荷里**与密文长度无关**的那部分字节数：
/// `nonce_len(4) + nonce(12) + ciphertext_len(4) + tag_len(4) + tag(16)` = **40**。
pub const ENVELOPE_FIXED_LEN: usize = 4 + GCM_NONCE_LEN + 4 + 4 + GCM_TAG_LEN;

/// 密钥信封**载荷**的字节数（给定 `container_id` 长度）。
///
/// 载荷布局（线协议 §3.1）：
///
/// ```text
/// u32 nonce_len(4) | nonce(12) | u32 ciphertext_len(4) | ciphertext(M)
/// | u32 tag_len(4) | tag(16)
/// ```
///
/// 密文长度 M == 明文长度 == `32 + N + 8`（GCM 是流模式，N = container_id 长度）。
/// ⇒ 载荷总长 = **40 + M** = `80 + N`。
/// （契约 §3.1 的固定向量：N = 5 ⇒ M = 45 ⇒ 载荷 85 字节 ⇒ 帧头 `len` = `0x55`。）
///
/// ⭐ 这个长度会进 **AAD**（帧头的 `len` 字段），
/// 所以发送方与接收方必须算出**同一个值**——否则认证必失败。
#[must_use]
pub fn envelope_payload_len(container_id_len: usize) -> usize {
    const PLAINTEXT_OVERHEAD: usize = SESSION_KEY_LEN + 8;
    ENVELOPE_FIXED_LEN + (PLAINTEXT_OVERHEAD + container_id_len)
}

/// 载荷游标：只做「取固定长度 / 取 u32 / 取 u64」，越界即失败。
struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    const fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], EnvelopeError> {
        let end = self.pos.checked_add(n).ok_or(EnvelopeError::Truncated)?;
        if end > self.data.len() {
            return Err(EnvelopeError::Truncated);
        }
        let out = &self.data[self.pos..end];
        self.pos = end;
        Ok(out)
    }

    fn take_u32(&mut self) -> Result<u32, EnvelopeError> {
        let bytes = self.take(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn is_exhausted(&self) -> bool {
        self.pos == self.data.len()
    }
}

/// 封装（加密）一个密钥信封。
///
/// - `k_machine`：机器码派生密钥（**不是**会话密钥——会话密钥此刻还没解出来）。
/// - `aad`：附加认证数据。线协议要求 = 帧头 8 字节 + Hello 的 8 字节 nonce，
///   共 16 字节。⚠️ 少给 hello nonce ⇒ 信封可被重放到另一次握手。
/// - `plain`：要封进去的明文（会话密钥 + 容器 id + 过期时间）。
/// - `gcm_nonce`：12 字节 GCM nonce，**每次信封独立随机**。
pub fn seal_envelope(
    k_machine: &[u8; K_MACHINE_LEN],
    aad: &[u8],
    plain: &EnvelopePlain,
    gcm_nonce: [u8; GCM_NONCE_LEN],
) -> Result<KeyEnvelope, EnvelopeError> {
    let cipher = Aes256Gcm::new_from_slice(k_machine).map_err(|_| EnvelopeError::DecryptFailed)?;

    let mut plaintext = Vec::with_capacity(SESSION_KEY_LEN + plain.container_id.len() + 8);
    plaintext.extend_from_slice(&plain.session_key);
    plaintext.extend_from_slice(&plain.container_id);
    plaintext.extend_from_slice(&plain.expires_unix.to_be_bytes());

    let nonce = Nonce::from_slice(&gcm_nonce);
    let sealed = cipher
        .encrypt(
            nonce,
            Payload {
                msg: &plaintext,
                aad,
            },
        )
        .map_err(|_| EnvelopeError::DecryptFailed)?;

    // aes-gcm 0.10 把 tag 拼在密文尾部，这里拆开。
    if sealed.len() < GCM_TAG_LEN {
        return Err(EnvelopeError::DecryptFailed);
    }
    let (ciphertext, tag_bytes) = sealed.split_at(sealed.len() - GCM_TAG_LEN);
    let mut tag = [0u8; GCM_TAG_LEN];
    tag.copy_from_slice(tag_bytes);

    let envelope = KeyEnvelope {
        nonce: gcm_nonce,
        ciphertext: ciphertext.to_vec(),
        tag,
    };

    // 明文含会话密钥 ⇒ 用完立刻清零。
    plaintext.zeroize();

    Ok(envelope)
}

/// 解封一个密钥信封。
///
/// ⛔ **失败绝不降级**：密钥不对、密文被改 1 字节、AAD 不对、载荷被截断——
/// 全部返回 `Err`，**不返回任何部分明文**。
///
/// ⭐ `container_id` 与 `expires_unix` **从明文里取**——
/// 载荷的明文区**没有**这两个字段（线协议 §3.1），
/// 所以它们**在认证范围内**（明文被 GCM 盖住）。
pub fn open_envelope(
    k_machine: &[u8; K_MACHINE_LEN],
    aad: &[u8],
    envelope: &KeyEnvelope,
) -> Result<EnvelopePlain, EnvelopeError> {
    let cipher = Aes256Gcm::new_from_slice(k_machine).map_err(|_| EnvelopeError::DecryptFailed)?;

    let nonce = Nonce::from_slice(&envelope.nonce);

    let mut sealed = Vec::with_capacity(envelope.ciphertext.len() + GCM_TAG_LEN);
    sealed.extend_from_slice(&envelope.ciphertext);
    sealed.extend_from_slice(&envelope.tag);

    let mut plaintext = cipher
        .decrypt(nonce, Payload { msg: &sealed, aad })
        .map_err(|_| EnvelopeError::DecryptFailed)?;

    // 明文 = session_key(32B) || container_id || expires_unix(u64 大端)。
    // container_id 的长度**不在载荷里** ⇒ 从尾部倒推：
    // 末尾 8 字节是 expires_unix，其余（去掉 32 字节会话密钥）就是 container_id。
    let result = split_plaintext(&plaintext);
    plaintext.zeroize();
    result
}

/// 拆明文：`session_key(32B) || container_id || expires_unix(u64 大端)`。
///
/// ⭐ `container_id` 的长度由明文长度**倒推**（末尾 8 字节固定是 `expires_unix`），
/// 因为载荷里**没有** `container_id_len` 字段（线协议 §3.1）。
fn split_plaintext(plaintext: &[u8]) -> Result<EnvelopePlain, EnvelopeError> {
    if plaintext.len() < SESSION_KEY_LEN + 8 {
        return Err(EnvelopeError::BadPlaintext);
    }

    let mut session_key = [0u8; SESSION_KEY_LEN];
    session_key.copy_from_slice(&plaintext[..SESSION_KEY_LEN]);

    // 末尾 8 字节 = expires_unix（大端）。
    let container_end = plaintext.len() - 8;
    let container_id = plaintext[SESSION_KEY_LEN..container_end].to_vec();

    let mut exp_bytes = [0u8; 8];
    exp_bytes.copy_from_slice(&plaintext[container_end..]);

    Ok(EnvelopePlain {
        session_key,
        container_id,
        expires_unix: u64::from_be_bytes(exp_bytes),
    })
}

/// 会话密钥生成方式（线协议 §1 待主理人拍板的两个候选）。
///
/// 本模块**不替主理人选**——由调用方注入，线协议字节格式不受影响。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionKeySource {
    /// 候选 A：CSPRNG 随机生成 32 字节（`env-channel-design.md` §2.4）。
    Random,
    /// 候选 B：`HKDF-SHA256(ikm=k_machine, salt=hello_nonce, info=...)`。
    ///
    /// ⚠️ 参数位置必须与线协议一致：`ikm` = `k_machine`，`salt` = nonce（非空），
    /// `info` = `lengsuo-kernel-session-v1`，`L` = 32。
    Hkdf,
}

/// 候选 A：CSPRNG 随机生成 32 字节会话密钥。
pub fn random_session_key() -> Result<[u8; SESSION_KEY_LEN], EnvelopeError> {
    let mut key = [0u8; SESSION_KEY_LEN];
    getrandom::getrandom(&mut key).map_err(|_| EnvelopeError::Random)?;
    Ok(key)
}

/// 候选 B：HKDF-SHA256 派生会话密钥。
///
/// `salt` = Hello 的 8 字节 nonce，`info` = [`SESSION_KEY_INFO`]。
pub fn derive_session_key(
    k_machine: &[u8; K_MACHINE_LEN],
    hello_nonce: &[u8; HELLO_NONCE_LEN],
) -> Result<[u8; SESSION_KEY_LEN], EnvelopeError> {
    let hk = Hkdf::<Sha256>::new(Some(hello_nonce), k_machine);
    let mut okm = [0u8; SESSION_KEY_LEN];
    hk.expand(SESSION_KEY_INFO, &mut okm)
        .map_err(|_| EnvelopeError::BadPlaintext)?;
    Ok(okm)
}

/// 按指定方式产生会话密钥（供调用方注入选择）。
pub fn make_session_key(
    source: SessionKeySource,
    k_machine: &[u8; K_MACHINE_LEN],
    hello_nonce: &[u8; HELLO_NONCE_LEN],
) -> Result<[u8; SESSION_KEY_LEN], EnvelopeError> {
    match source {
        SessionKeySource::Random => random_session_key(),
        SessionKeySource::Hkdf => derive_session_key(k_machine, hello_nonce),
    }
}

/// 帧错误 → 信封错误的桥接（解信封失败时说明是哪一步）。
///
/// 存在的意义：让调用方能区分「帧坏了」与「信封坏了」，
/// 而不必把帧错误原文（可能带偏移信息）当成认证失败。
#[must_use]
pub fn envelope_error_from_frame(err: FrameError) -> EnvelopeError {
    match err {
        FrameError::TruncatedHeader { .. } | FrameError::TruncatedPayload { .. } => {
            EnvelopeError::Truncated
        }
        FrameError::PayloadTooLarge { .. }
        | FrameError::ReservedNotZero
        | FrameError::UnknownType { .. } => EnvelopeError::InconsistentLength,
    }
}

#[cfg(test)]
mod key_tests {
    use super::*;
    use crate::kernel_pipe::tests::{
        fixed_k_machine, HELLO_NONCE, K_MACHINE_HEX, MACHINE_CODE, MACHINE_JSON, SESSION_KEY_HEX,
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
            make_session_key(SessionKeySource::Random, &fixed_k_machine(), &HELLO_NONCE)
                .expect("应可生成");
        assert_eq!(via_random.len(), SESSION_KEY_LEN);
        let via_hkdf = make_session_key(SessionKeySource::Hkdf, &fixed_k_machine(), &HELLO_NONCE)
            .expect("应可派生");
        assert_eq!(via_hkdf, derived);
    }
}
