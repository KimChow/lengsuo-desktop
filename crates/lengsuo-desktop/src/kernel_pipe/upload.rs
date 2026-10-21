//! Profile 云同步的**客户端侧编排层**：打包 ⇒ 上传 ⇒ 失败不阻塞。
//!
//! # 分工（契约 §4.5）
//!
//! 内核侧（`native/profile_bundle.*`）拥有**规则**（白名单/黑名单/上限），
//! 客户端侧拥有**清单的形状、打包与上传**。
//!
//! ⇒ 本模块**不另定一套收集规则**：规则在
//! [`crate::kernel_pipe::profile_rules`]，路径形状与排序规则由契约 §4.5
//! 钉死（`path` / `size` / `kind` 三字段，`/` 分隔的相对路径，按路径
//! **字节序**升序）⇒ 两侧算出的清单才逐项相同。
//!
//! # 隐私（拍板 1B / 2A / 3A）
//!
//! - **1B**：浏览历史**默认上传**。白名单写在配置里，不写死在代码里。
//!   ⚠️ 代价：浏览历史默认上云，是隐私事故的高发点；163,840 B/包的体量
//!   会拖慢关闭流程。上线前应补一条隐私提示。
//! - **2A**：上传失败**不阻塞**容器关闭。失败记日志 + 计数，容器照常关闭。
//!   ⚠️ 代价：失败是**静默**的，只能靠日志/计数发现；下次同步补。
//! - **3A**：`data_url`（画布位图）**默认不采**。本模块**不产生**任何
//!   `data_url`；内核侧 `action_table` 的 `data_url_allowed_` 默认 `false`
//!   保持不动。
//!
//! # 安全边界
//!
//! - ⛔ `Cookies` / `Network Cache` **任何情况下都不带**。这是**地板**
//!   不是偏好：配置是远端输入，即便它把 cookies 写进白名单也不带。
//! - ⛔ 32 MiB 单包上限，超限**截断并记日志**。
//! - ⛔ 不把整包读进内存：流式写 + 流式上传（见
//!   [`crate::kernel_pipe::zip_writer`]）。
//! - ⛔ 日志里只有**文件名与字节数**，不记文件内容。

use std::path::Path;

use crate::kernel_pipe::zip_writer::{
    build_entries, scan_profile, write_archive, ProfileManifest, ScanStats,
};
use crate::kernel_pipe::{Frame, FrameType};

pub use crate::kernel_pipe::profile_rules::{
    match_glob, never_goes_out, ProfileSyncRules, RulesError, MAX_PACKAGE_BYTES, PROFILE_SYNC_KEY,
};

// ---------------------------------------------------------------------------
// 错误
// ---------------------------------------------------------------------------

/// 打包 / 上传失败。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadError {
    /// 配置包畸形。
    MalformedConfig,
    /// 清单畸形。
    MalformedManifest,
    /// 读文件失败。
    Io(String),
    /// 写 zip 失败。
    Zip(String),
    /// 上传失败。
    Transport(String),
}

impl std::fmt::Display for UploadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MalformedConfig => write!(f, "配置包畸形"),
            Self::MalformedManifest => write!(f, "清单畸形"),
            Self::Io(what) => write!(f, "读文件失败：{what}"),
            Self::Zip(what) => write!(f, "写 zip 失败：{what}"),
            Self::Transport(what) => write!(f, "上传失败：{what}"),
        }
    }
}

impl std::error::Error for UploadError {}

impl From<RulesError> for UploadError {
    fn from(err: RulesError) -> Self {
        match err {
            RulesError::MalformedConfig => Self::MalformedConfig,
        }
    }
}

// ---------------------------------------------------------------------------
// 上传
// ---------------------------------------------------------------------------

