//! profile 包的**清单层与写出层**：契约 §4.5 的清单 ⇄ zip 归档。
//!
//! # 这一层做什么
//!
//! - **读清单**：内核侧产出的清单 JSON（`{"files":[{"path","size","kind"}]}`）
//!   按契约 §4.5 的形状解析；
//! - **算清单**：扫 profile 根目录，按**配置里的规则**算出一份同形状的清单
//!   （路径与排序规则与契约 §4.5 逐字节一致）；
//! - **写归档**：把清单里的文件流式写进一个 zip（deflate(8)）。
//!
//! 上层的编排（规则从哪来、传到哪里、失败怎么办）在
//! [`crate::kernel_pipe::upload`]。
//!
//! # ⛔ 流式
//!
//! 归档**整个写进目标文件**（`File` 是 `Write + Seek`），源文件按 64 KiB
//! 块读进 zip ⇒ **整包从不进内存**。`zip::ZipWriter` 直接写进 `File`，
//! 不用 `Cursor<Vec<u8>>`。
//!
//! # ⛔ zip slip 防护
//!
//! 条目名只能是 `/` 分隔的**相对路径**：不带前导 `/`、不带盘符、不带
//! `..`、不含空段、结尾不带 `/`、不含 `\`。
//! 解压端会按这个路径写文件 ⇒ 一个 `../` 就能写到它不该写的地方。
//!
//! # ⛔ 符号链接
//!
//! 清单里**只收普通文件**：符号链接、目录、管道一律跳过。profile 目录里
//! 一个指向别处的链接会把 profile 外面的东西带进包里。

use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};

use crate::kernel_pipe::profile_rules::{never_goes_out, ProfileSyncRules};

#[cfg(test)]
#[path = "zip_writer_tests.rs"]
mod zip_writer_tests;

/// zip 层的失败原因。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZipError {
    /// 条目名不安全（绝对路径 / 盘符 / 含 `..` / 空段 / 结尾 `/` / 含 `\`）。
    UnsafeEntryName,
    /// 清单畸形（形状不对契约 §4.5）。
    MalformedManifest,
    /// 打开源文件失败。
    Io(String),
    /// 写归档失败。
    Write(String),
}

impl std::fmt::Display for ZipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsafeEntryName => write!(f, "条目名不安全"),
            Self::MalformedManifest => write!(f, "清单畸形"),
            Self::Io(what) => write!(f, "读文件失败：{what}"),
            Self::Write(what) => write!(f, "写归档失败：{what}"),
        }
    }
}

impl std::error::Error for ZipError {}

// ---------------------------------------------------------------------------
// 清单（契约 §4.5）
// ---------------------------------------------------------------------------

/// 清单里的一个文件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestFile {
    /// 相对 profile 根的路径，`/` 分隔（契约 §4.5）。
    pub path: String,
    /// 字节数。
    pub size: u64,
}

/// 一份清单（契约 §4.5）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProfileManifest {
    /// 收集到的文件，按路径**字节序**升序。
    pub files: Vec<ManifestFile>,
    /// 收集到的总字节数。
    pub total_bytes: u64,
}

impl ProfileManifest {
    /// 读内核发来的清单 JSON（契约 §4.5）。
    ///
    /// 形状：`{"files":[{"path":"…","size":1024,"kind":"file"}]}`。
    ///
    /// - 顶层**恰好一个键** `files`；空清单是 `[]`（⛔ 不是 `null`）。
    /// - `kind` 目前只有 `"file"`（契约 §4.5）⇒ 别的值**拒绝**。
    /// - 路径按契约 §4.5 校验：相对、无 `..`、无空段、无结尾 `/`。
    /// - 顺序**重新按字节序排**：契约要求内核排好，但客户端再排一次是
    ///   幂等的，且一份乱序的清单也能按契约的顺序打包。
    ///
    /// ⛔ 清单里不许有密钥、会话密钥、真实用户数据（只许路径与大小）——
    /// 形状本身只容得下路径与大小，这是**形状**给的保证。
    pub fn parse(manifest_json: &str) -> Result<Self, ZipError> {
        let value: serde_json::Value =
            serde_json::from_str(manifest_json).map_err(|_| ZipError::MalformedManifest)?;
        let object = value.as_object().ok_or(ZipError::MalformedManifest)?;
        if object.len() != 1 {
            // 契约 §4.5：顶层恰好一个键 `files`。
            return Err(ZipError::MalformedManifest);
        }
        let array = object
            .get("files")
            .and_then(serde_json::Value::as_array)
            .ok_or(ZipError::MalformedManifest)?;
        let mut manifest = ProfileManifest::default();
        for item in array {
            let item = item.as_object().ok_or(ZipError::MalformedManifest)?;
            // ⛔ 契约 §4.5：条目**恰好三个字段**（`path` / `size` / `kind`）。
            // 多一个字段就拒绝 —— 清单是远端输入，多出来的字段是塞数据
            // （密钥、真实用户数据、`data_url`）的入口。形状不给它位置。
            if item.len() != 3 {
                return Err(ZipError::MalformedManifest);
            }
            let path = item
                .get("path")
                .and_then(serde_json::Value::as_str)
                .ok_or(ZipError::MalformedManifest)?;
            let size = item
                .get("size")
                .and_then(serde_json::Value::as_u64)
                .ok_or(ZipError::MalformedManifest)?;
            let kind = item
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .ok_or(ZipError::MalformedManifest)?;
            if kind != "file" {
                return Err(ZipError::MalformedManifest);
            }
            check_relative_path(path)?;
            manifest.files.push(ManifestFile {
                path: path.to_string(),
                size,
            });
            manifest.total_bytes = manifest.total_bytes.saturating_add(size);
        }
        manifest
            .files
            .sort_by(|left, right| compare_paths(&left.path, &right.path));
        Ok(manifest)
    }
}

