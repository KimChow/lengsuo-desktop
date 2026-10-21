//! `zip_writer` 的单元测试：契约 §4.5 的清单形状 + zip slip 防护 + 上限。
//!
//! ⛔ 固定测试向量**硬编码**（契约 §4.5 的 JSON 形状）——不在测试里算期望值。
//! ⛔ 测试固件里**不许**出现凭据、密钥、真实用户数据（只有人造字节）。

use crate::kernel_pipe::profile_rules::ProfileSyncRules;
use crate::kernel_pipe::zip_writer::*;

/// 造一个临时目录（测试结束删掉）。
fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "lengsuo-zip-{tag}-{}-{:?}",
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

// -- 契约 §4.5：清单的形状 ---------------------------------------------

#[test]
fn the_manifest_shape_is_the_contract_one() {
    // 契约 §4.5 的 JSON 形状，逐字节照抄。
    let json =
        r#"{"files":[{"path":"Local Storage/leveldb/000003.log","size":1024,"kind":"file"}]}"#;
    let manifest = ProfileManifest::parse(json).expect("应能解析");
    assert_eq!(manifest.files.len(), 1);
    assert_eq!(manifest.files[0].path, "Local Storage/leveldb/000003.log");
    assert_eq!(manifest.files[0].size, 1024);
    assert_eq!(manifest.total_bytes, 1024);
}