/// 上传的目的地。
///
/// 走通道①（runner ↔ 平台）。本卡**不实现** HTTPS 客户端：平台侧的
/// 凭据与端点在另一条线（任务 S），这里只把「把字节交出去」这件事
/// 做成可注入的，这样失败分支能测。
///
/// ⛔ 平台凭据**不进**这个接口：上传方自己拿凭据。
pub trait ProfileUploader {
    /// 把一个 zip 包上传到平台。
    ///
    /// `package_path` 是归档**在磁盘上的位置**：上传方自己流式读它。
    /// ⛔ 本层**不把整包读进内存**（32 MiB 的包会把进程内存吃穿）。
    ///
    /// 返回 `Err` 表示上传失败。**调用方不许因此阻塞容器关闭**。
    fn upload(&mut self, container_id: &str, package_path: &Path) -> Result<(), UploadError>;
}

/// 通道①**尚未接入**时的默认上传器：记日志 + 返回失败。
///
/// # 为什么不是一个「假装成功」的空实现
///
/// 通道①（runner ↔ 平台）在本仓**没有实现**（那是任务 S 的活）。
/// 若默认实现返回 `Ok`，计数会说「上传了 N 个包」而实际一个字节都没
/// 出去 ⇒ **计数撒谎**。⇒ 默认实现**诚实地失败**：
/// 记一行日志（说明包已备好、只差传输）+ 计数加一。
///
/// 拍板 2A 保证这**不阻塞**容器关闭：容器照常关闭，下次同步补。
///
/// ⛔ 上层接上真实的通道①之后，应当注入真正的实现（[`ProfileUploader`]）。
#[derive(Debug, Clone, Copy, Default)]
pub struct UnavailableUploader;

impl ProfileUploader for UnavailableUploader {
    fn upload(&mut self, _container_id: &str, package_path: &Path) -> Result<(), UploadError> {
        Err(UploadError::Transport(format!(
            "通道①尚未接入（包已备好在 {}）",
            package_path.display()
        )))
    }
}

/// 上传统计。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UploadStats {
    /// 成功上传的包数。
    pub uploaded: u64,
    /// 上传失败的包数（拍板 2A：只计数，不阻塞关闭）。
    pub failed: u64,
    /// 因超限被截断的次数。
    pub truncated: u64,
}

/// 上传的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadOutcome {
    /// 包上传成功了吗。
    pub uploaded: bool,
    /// 统计。
    pub stats: UploadStats,
}

/// 把清单打包成 zip 并上传。
///
/// # 失败语义（拍板 2A）
///
/// 上传失败**不阻塞**容器关闭：本函数**返回 `Ok`**（`uploaded = false`），
/// 把失败记进 `stats.failed` 并交给 `log`。容器照常关闭。
///
/// 打包本身失败（zip 写不下去）也一样不阻塞：那是本地的失败，
/// 上传没发生，`stats.failed` 加一。
///
/// ⛔ 打包与上传都**不返回错误给调用方去中断关闭流程** —— 这是 2A 的
/// 全部内容。调用方拿到的 `Ok` 意味着「关闭可以继续」。
///
/// # 打包到哪儿
///
/// `package_path` 是归档的落盘位置：zip **直接写进那个文件**（流式），
/// 整包从不进内存 ⇒ 32 MiB 的包不会把进程的内存吃穿。
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn upload_bundle(
    uploader: &mut dyn ProfileUploader,
    container_id: &str,
    manifest: &ProfileManifest,
    root: &Path,
    rules: &ProfileSyncRules,
    package_path: &Path,
    log: &mut dyn FnMut(&str),
    stats: &mut UploadStats,
) -> UploadOutcome {
    let (entries, skipped) = build_entries(manifest, root);
    if skipped > 0 {
        log(&format!(
            "[lengsuo-desktop] profile 同步：{} 个文件被规则挡下",
            skipped
        ));
    }

    let mut file = match std::fs::File::create(package_path) {
        Ok(file) => file,
        Err(err) => {
            stats.failed += 1;
            log(&format!(
                "[lengsuo-desktop] profile 同步：建归档失败（{}）",
                err
            ));
            return UploadOutcome {
                uploaded: false,
                stats: *stats,
            };
        }
    };
    let written = write_archive(&mut file, &entries, rules.ceiling());
    // 归档文件句柄在这里就关上（后面要读它的字节来上传）。
    drop(file);

    let outcome = match written {
        Ok(outcome) => outcome,
        Err(err) => {
            stats.failed += 1;
            log(&format!(
                "[lengsuo-desktop] profile 同步：打包失败（{err}）"
            ));
            return UploadOutcome {
                uploaded: false,
                stats: *stats,
            };
        }
    };
    if outcome.size_capped {
        stats.truncated += 1;
        log("[lengsuo-desktop] profile 同步：包超上限，已截断");
    }
    if outcome.skipped_unreadable > 0 {
        log(&format!(
            "[lengsuo-desktop] profile 同步：{} 个文件读不动，已跳过",
            outcome.skipped_unreadable
        ));
    }
    log(&format!(
        "[lengsuo-desktop] profile 同步：打包完成，{} 个文件，{} 字节",
        outcome.packed.len(),
        outcome.archive_bytes
    ));

    // 上传传的是**归档文件的位置**（不是内存里的副本）⇒ 整包不进内存。
    match uploader.upload(container_id, package_path) {
        Ok(()) => {
            stats.uploaded += 1;
            log("[lengsuo-desktop] profile 同步：上传完成");
            UploadOutcome {
                uploaded: true,
                stats: *stats,
            }
        }
        Err(err) => {
            // ⛔ 2A：失败是静默的，只靠日志/计数发现。容器照常关闭。
            stats.failed += 1;
            log(&format!(
                "[lengsuo-desktop] profile 同步：上传失败（{err}）"
            ));
            UploadOutcome {
                uploaded: false,
                stats: *stats,
            }
        }
    }
}