/// 按字节比较两个路径（无符号字节升序）。
///
/// 与内核侧 `profile_bundle.cc` 的 `ComparePaths` 是同一套比较。
fn compare_paths(left: &str, right: &str) -> std::cmp::Ordering {
    left.as_bytes().cmp(right.as_bytes())
}

/// 校验一个相对路径（契约 §4.5 的路径规则）。
///
/// ⛔ 不带前导 `/`、不带盘符、不带 `./`、不含 `..` 段、不含连续 `//`、
/// 不含空段、文件路径不带结尾 `/`。
/// ⛔ **分隔符一律 `/`**：Windows 上也不许出现 `\`（契约 §4.5）。
fn check_relative_path(path: &str) -> Result<(), ZipError> {
    if path.is_empty() || path.starts_with('/') || path.starts_with('\\') {
        return Err(ZipError::UnsafeEntryName);
    }
    // Windows 的 `C:\...` 形状。
    if path.len() >= 2 && path.as_bytes()[1] == b':' {
        return Err(ZipError::UnsafeEntryName);
    }
    if path.ends_with('/') {
        return Err(ZipError::UnsafeEntryName);
    }
    // ⛔ 分隔符一律 `/`：Windows 上也不许出现 `\`（契约 §4.5）。
    if path.contains('\\') {
        return Err(ZipError::UnsafeEntryName);
    }
    for part in path.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return Err(ZipError::UnsafeEntryName);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 扫目录算清单
// ---------------------------------------------------------------------------

/// 扫目录的计数。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScanStats {
    /// 取不到的文件（跳过 + 计数，⛔ 不让整份清单失败）。
    pub unreadable: u64,
    /// 规则没收集的文件。
    pub excluded: u64,
    /// 因超上限被截断的文件。
    pub truncated: u64,
    /// 上限是否触顶。
    pub size_capped: bool,
}

