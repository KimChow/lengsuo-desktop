//! 指纹值治理的**本地缓存**：每环境一份、**加密落盘**、**原子写**。
//!
//! # 三条硬要求
//!
//! 1. **按环境分目录**：⛔ 不许所有环境共用一个文件
//!    （`root/<env_id>/values.bin`）。
//! 2. **加密落盘**：用仓库里已有的加密组件（[`aes_gcm`] + 复用
//!    [`crate::kernel_pipe::keys::KeyEnvelope`] 的容器布局），
//!    ⛔ 不新写加密实现。
//! 3. **原子写**：写临时文件 + `rename` ⇒ 进程被杀不会留下半截缓存。
//!
//! # 损坏即丢弃
//!
//! 缓存解不开 / 解析不出来 ⇒ **丢弃并重新拉取**，⛔ 不许让浏览器起不来。
//! ⇒ [`FpValueCache::load`] **永远不返回错误**：坏掉就是 `None`（= 未命中）。
//!
//! # 密文里不许有明文凭据
//!
//! 整个缓存条目（含 `sign` 与目标哈希）都在密文里；文件字节里**搜不到**
//! 任何明文目标值。

use std::path::PathBuf;

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use hkdf::Hkdf;
use sha2::Sha256;

use crate::kernel_pipe::fp_values::{FpValueCacheEntry, FpValueStats};
use crate::kernel_pipe::keys::{KeyEnvelope, GCM_NONCE_LEN, GCM_TAG_LEN, K_MACHINE_LEN};

/// 缓存密钥派生用的 `salt`（非空 —— `hkdf` 的 `salt` 位置必须给值）。
const CACHE_KEY_SALT: &[u8] = b"lengsuo-fp-values-cache-salt-v1";

/// 缓存密钥派生用的 `info` 串。
const CACHE_KEY_INFO: &[u8] = b"lengsuo-fp-values-cache-key-v1";

/// 缓存密文的附加认证数据（AAD）。
///
/// ⛔ 绑定用途 ⇒ 缓存密文不能被当成别的载荷来用。
const CACHE_AAD: &[u8] = b"lengsuo-fp-values-cache-v1";

/// 缓存密钥长度（字节）。
pub const CACHE_KEY_LEN: usize = 32;

/// 缓存操作的失败原因。
///
/// ⛔ 错误文本**不含**任何缓存内容（密文 / 明文目标值都不进错误文本）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FpValueCacheError {
    /// 加密 / 解密失败（密钥不对或随机源失败）。
    #[error("值缓存加解密失败")]
    Crypto,
    /// 缓存文件损坏（容器布局不合法或认证不通过）。
    #[error("值缓存损坏")]
    Corrupt,
}

/// 指纹值缓存（**一个根目录**下按环境分文件）。
#[derive(Debug, Clone)]
pub struct FpValueCache {
    root: PathBuf,
    key: [u8; CACHE_KEY_LEN],
}

impl FpValueCache {
    /// 用一个根目录与一把缓存密钥构造。
    #[must_use]
    pub fn new(root: impl Into<PathBuf>, key: [u8; CACHE_KEY_LEN]) -> Self {
        Self {
            root: root.into(),
            key,
        }
    }

    /// 从机器码派生缓存密钥。
    ///
    /// `HKDF-SHA256(ikm=k_machine, salt=CACHE_KEY_SALT, info=CACHE_KEY_INFO, L=32)`。
    ///
    /// ⛔ 缓存密钥**只在本机有效** ⇒ 缓存文件拷到别的机器解不开（这是
    /// 好事：缓存里含会话凭证）。
    #[must_use]
    pub fn derive_key(k_machine: &[u8; K_MACHINE_LEN]) -> [u8; CACHE_KEY_LEN] {
        let hk = Hkdf::<Sha256>::new(Some(CACHE_KEY_SALT), k_machine);
        let mut okm = [0u8; CACHE_KEY_LEN];
        hk.expand(CACHE_KEY_INFO, &mut okm)
            .expect("HKDF 展开 32 字节必然可行");
        okm
    }

    /// 这个环境的目录（`root/<env_id>/`）。
    #[must_use]
    pub fn env_dir(&self, env_id: &str) -> Option<PathBuf> {
        Some(self.root.join(sanitize_env_id(env_id)?))
    }

    /// 这个环境的缓存文件路径（`root/<env_id>/values.bin`）。
    #[must_use]
    pub fn cache_path(&self, env_id: &str) -> Option<PathBuf> {
        Some(self.env_dir(env_id)?.join("values.bin"))
    }