/// **容器关闭**时的一次 profile 同步：扫目录 ⇒ 打包 ⇒ 上传。
///
/// 这是本卡给关闭流程留的**唯一入口**：调用方在关容器前调它，然后
/// **照常关闭**——它不会返回「必须中断关闭」的信号（拍板 2A）。
///
/// - `profile_root`：profile 根目录（清单里的路径相对它）。
/// - `package_path`：归档落盘位置（临时文件；调用方负责清理）。
/// - `rules`：收集规则（平台下发什么就用什么）。
///
/// ⚠️ 本卡**不实现**平台侧的 HTTPS 客户端（那是任务 S）：调用方注入一个
/// [`ProfileUploader`]。本卡只保证「打包 + 交出去 + 失败不阻塞」这三件事。
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn sync_profile_at_close(
    uploader: &mut dyn ProfileUploader,
    container_id: &str,
    profile_root: &Path,
    rules: &ProfileSyncRules,
    package_path: &Path,
    log: &mut dyn FnMut(&str),
    stats: &mut UploadStats,
) -> UploadOutcome {
    let (manifest, scan): (ProfileManifest, ScanStats) = scan_profile(profile_root, rules);
    log(&format!(
        "[lengsuo-desktop] profile 同步：清单 {} 个文件，{} 字节",
        manifest.files.len(),
        manifest.total_bytes
    ));
    if scan.unreadable > 0 {
        log(&format!(
            "[lengsuo-desktop] profile 同步：{} 个文件读不动，已跳过",
            scan.unreadable
        ));
    }
    if scan.size_capped {
        // ⛔ 清单层面就撞了上限 ⇒ 记日志 + 计数（与打包层同一句话）。
        stats.truncated += 1;
        log("[lengsuo-desktop] profile 同步：包超上限，已截断");
    }
    upload_bundle(
        uploader,
        container_id,
        &manifest,
        profile_root,
        rules,
        package_path,
        log,
        stats,
    )
}

// ---------------------------------------------------------------------------
// 握手后立刻发的那一帧 ConfigBundle
// ---------------------------------------------------------------------------