/// 扫 profile 根目录，按规则算出一份清单（契约 §4.5 的形状与顺序）。
///
/// # 为什么客户端要自己扫
///
/// 线协议的帧类型里**没有「清单」这一帧**（`0x01`–`0x05`、`0x81`、`0x82`
/// 都不是清单）⇒ 清单到不了客户端手上。⇒ 客户端按**同一套规则**自己算，
/// 算出来的形状与内核侧逐字节一致（路径规则 + 字节序排序）。
///
/// # 规则（与内核侧 `CollectProfileBundle` 同一条）
///
/// 1. 取不到的文件 ⇒ 跳过 + 计数（**不**让整份清单失败）；
/// 2. **地板**：`Cookies` / `Network` 任何情况下都不带；
/// 3. 不匹配任何白名单 ⇒ 不收；
/// 4. 匹配黑名单 ⇒ 不收（黑名单优先于白名单）；
/// 5. 超上限 ⇒ **截断**（该文件与它之后的都不收）+ 计数。
///
/// ⛔ 符号链接、目录、管道**一律不收**：profile 里一个指向别处的链接会
/// 把 profile 外面的东西带进包里。
#[must_use]
pub fn scan_profile(root: &Path, rules: &ProfileSyncRules) -> (ProfileManifest, ScanStats) {
    let mut stats = ScanStats::default();
    let mut found: Vec<ManifestFile> = Vec::new();
    // 待扫的目录 + 它相对根的路径（`/` 分隔）。
    let mut stack: Vec<(PathBuf, String)> = vec![(root.to_path_buf(), String::new())];

    while let Some((dir, relative)) = stack.pop() {
        let Ok(read_dir) = std::fs::read_dir(&dir) else {
            // 目录读不动 ⇒ 跳过（不失败）。
            stats.unreadable += 1;
            continue;
        };
        for entry in read_dir.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let child = if relative.is_empty() {
                name
            } else {
                format!("{relative}/{name}")
            };
            // ⛔ 符号链接不跟：`DirEntry::metadata` 不跟随符号链接。
            let Ok(meta) = entry.metadata() else {
                stats.unreadable += 1;
                continue;
            };
            if meta.is_dir() {
                stack.push((entry.path(), child));
                continue;
            }
            // ⛔ 只收普通文件（跳过符号链接 / 管道 / 设备）。
            if !meta.is_file() {
                continue;
            }
            if never_goes_out(&child) {
                stats.excluded += 1;
                continue;
            }
            if !rules.allows(&child) {
                stats.excluded += 1;
                continue;
            }
            found.push(ManifestFile {
                path: child,
                size: meta.len(),
            });
        }
    }

    // 契约 §4.5：按路径的**字节序**升序。顺序在套上限**之前**定下来 ⇒
    // 上限切在哪里只取决于清单，不取决于扫目录的顺序。
    found.sort_by(|left, right| compare_paths(&left.path, &right.path));

    let ceiling = rules.ceiling();
    let mut manifest = ProfileManifest::default();
    for file in found {
        if stats.size_capped {
            stats.truncated += 1;
            continue;
        }
        // 减法不会下溢：total_bytes 永远不超过上限。
        if file.size > ceiling.saturating_sub(manifest.total_bytes) {
            stats.size_capped = true;
            stats.truncated += 1;
            continue;
        }
        manifest.total_bytes += file.size;
        manifest.files.push(file);
    }
    (manifest, stats)
}

// ---------------------------------------------------------------------------
// 写归档
// ---------------------------------------------------------------------------

/// 一个待写进归档的条目。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZipEntry {
    /// 归档里的路径（`/` 分隔的相对路径）。
    pub name: String,
    /// 源文件在磁盘上的位置。
    pub source: PathBuf,
    /// 源文件的字节数（用于套上限）。
    pub size: u64,
}

/// 把清单变成待写进归档的条目（**地板**在这里生效）。
///
/// `root` 是 profile 根目录：清单里的 `path` 相对它解析。
///
/// ⛔ 地板：清单里出现 `Cookies` / `Network Cache` 的条目**直接丢掉**。
/// 清单是远端输入（内核产的清单也是从管道过来的），客户端不该假设它干净。
///
/// ⚠️ 这里**不重做**白名单/黑名单：那是内核侧的事（它产出清单时已经按
/// 规则筛过）。客户端只补一层地板。
#[must_use]
pub fn build_entries(manifest: &ProfileManifest, root: &Path) -> (Vec<ZipEntry>, u64) {
    let mut entries = Vec::with_capacity(manifest.files.len());
    let mut skipped = 0u64;
    for file in &manifest.files {
        if never_goes_out(&file.path) {
            skipped += 1;
            continue;
        }
        entries.push(ZipEntry {
            name: file.path.clone(),
            source: root.join(&file.path),
            size: file.size,
        });
    }
    (entries, skipped)
}

/// 写归档的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZipOutcome {
    /// 归档的字节数。
    pub archive_bytes: u64,
    /// 实际写进去的条目名（按写入顺序）。
    pub packed: Vec<String>,
    /// 因超限被截断的条目数。
    pub truncated: u64,
    /// 打不开而被跳过的条目数（⛔ 不让整包失败）。
    pub skipped_unreadable: u64,
    /// 上限是否触顶。
    pub size_capped: bool,
}

/// 校验一个归档条目名（zip slip 防护 + 契约 §4.5 的路径规则）。
///
/// ⛔ 不带前导 `/`、不带盘符（`C:`）、不带 `./`、不含 `..` 段、
/// 不含连续 `//`、不含空段、结尾不带 `/`、不含 `\`。
pub fn check_entry_name(name: &str) -> Result<(), ZipError> {
    if name.is_empty() || name.starts_with('/') || name.starts_with('\\') {
        return Err(ZipError::UnsafeEntryName);
    }
    // Windows 的 `C:\...` 形状。
    if name.len() >= 2 && name.as_bytes()[1] == b':' {
        return Err(ZipError::UnsafeEntryName);
    }
    if name.ends_with('/') {
        return Err(ZipError::UnsafeEntryName);
    }
    // ⛔ 分隔符一律 `/`：Windows 上也不许出现 `\`（契约 §4.5）。
    if name.contains('\\') {
        return Err(ZipError::UnsafeEntryName);
    }
    for part in name.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return Err(ZipError::UnsafeEntryName);
        }
    }
    Ok(())
}