    /// 读缓存。
    ///
    /// - `Some(entry)` = **命中**。
    /// - `None` = **未命中**（没有文件 / 解不开 / 结构不合法）⇒ 重新拉取。
    ///
    /// ⛔ **永远不失败**：坏掉的缓存不许让浏览器起不来。
    pub fn load(&self, env_id: &str, stats: &mut FpValueStats) -> Option<FpValueCacheEntry> {
        let Some(path) = self.cache_path(env_id) else {
            stats.cache_misses += 1;
            return None;
        };
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => {
                // 没有文件 = 未命中（不是错误）。
                stats.cache_misses += 1;
                return None;
            }
        };
        let plain = match open_blob(&self.key, &bytes) {
            Ok(plain) => plain,
            Err(_) => {
                // ⛔ 损坏 ⇒ 丢弃（计数）⇒ 下次重新拉取。
                stats.cache_corrupt += 1;
                stats.cache_misses += 1;
                return None;
            }
        };
        let text = match String::from_utf8(plain) {
            Ok(text) => text,
            Err(_) => {
                stats.cache_corrupt += 1;
                stats.cache_misses += 1;
                return None;
            }
        };
        match FpValueCacheEntry::parse_json(&text) {
            Ok(entry) => {
                stats.cache_hits += 1;
                Some(entry)
            }
            Err(_) => {
                stats.cache_corrupt += 1;
                stats.cache_misses += 1;
                None
            }
        }
    }

    /// 写缓存（**加密 + 原子**）。
    ///
    /// 返回 `false` 表示写失败（调用方只记日志 + 计数，⛔ 不阻塞容器关闭）。
    pub fn store(&self, env_id: &str, entry: &FpValueCacheEntry) -> bool {
        let Some(path) = self.cache_path(env_id) else {
            return false;
        };
        let Some(dir) = path.parent() else {
            return false;
        };
        if std::fs::create_dir_all(dir).is_err() {
            return false;
        }
        let blob = match seal_blob(&self.key, &entry.to_json().into_bytes()) {
            Ok(blob) => blob,
            Err(_) => return false,
        };
        // ⭐ 原子写：先写临时文件，再 rename（同目录 ⇒ 同一文件系统）。
        let tmp = path.with_extension("bin.tmp");
        if std::fs::write(&tmp, &blob).is_err() {
            let _ = std::fs::remove_file(&tmp);
            return false;
        }
        if std::fs::rename(&tmp, &path).is_err() {
            let _ = std::fs::remove_file(&tmp);
            return false;
        }
        true
    }
}

/// 把环境标识变成一个**安全的目录名**。
///
/// ⛔ 环境标识来自远端配置 ⇒ 不能直接拼进路径（路径穿越）。
/// 只保留 `[A-Za-z0-9._-]`，其余换成 `_`；`.` / `..` / 空 ⇒ `None`。
#[must_use]
pub fn sanitize_env_id(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() || cleaned == "." || cleaned == ".." {
        return None;
    }
    Some(cleaned)
}

/// 加密一段明文（容器布局复用 [`KeyEnvelope`]）。
fn seal_blob(key: &[u8; CACHE_KEY_LEN], plaintext: &[u8]) -> Result<Vec<u8>, FpValueCacheError> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| FpValueCacheError::Crypto)?;
    let mut nonce_bytes = [0u8; GCM_NONCE_LEN];
    getrandom::getrandom(&mut nonce_bytes).map_err(|_| FpValueCacheError::Crypto)?;
    let sealed = cipher
        .encrypt(
            Nonce::from_slice(&nonce_bytes),
            Payload {
                msg: plaintext,
                aad: CACHE_AAD,
            },
        )
        .map_err(|_| FpValueCacheError::Crypto)?;
    if sealed.len() < GCM_TAG_LEN {
        return Err(FpValueCacheError::Crypto);
    }
    let (ciphertext, tag_bytes) = sealed.split_at(sealed.len() - GCM_TAG_LEN);
    let mut tag = [0u8; GCM_TAG_LEN];
    tag.copy_from_slice(tag_bytes);
    Ok(KeyEnvelope {
        nonce: nonce_bytes,
        ciphertext: ciphertext.to_vec(),
        tag,
    }
    .encode())
}

/// 解开一段密文。
///
/// 容器布局不合法 **或** 认证不通过 ⇒ [`FpValueCacheError::Corrupt`]
/// （调用方据此丢弃缓存）。
fn open_blob(key: &[u8; CACHE_KEY_LEN], blob: &[u8]) -> Result<Vec<u8>, FpValueCacheError> {
    let envelope = KeyEnvelope::decode(blob).map_err(|_| FpValueCacheError::Corrupt)?;
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| FpValueCacheError::Crypto)?;
    let mut sealed = Vec::with_capacity(envelope.ciphertext.len() + GCM_TAG_LEN);
    sealed.extend_from_slice(&envelope.ciphertext);
    sealed.extend_from_slice(&envelope.tag);
    cipher
        .decrypt(
            Nonce::from_slice(&envelope.nonce),
            Payload {
                msg: &sealed,
                aad: CACHE_AAD,
            },
        )
        .map_err(|_| FpValueCacheError::Corrupt)
}