/// 握手后立刻发的那一帧 `ConfigBundle`。
///
/// 内核侧 `KernelPipeClient::Initialize` 的顺序是
/// `Hello` → `KeyEnvelope` → **`ConfigBundle`** → 才开始发事件。
/// P2 只做到 `KeyEnvelope` ⇒ **本卡必须补上这一步**，否则内核会
/// **卡在等 `ConfigBundle`**（连接不报错，但内核不进入事件阶段）。
///
/// 载荷是 **JSON**：内核侧 `KernelPipeConfig::Freeze` 解析一次后冻结，
/// 再也不读 ⇒ 必须是合法 JSON，且**冻结后不可变**。
///
/// ⛔ 载荷里**不许**有密钥、会话密钥、真实用户数据、`data_url`。
/// `data_url` 是画布位图（拍板 3A：默认不采）——它属于上行事件，
/// 不属于配置。
///
/// ⭐ 键序**手工拼**（不用 `serde_json::to_string`）：`serde_json::Map`
/// 的键序不保证 ⇒ 两侧的固定向量可能逐字节不同。
#[must_use]
pub fn config_bundle_frame(rules: &ProfileSyncRules) -> Frame {
    Frame::new(
        FrameType::ConfigBundle,
        config_bundle_payload(rules).into_bytes(),
    )
}

/// `ConfigBundle` 的载荷文本（**跨仓固定向量**）。
///
/// 键序写死：`profile_sync` → `include` → `exclude` → `max_package_bytes`。
/// ⚠️ 内核侧 `ParseProfileSyncConfig` 读的是**同一段**（`include` 是
/// 必填、`exclude` 可选、`max_package_bytes` 可选）⇒ 两侧对这份载荷
/// 给出同一个答案。
///
/// ⛔ 这里只出现**路径规则**，不出现密钥、会话密钥、真实用户数据、
/// `data_url`。
#[must_use]
pub fn config_bundle_payload(rules: &ProfileSyncRules) -> String {
    let mut payload = String::from("{\"profile_sync\":{\"include\":[");
    push_patterns(&mut payload, &rules.include);
    payload.push_str("],\"exclude\":[");
    push_patterns(&mut payload, &rules.exclude);
    payload.push_str("],\"max_package_bytes\":");
    payload.push_str(&rules.max_package_bytes.to_string());
    payload.push_str("}}");
    payload
}

/// 把一组模式写进一个 JSON 数组。
fn push_patterns(out: &mut String, patterns: &[String]) {
    for (index, pattern) in patterns.iter().enumerate() {
        if index != 0 {
            out.push(',');
        }
        out.push('"');
        out.push_str(&escape_json(pattern));
        out.push('"');
    }
}