/// 把条目写成一个 zip 归档。
///
/// - 压缩方法：**deflate(8)**（zip 规范只允许 store(0) 与 deflate(8)，
///   其余方法本卡不用）。
/// - ⛔ **流式**：每个源文件按 64 KiB 块读进 zip，**不把文件整个读进
///   内存**；归档直接写进 `out`（生产路径上是 `File`）⇒ **整包不进内存**。
/// - 单包上限 `ceiling` 字节：超限**截断**（后面的条目不再写），
///   计数在 [`ZipOutcome::truncated`]。
///
/// 条目名在写之前**全部**校验一遍（zip slip 防护）：不合格的条目直接失败，
/// 不写半截归档。⛔ 但**打不开的文件**只跳过 + 计数（[`ZipOutcome::skipped_unreadable`]）
/// —— 一个锁住的文件不该让整包失败。
pub fn write_archive<W: Write + Seek>(
    out: &mut W,
    entries: &[ZipEntry],
    ceiling: u64,
) -> Result<ZipOutcome, ZipError> {
    // 先把条目名**全部**校验一遍：宁可一个都不写，也不要留半截归档。
    for entry in entries {
        check_entry_name(&entry.name)?;
    }

    let mut outcome = ZipOutcome {
        archive_bytes: 0,
        packed: Vec::new(),
        truncated: 0,
        skipped_unreadable: 0,
        size_capped: false,
    };
    let mut packed_bytes = 0u64;
    let mut writer = zip::ZipWriter::new(out);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for entry in entries {
        if outcome.size_capped {
            outcome.truncated += 1;
            continue;
        }
        // 减法不会下溢：packed_bytes 永远不超过上限。
        if entry.size > ceiling.saturating_sub(packed_bytes) {
            outcome.size_capped = true;
            outcome.truncated += 1;
            continue;
        }
        // ⛔ 先开文件：打不开就跳过 + 计数（不许让整包失败）。
        let mut file = match std::fs::File::open(&entry.source) {
            Ok(file) => file,
            Err(_) => {
                outcome.skipped_unreadable += 1;
                continue;
            }
        };
        writer
            .start_file(entry.name.clone(), options)
            .map_err(|err| ZipError::Write(err.to_string()))?;
        // 写进去的字节**不许超过剩下的额度** ⇒ 上限是真的上限。
        let (written, capped) = copy_capped(&mut file, &mut writer, ceiling - packed_bytes)?;
        packed_bytes += written;
        if capped {
            outcome.size_capped = true;
            outcome.truncated += 1;
        }
        outcome.packed.push(entry.name.clone());
    }
    // `finish` 写完中央目录并把底层 writer 还回来 ⇒ 归档的字节数就是
    // 底层流的位置。
    let inner = writer
        .finish()
        .map_err(|err| ZipError::Write(err.to_string()))?;
    outcome.archive_bytes = inner
        .stream_position()
        .map_err(|err| ZipError::Write(err.to_string()))?;
    Ok(outcome)
}

/// 把一个文件按块复制进 zip，最多写 `limit` 字节。
///
/// 返回 `(写入字节数, 是否撞到上限)`。
fn copy_capped<W: Write>(
    file: &mut std::fs::File,
    writer: &mut W,
    limit: u64,
) -> Result<(u64, bool), ZipError> {
    let mut chunk = vec![0u8; 64 * 1024];
    let mut written = 0u64;
    loop {
        if written == limit {
            // 额度用尽：再看一眼还有没有数据 ⇒ 有就是撞了上限。
            let mut probe = [0u8; 1];
            let read = file
                .read(&mut probe)
                .map_err(|err| ZipError::Io(err.to_string()))?;
            return Ok((written, read > 0));
        }
        let want = std::cmp::min(chunk.len() as u64, limit - written) as usize;
        let read = file
            .read(&mut chunk[..want])
            .map_err(|err| ZipError::Io(err.to_string()))?;
        if read == 0 {
            return Ok((written, false));
        }
        writer
            .write_all(&chunk[..read])
            .map_err(|err| ZipError::Write(err.to_string()))?;
        written += read as u64;
    }
}