#[test]
fn an_empty_manifest_is_an_empty_array_not_null() {
    // 契约 §4.5：空清单是 `[]`（⛔ 不是 `null`）。
    let manifest = ProfileManifest::parse(r#"{"files":[]}"#).expect("应能解析");
    assert!(manifest.files.is_empty());
    assert_eq!(manifest.total_bytes, 0);
    // ⛔ `null` 不该被当成空清单蒙混过关。
    assert!(ProfileManifest::parse(r#"{"files":null}"#).is_err());
}

#[test]
fn the_manifest_must_have_exactly_the_files_key() {
    // 契约 §4.5：顶层**恰好一个键** `files`。
    assert!(ProfileManifest::parse(r#"{"files":[],"extra":1}"#).is_err());
    assert!(ProfileManifest::parse(r#"{"other":[]}"#).is_err());
}

#[test]
fn the_manifest_is_sorted_byte_wise_not_by_locale() {
    // 契约 §4.5：按 path 的**字节序**升序。
    // ⛔ 不是 locale 序（随 LC_COLLATE 变）、⛔ 不是大小写不敏感序。
    let json = r#"{"files":[
        {"path":"b","size":1,"kind":"file"},
        {"path":"History","size":1,"kind":"file"},
        {"path":"history","size":1,"kind":"file"},
        {"path":"A","size":1,"kind":"file"}
    ]}"#;
    let manifest = ProfileManifest::parse(json).expect("应能解析");
    let paths: Vec<&str> = manifest.files.iter().map(|f| f.path.as_str()).collect();
    // 字节序：大写 < 小写；`History` < `history`（不折叠大小写）。
    assert_eq!(paths, vec!["A", "History", "b", "history"]);
}

#[test]
fn a_manifest_entry_with_a_bad_kind_is_refused() {
    // 契约 §4.5：`kind` 目前只有 `"file"`。
    let json = r#"{"files":[{"path":"a","size":1,"kind":"dir"}]}"#;
    assert!(ProfileManifest::parse(json).is_err());
}

#[test]
fn a_manifest_entry_missing_a_field_is_refused() {
    for json in [
        r#"{"files":[{"size":1,"kind":"file"}]}"#,
        r#"{"files":[{"path":"a","kind":"file"}]}"#,
        r#"{"files":[{"path":"a","size":1}]}"#,
        r#"{"files":[{"path":"a","size":-1,"kind":"file"}]}"#,
    ] {
        assert!(
            ProfileManifest::parse(json).is_err(),
            "缺字段的条目 {json} 必须被拒绝"
        );
    }
}

#[test]
fn a_manifest_path_that_escapes_the_root_is_refused() {
    // ⛔ 契约 §4.5：不含 `..` 段、不含连续 `//`、不含空段、
    // 不带前导 `/`、不带盘符、不带结尾 `/`。
    for path in [
        "../etc/passwd",
        "a/../b",
        "/absolute",
        "C:/windows",
        "a//b",
        "a/",
        "./a",
        "a/./b",
        // ⛔ 分隔符一律 `/`：Windows 上也不许出现 `\`（契约 §4.5）。
        "a\\b",
    ] {
        // ⛔ 用 serde_json 造 JSON：`\` 在 JSON 里有转义规则
        // （`"a\\b"` 手工拼出来会被读成 `a` + 退格 + `b`）。
        let json = serde_json::json!({
            "files": [{"path": path, "size": 1, "kind": "file"}]
        })
        .to_string();
        assert!(
            ProfileManifest::parse(&json).is_err(),
            "越界路径 {path} 必须被拒绝"
        );
    }
}

// -- zip slip 防护 -------------------------------------------------------

#[test]
fn an_entry_name_that_escapes_the_root_is_refused() {
    // ⛔ zip slip：解压端按条目名写文件 ⇒ 一个 `../` 就能写到别处。
    for name in [
        "../evil",
        "/absolute",
        "C:/evil",
        "a/../../evil",
        "a\\b",
        "",
        "a/",
    ] {
        assert!(
            check_entry_name(name).is_err(),
            "不安全条目名 {name:?} 必须被拒绝"
        );
    }
    assert!(check_entry_name("Default/Local Storage/x.log").is_ok());
}

#[test]
fn the_written_archive_never_carries_an_absolute_path() {
    // ⛔ 验收项：zip 里的路径不许带绝对路径。
    let root = temp_dir("abs");
    make_file(&root, "Default/History", b"history-bytes");
    let (manifest, _) = scan_profile(&root, &ProfileSyncRules::default());
    let (entries, skipped) = build_entries(&manifest, &root);
    assert_eq!(skipped, 0);

    let archive = root.join("package.zip");
    let mut file = std::fs::File::create(&archive).expect("应能建归档");
    let outcome = write_archive(&mut file, &entries, MAX_CEILING).expect("应能写");
    drop(file);

    assert_eq!(outcome.packed.len(), 1);
    let name = &outcome.packed[0];
    assert!(!name.starts_with('/'), "不许带前导 /：{name}");
    assert!(!name.contains('\\'), "不许带反斜杠：{name}");
    assert!(!name.contains(':'), "不许带盘符：{name}");
    assert!(!name.contains(".."), "不许带 ..：{name}");

    // ⛔ 再从归档里读一遍：条目名与写进去的一致。
    let mut archive_file = std::fs::File::open(&archive).expect("应能打开归档");
    let mut zip = zip::ZipArchive::new(&mut archive_file).expect("应是合法 zip");
    assert_eq!(zip.len(), 1);
    let entry = zip.by_index(0).expect("应有条目");
    assert_eq!(entry.name().expect("应有条目名"), name.as_str());
    // 压缩方法是 deflate(8)（zip 规范里本卡只用这两种）。
    assert_eq!(entry.compression(), zip::CompressionMethod::Deflated);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_unsafe_entry_name_fails_the_whole_archive() {
    // ⛔ 条目名在写之前**全部**校验一遍：宁可一个都不写，
    // 也不留半截归档。
    let root = temp_dir("unsafe");
    let archive = root.join("package.zip");
    let mut file = std::fs::File::create(&archive).expect("应能建归档");
    let entries = vec![ZipEntry {
        name: "../evil".to_string(),
        source: root.join("x"),
        size: 0,
    }];
    assert!(write_archive(&mut file, &entries, MAX_CEILING).is_err());
    drop(file);
    let _ = std::fs::remove_dir_all(&root);
}

// -- 收集规则与上限 -----------------------------------------------------

#[test]
fn the_scan_collects_history_by_default_and_drops_the_forbidden() {
    // 拍板 1B + 地板。
    let root = temp_dir("scan");
    make_file(&root, "Default/History", b"history-bytes");
    make_file(&root, "Default/Local Storage/leveldb/0001.log", b"log");
    make_file(&root, "Default/Cookies/db.sqlite", b"cookies");
    make_file(&root, "Default/Network/Cache/data_1", b"cache");

    let (manifest, stats) = scan_profile(&root, &ProfileSyncRules::default());
    let paths: Vec<&str> = manifest.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        vec!["Default/History", "Default/Local Storage/leveldb/0001.log"]
    );
    assert_eq!(stats.excluded, 2, "Cookies 与 Network Cache 必须被排除");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_scan_sorts_byte_wise_regardless_of_the_walk_order() {
    // 契约 §4.5：顺序在**套上限之前**定下来 ⇒ 上限切在哪里只取决于
    // 清单，不取决于扫目录的顺序。
    let root = temp_dir("order");
    make_file(&root, "Default/History", b"h");
    make_file(&root, "Default/Extension State/a.dat", b"e");
    make_file(&root, "Default/Local Storage/b.log", b"l");
    let (manifest, _) = scan_profile(&root, &ProfileSyncRules::default());
    let paths: Vec<&str> = manifest.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        vec![
            "Default/Extension State/a.dat",
            "Default/History",
            "Default/Local Storage/b.log",
        ]
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_scan_cuts_the_collection_at_the_ceiling() {
    // 单包上限：超限 ⇒ 截断 + 计数（不失败）。
    let root = temp_dir("ceiling");
    make_file(&root, "Default/History", b"0123456789");
    make_file(&root, "Default/Local Storage/x.log", b"0123456789");
    let rules = ProfileSyncRules {
        include: vec!["*".to_string()],
        exclude: Vec::new(),
        max_package_bytes: 15,
    };
    let (manifest, stats) = scan_profile(&root, &rules);
    assert_eq!(manifest.files.len(), 1, "只收得下第一个");
    assert_eq!(stats.truncated, 1);
    assert!(stats.size_capped);
    assert_eq!(manifest.total_bytes, 10);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_manifest_carries_only_paths_and_sizes() {
    // ⛔ 清单里不许出现密钥、会话密钥、真实用户数据（只许路径与大小）
    // ⇒ **形状**本身只容得下路径与大小：条目只有 `path` / `size` / `kind`
    // 三个字段，多一个字段就解析不了。
    let root = temp_dir("shape");
    make_file(&root, "Default/History", b"history-bytes");
    let (manifest, _) = scan_profile(&root, &ProfileSyncRules::default());

    // 扫出来的清单能按契约 §4.5 的形状原样往返。
    let json = format!(
        r#"{{"files":[{}]}}"#,
        manifest
            .files
            .iter()
            .map(|f| format!(r#"{{"path":"{}","size":{},"kind":"file"}}"#, f.path, f.size))
            .collect::<Vec<_>>()
            .join(",")
    );
    let reparsed = ProfileManifest::parse(&json).expect("扫出来的清单应能按契约形状往返");
    assert_eq!(reparsed.files, manifest.files);

    // ⛔ 多一个字段（如 `data_url`）⇒ 解析不了（形状不给它位置）。
    let with_extra = r#"{"files":[{"path":"a","size":1,"kind":"file","data_url":"x"}]}"#;
    assert!(ProfileManifest::parse(with_extra).is_err());

    // ⛔ 路径里不许有 `data_url`。
    for file in &manifest.files {
        assert!(!file.path.contains("data_url"), "路径里不许有 data_url");
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// 测试用的上限（不必是 32 MiB —— 这里只验机制）。
const MAX_CEILING: u64 = 32 * 1024 * 1024;

#[test]
fn an_oversized_file_is_dropped_whole_and_the_archive_stays_readable() {
    // ⛔ 归档层撞上限时**整个文件被丢掉**（不是写一半）：
    // 半个 History 文件对平台是**损坏数据**，比没有更糟。
    //
    // ⛔ 关键不变量：截断后的归档**仍然是合法 zip**（能打开、能读）。
    // 损坏的归档 ⇒ 平台根本解不开 ⇒ 32 MiB 上限反而害事。
    let root = temp_dir("trunc");
    make_file(&root, "Default/History", &vec![0x5au8; 4096]);

    let (manifest, _) = scan_profile(&root, &ProfileSyncRules::default());
    let (entries, _) = build_entries(&manifest, &root);
    let archive = root.join("package.zip");
    let mut file = std::fs::File::create(&archive).expect("应能建归档");
    // 上限 1 KiB ⇒ 4 KiB 的文件装不下。
    let outcome = write_archive(&mut file, &entries, 1024).expect("应能写");
    drop(file);

    assert!(outcome.size_capped, "必须撞上限");
    assert_eq!(outcome.truncated, 1, "必须记一次截断");
    assert!(
        outcome.packed.is_empty(),
        "装不下的文件不许写进去：{:?}",
        outcome.packed
    );

    // ⛔ 关键：归档仍然合法（能打开）。
    let mut archive_file = std::fs::File::open(&archive).expect("应能打开归档");
    let zip = zip::ZipArchive::new(&mut archive_file).expect("截断后仍应是合法 zip");
    assert_eq!(zip.len(), 0, "装不下的文件不该有残留条目");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_file_that_grew_since_the_manifest_is_written_partially_but_the_archive_stays_readable() {
    // ⛔ 清单与归档之间文件变大了（TOCTOU）⇒ 条目只写进前半截。
    // 那种情况下**半个文件**已经在归档里了 ⇒ 归档必须仍然合法，
    // 否则平台会拿到一个打不开的包。
    let root = temp_dir("grew");
    make_file(&root, "Default/History", &vec![0x5au8; 4096]);

    let archive = root.join("package.zip");
    let mut file = std::fs::File::create(&archive).expect("应能建归档");
    // 清单里说它只有 10 字节（实际 4096）⇒ 归档层只写进 1024 字节。
    let entries = vec![ZipEntry {
        name: "Default/History".to_string(),
        source: root.join("Default/History"),
        size: 10,
    }];
    let outcome = write_archive(&mut file, &entries, 1024).expect("应能写");
    drop(file);

    assert!(outcome.size_capped, "必须撞上限");
    assert_eq!(outcome.truncated, 1, "必须记一次截断");
    assert_eq!(outcome.packed, vec!["Default/History".to_string()]);

    // ⛔ 关键：归档仍然合法，条目能读出来（是原文的前半截）。
    let mut archive_file = std::fs::File::open(&archive).expect("应能打开归档");
    let mut zip = zip::ZipArchive::new(&mut archive_file).expect("截断后仍应是合法 zip");
    assert_eq!(zip.len(), 1);
    let mut entry = zip.by_index(0).expect("应有条目");
    assert_eq!(entry.name().expect("应有条目名"), "Default/History");
    let mut got = Vec::new();
    std::io::Read::read_to_end(&mut entry, &mut got).expect("应能读完");
    assert!(!got.is_empty(), "截断后至少要有内容");
    assert!(got.len() <= 4096, "截断后的内容不许比原文长：{}", got.len());
    let _ = std::fs::remove_dir_all(&root);
}