/// 把一个字符串转义成 JSON 字符串的内容（不含两端的引号）。
fn escape_json(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 跨仓固定向量：默认规则下的 `ConfigBundle` 载荷。
    ///
    /// 内核侧 `profile_bundle_test.cc` 用**同一组模式**（顺序也相同）
    /// ⇒ 两侧的默认配置包语义一致。
    const DEFAULT_BUNDLE_PAYLOAD: &str = concat!(
        r#"{"profile_sync":{"include":["*Local Storage*","*Extension State*","*History*"],"#,
        r#""exclude":["*Cache*","*Cookies*","*Network*"],"#,
        r#""max_package_bytes":33554432}}"#
    );

    /// 造一个临时目录（测试结束删掉）。
    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "lengsuo-upload-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("应能建临时目录");
        dir
    }

    /// 在 `root` 下造一个文件（内容是**人造字节**，不是真实用户数据）。
    fn make_file(root: &Path, relative: &str, content: &[u8]) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("应能建子目录");
        }
        std::fs::write(&path, content).expect("应能写文件");
    }

    /// 一个总是失败的上传器（测 2A 的失败分支）。
    struct FailingUploader;

    impl ProfileUploader for FailingUploader {
        fn upload(&mut self, _container_id: &str, _package_path: &Path) -> Result<(), UploadError> {
            Err(UploadError::Transport("平台不可达".to_string()))
        }
    }

    /// 一个成功的上传器（记下收到的字节）。
    struct RecordingUploader {
        got: Vec<u8>,
    }

    impl ProfileUploader for RecordingUploader {
        fn upload(&mut self, _container_id: &str, package_path: &Path) -> Result<(), UploadError> {
            self.got =
                std::fs::read(package_path).map_err(|err| UploadError::Io(err.to_string()))?;
            Ok(())
        }
    }

    // -- 3.2：32 MiB 上限 ⇒ 截断并记日志 -----------------------------------

    #[test]
    fn the_package_is_truncated_and_logged_when_it_exceeds_the_ceiling() {
        // 单包上限 32 MiB：一个 33 MiB 的条目必须被**截断**，且**记日志**。
        let root = temp_dir("cap");
        let big = root.join("Default/History");
        std::fs::create_dir_all(big.parent().expect("应有父目录")).expect("应能建目录");
        // 稀疏文件：只设长度，不真写 33 MiB。
        let file = std::fs::File::create(&big).expect("应能建文件");
        file.set_len(MAX_PACKAGE_BYTES + 1024).expect("应能设长度");

        let mut stats = UploadStats::default();
        let mut lines: Vec<String> = Vec::new();
        let mut uploader = RecordingUploader { got: Vec::new() };
        let outcome = sync_profile_at_close(
            &mut uploader,
            "ct-01",
            &root,
            &ProfileSyncRules::default(),
            &root.join("package.zip"),
            &mut |line| lines.push(line.to_string()),
            &mut stats,
        );
        assert!(outcome.uploaded, "上传本身应成功");
        assert_eq!(stats.truncated, 1, "超限必须被计数");
        assert!(
            lines.iter().any(|line| line.contains("已截断")),
            "截断必须记日志：{lines:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    // -- 3.3 / 2A：上传失败不阻塞关闭 --------------------------------------

    #[test]
    fn a_failed_upload_does_not_block_the_close() {
        // 拍板 2A：上传失败**不阻塞**容器关闭 ⇒ 函数返回 Ok（uploaded=false），
        // 失败只记日志 + 计数。
        let root = temp_dir("fail");
        make_file(&root, "Default/History", b"history-bytes");
        let mut stats = UploadStats::default();
        let mut lines: Vec<String> = Vec::new();
        let mut uploader = FailingUploader;
        let outcome = sync_profile_at_close(
            &mut uploader,
            "ct-01",
            &root,
            &ProfileSyncRules::default(),
            &root.join("package.zip"),
            &mut |line| lines.push(line.to_string()),
            &mut stats,
        );
        // ⛔ 关键：返回值不是 Err —— 调用方（关闭流程）拿不到「必须中断」的信号。
        assert!(!outcome.uploaded, "上传应失败");
        assert_eq!(stats.failed, 1, "失败必须被计数");
        assert_eq!(stats.uploaded, 0);
        assert!(
            lines.iter().any(|line| line.contains("上传失败")),
            "失败必须记日志：{lines:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_successful_upload_counts_and_delivers_the_bytes() {
        let root = temp_dir("ok");
        make_file(&root, "Default/History", b"history-bytes");
        let mut stats = UploadStats::default();
        let mut uploader = RecordingUploader { got: Vec::new() };
        let outcome = sync_profile_at_close(
            &mut uploader,
            "ct-01",
            &root,
            &ProfileSyncRules::default(),
            &root.join("package.zip"),
            &mut |_line| {},
            &mut stats,
        );
        assert!(outcome.uploaded);
        assert_eq!(stats.uploaded, 1);
        assert_eq!(stats.failed, 0);
        assert!(!uploader.got.is_empty(), "上传器应收到归档字节");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_default_uploader_fails_honestly_instead_of_pretending() {
        // 通道①未接入 ⇒ 默认实现**诚实地失败**（计数不许撒谎）。
        let root = temp_dir("default-uploader");
        make_file(&root, "Default/History", b"history-bytes");
        let mut stats = UploadStats::default();
        let mut lines: Vec<String> = Vec::new();
        let mut uploader = UnavailableUploader;
        let outcome = sync_profile_at_close(
            &mut uploader,
            "ct-01",
            &root,
            &ProfileSyncRules::default(),
            &root.join("package.zip"),
            &mut |line| lines.push(line.to_string()),
            &mut stats,
        );
        assert!(!outcome.uploaded, "没有通道①就不许说上传成功");
        assert_eq!(stats.uploaded, 0, "上传计数不许撒谎");
        assert_eq!(stats.failed, 1, "失败必须被计数");
        let _ = std::fs::remove_dir_all(&root);
    }

    // -- 3A：data_url 默认不采 ---------------------------------------------

    #[test]
    fn the_config_bundle_never_carries_a_data_url_or_key_material() {
        // 拍板 3A：data_url 默认不采 ⇒ 配置包里不许出现它。
        let frame = config_bundle_frame(&ProfileSyncRules::default());
        let text = String::from_utf8(frame.payload).expect("载荷应是 UTF-8");
        assert!(
            !text.contains("data_url"),
            "配置包里不许有 data_url：{text}"
        );
        // ⛔ 也不许有密钥 / 会话密钥。
        assert!(
            !text.contains("session_key"),
            "配置包里不许有会话密钥：{text}"
        );
        assert!(
            !text.contains("k_machine"),
            "配置包里不许有机器密钥：{text}"
        );
    }

    #[test]
    fn the_uploaded_package_never_carries_a_data_url() {
        // 清单形状本身只容得下路径与大小 ⇒ 不可能有 data_url。
        let root = temp_dir("dataurl");
        make_file(&root, "Default/History", b"history-bytes");
        let mut uploader = RecordingUploader { got: Vec::new() };
        let mut stats = UploadStats::default();
        let _outcome = sync_profile_at_close(
            &mut uploader,
            "ct-01",
            &root,
            &ProfileSyncRules::default(),
            &root.join("package.zip"),
            &mut |_line| {},
            &mut stats,
        );
        assert!(
            !String::from_utf8_lossy(&uploader.got).contains("data_url"),
            "包里不许有 data_url"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    // -- 必做项 0：ConfigBundle 帧 ------------------------------------------

    #[test]
    fn the_config_bundle_frame_is_well_formed() {
        let frame = config_bundle_frame(&ProfileSyncRules::default());
        assert_eq!(frame.frame_type, FrameType::ConfigBundle);
        assert_eq!(frame.frame_type.code(), 0x03, "帧类型必须是 0x03");

        let encoded = frame.encode().expect("应可编码");
        // 帧头 8 字节：len 大端、type = 0x03、reserved 三字节全 0。
        assert_eq!(
            &encoded[..8],
            &[0x00, 0x00, 0x00, 0x9b, 0x03, 0x00, 0x00, 0x00],
            "帧头必须是 len=155（大端）、type=0x03、reserved=0"
        );
        assert_eq!(encoded.len() - 8, 155, "len 必须是载荷字节数");

        // 载荷是合法 JSON，且与跨仓固定向量逐字节相同。
        let payload = &encoded[8..];
        assert_eq!(payload, DEFAULT_BUNDLE_PAYLOAD.as_bytes());
        let parsed: serde_json::Value = serde_json::from_slice(payload).expect("应是合法 JSON");
        assert!(parsed.get("profile_sync").is_some());

        // 能被解帧器解开（往返）。
        let (decoded, used) = crate::kernel_pipe::decode_frame(&encoded).expect("应能解码");
        assert_eq!(used, encoded.len());
        assert_eq!(decoded.frame_type, FrameType::ConfigBundle);
        assert_eq!(decoded.payload, payload);
    }

    #[test]
    fn the_config_bundle_round_trips_through_the_rules_parser() {
        // 两侧对同一份 JSON 必须给出同一个答案（内核侧
        // `ParseProfileSyncConfig` 是这份载荷的消费者）。
        let rules = ProfileSyncRules::from_config_bundle(DEFAULT_BUNDLE_PAYLOAD)
            .expect("应能解析")
            .expect("应有 profile_sync 段");
        assert_eq!(rules.include, ProfileSyncRules::default().include);
        assert_eq!(rules.exclude, ProfileSyncRules::default().exclude);
        assert_eq!(rules.max_package_bytes, MAX_PACKAGE_BYTES);
    }
}
