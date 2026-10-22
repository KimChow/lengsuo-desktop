//! 机器码**取值规则**的单元测试 + 跨仓取值互验向量组（任务 m m0-machine-code-align）。
//!
//! # 两部分
//!
//! 1. **单元测试**：每个平台的采集/折叠/解析函数在**假数据**下产出期望的字面量。
//!    ⛔ 不依赖真实机器的真实标识（CI 上没有那些路径）。
//! 2. **跨仓取值互验向量组**（[`RULE_VECTORS`]）：四台**假机器** ×
//!    (四个字段 + 两个派生摘要)。两侧各发布**同一张表**，
//!    内核仓的 `tools/cross_repo_machine.py` 逐行比较两侧的七个字段。
//!
//! # ⭐ 向量组为什么在**这个文件**而不是 `tests.rs`
//!
//! 任务书 §5.2 说「本卡在本仓 `kernel_pipe/tests.rs` 加对应的一组」，
//! 但任务书 §4.2 同时授权把 `machine` 模块拆成 `machine.rs` + `machine_tests.rs`
//! （`tests.rs` 当时已 586 行，本卡加进来的向量组会让它越过 700 行硬上限）。
//! ⇒ 互验工具 `tools/cross_repo_machine.py` 的 `CLIENT_VECTOR_CANDIDATES`
//! **四个候选文件里就有 `machine_tests.rs`**，两侧规则表仍逐字对齐。
//!
//! # ⭐ 摘要值是**硬编码的期望值**，不是运行时算出来的
//!
//! ⛔ 不许在测试里调用外部脚本生成期望值：那样两边都错也会「一致」。
//! 这六个摘要由**内核侧**的派生算法（契约 §2：
//! `SHA-256(UTF-16LE(固定键序 JSON))` → 十六进制 → `SHA-256(ASCII)`）算出，
//! 内核侧的同一组值在 `native/machine_facts_test.cc` 的 `kRuleVectors[]` 里。
//! ⇒ 下面的 `rule_vectors_recompute` 用**本仓**的
//! [`keys::machine_code`] / [`keys::k_machine`] 复算并断言相等。

use super::keys::{self, MachineInputs};
use super::machine::{
    fold_cpuid, parse_storage_descriptor, parse_storage_descriptor_size, select_first_serial,
    vendor_from_cpuid,
};

/// 一台**假机器**的取值规则向量：四个字段 + 两个派生摘要。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RuleVector {
    /// 向量名（只是给人看的标签，不参与派生）。
    pub(crate) label: &'static str,
    /// `os` 的取值。
    pub(crate) os: &'static str,
    /// `cpuid` 的取值。
    pub(crate) cpuid: &'static str,
    /// `hddid` 的取值。
    pub(crate) hddid: &'static str,
    /// `computername` 的取值。
    pub(crate) computername: &'static str,
    /// 由上面四个字段派生的 `machine_code`（十六进制小写）。
    pub(crate) machine_code: &'static str,
    /// 由 `machine_code` 派生的 `k_machine`（十六进制小写）。
    pub(crate) k_machine: &'static str,
}

/// 跨仓「取值规则互验」向量组（任务 m0-machine-code-align）。
///
/// ⭐ **两侧逐行复制这一组**：本仓在
/// `crates/lengsuo-desktop/src/kernel_pipe/machine_tests.rs`，
/// 内核侧在 `native/machine_facts_test.cc` 的 `kRuleVectors[]`。
/// 两边用**同一组固定输入**（全是假值 ⇒ 不含真实机器标识），
/// 各自算出的 `machine_code` / `k_machine` 必须**逐字节相同**。
///
/// ⭐ 四个向量覆盖 `os` 的三个标签（`linux` / `windows` / `macos`）加**全空**那一格
/// （「取不到就取空串」）。
pub(crate) const RULE_VECTORS: &[RuleVector] = &[
    RuleVector {
        label: "linux-demo",
        os: "linux",
        cpuid: "GenuineIntel/6/183/1",
        hddid: "DEMO-SERIAL-0001",
        computername: "demo-host",
        machine_code: "a64a48e92bef5ce24a4c3982bf33ea210a2dd5d0ad4fa5f23dc0172e7ee1409c",
        k_machine: "a917d3ac761be29fcea63dc80fadd30586ee21250ea3ea56e31f5b39f47f04ec",
    },
    RuleVector {
        label: "windows-demo",
        os: "windows",
        cpuid: "AuthenticAMD/25/33/0",
        hddid: "DEMO-SERIAL-0002",
        computername: "demo-win",
        machine_code: "550635314afdee4c9b6b07370c2f6b0760b5dd6bc39cae6741645a46e8fa0891",
        k_machine: "d401aa6686460d0cd36059fa0a1a05afd876414a8b4c632694e9d829e4ce5f95",
    },
    RuleVector {
        label: "macos-demo",
        os: "macos",
        cpuid: "Apple/0/0/0",
        hddid: "",
        computername: "demo-mac",
        machine_code: "38ac35734aedf6623c2c30c4513dbffd08f54e3442af7a51a75a5b8ad68024de",
        k_machine: "cde8543cfbfde4028579b96b39ee34cdd7bc2ca612c3c3b7eb67388014ceab3f",
    },
    RuleVector {
        label: "empty-demo",
        os: "",
        cpuid: "",
        hddid: "",
        computername: "",
        machine_code: "c93a0514938359dddef7414908ff15348d058d8fed1cd0a238bd2e72357d065c",
        k_machine: "0aea56c49b250fecbfe6c5cb4099987572519e5da82620eaf743be882815e684",
    },
];

// ---------------------------------------------------------------------------
// 跨仓取值互验
// ---------------------------------------------------------------------------

/// 本仓的派生必须复算出内核侧发布的那两个摘要。
///
/// ⭐ 这是「跨仓逐字节相同」在**本仓**可实跑的那一半：
/// 两侧用同一组固定输入，内核侧在自己的 gtest 里断言同样的两行，
/// `tools/cross_repo_machine.py` 再逐行比较两侧的**表**。
#[test]
fn rule_vectors_recompute_with_this_repo_derivation() {
    for vector in RULE_VECTORS {
        let inputs = MachineInputs {
            os: vector.os.to_string(),
            cpuid: vector.cpuid.to_string(),
            hddid: vector.hddid.to_string(),
            computername: vector.computername.to_string(),
        };
        let code = keys::machine_code(&inputs);
        assert_eq!(
            code, vector.machine_code,
            "rule vector {}: machine_code",
            vector.label
        );
        let key = keys::k_machine(&code);
        assert_eq!(
            keys::to_hex(&key),
            vector.k_machine,
            "rule vector {}: k_machine",
            vector.label
        );
    }
}

/// 向量组必须覆盖 `os` 的三个标签加**全空**那一格。
///
/// ⛔ 少一行 ⇒ 某个平台的取值格可能悄悄漂移而没人发现。
#[test]
fn rule_vectors_cover_every_platform_tag() {
    for tag in ["linux", "windows", "macos", ""] {
        assert!(
            RULE_VECTORS.iter().any(|v| v.os == tag),
            "no rule vector for os=\"{tag}\""
        );
    }
}

// ---------------------------------------------------------------------------
// cpuid 的折叠（R3）
// ---------------------------------------------------------------------------

/// `cpuid` 的折叠必须与内核侧 `base::CPU` **逐字同形**。
///
/// ⭐ `base::CPU` 的三个数是 Chromium `base/cpu.cc` 的
/// `ComputeX86FamilyAndModel` 折出来的；本仓用 `__cpuid` 复刻同一套规则。
/// 下面的 EAX 是**假数据**（按规则反推出来的），⛔ 不是本机 CPU 的值。
#[test]
fn fold_cpuid_matches_the_chromium_fold() {
    // 47 上的目标形态：family 6、model 183（base 0x7 + ext 0xb<<4）、stepping 1。
    assert_eq!(
        fold_cpuid("GenuineIntel", 0x000B_0671),
        "GenuineIntel/6/183/1"
    );
    // family 0x6 且 vendor 是 GenuineIntel ⇒ model 加 ext_model << 4。
    assert_eq!(
        fold_cpuid("GenuineIntel", 0x000A_06B1),
        "GenuineIntel/6/171/1"
    );
    // family 0x6 但**不是** GenuineIntel ⇒ 不加 ext_model。
    assert_eq!(
        fold_cpuid("AuthenticAMD", 0x000A_06B1),
        "AuthenticAMD/6/11/1"
    );
    // family 0xf ⇒ family 加 extended family（25 = 0xf + 0xa），model 加 ext_model << 4。
    assert_eq!(
        fold_cpuid("AuthenticAMD", 0x00A2_0F10),
        "AuthenticAMD/25/33/0"
    );
    // 非 x86 上 `base::CPU` 的取值：vendor "unknown"，三个数全 0。
    assert_eq!(fold_cpuid("unknown", 0), "unknown/0/0/0");
    assert_eq!(fold_cpuid("Apple", 0), "Apple/0/0/0");
}

/// vendor 串取 **EBX‖EDX‖ECX**（人可读顺序，⛔ 不是寄存器顺序）。
#[test]
fn vendor_from_cpuid_reads_ebx_edx_ecx_in_that_order() {
    // GenuineIntel 的三个寄存器：EBX="Genu"、EDX="ineI"、ECX="ntel"。
    let vendor = vendor_from_cpuid(0x756E_6547, 0x4965_6E69, 0x6C65_746E);
    assert_eq!(vendor, "GenuineIntel");
    // 顺序反了就拼出垃圾。
    let wrong = vendor_from_cpuid(0x6C65_746E, 0x4965_6E69, 0x756E_6547);
    assert_eq!(wrong, "ntelineIGenu");
}

// ---------------------------------------------------------------------------
// hddid：多盘选择规则（R1）
// ---------------------------------------------------------------------------

/// ⭐ R1：**排序 → 取第一个**（⛔ 不是「目录顺序的第一个」）。
#[test]
fn storage_serial_sorts_and_takes_the_first() {
    assert_eq!(
        select_first_serial(&[
            "SERIAL-B".to_string(),
            "SERIAL-A".to_string(),
            "SERIAL-C".to_string()
        ]),
        "SERIAL-A"
    );
    // 到达顺序不影响结果。
    assert_eq!(
        select_first_serial(&[
            "SERIAL-C".to_string(),
            "SERIAL-A".to_string(),
            "SERIAL-B".to_string()
        ]),
        "SERIAL-A"
    );
}

/// trim 在排序**之前** ⇒ 空白不参与比较。
#[test]
fn storage_serial_trims_before_choosing() {
    // "/sys/block/*/device/serial" 的文件以换行结尾。
    assert_eq!(
        select_first_serial(&["  SERIAL-A\n".to_string()]),
        "SERIAL-A"
    );
    assert_eq!(
        select_first_serial(&[" \tSERIAL-A \r\n".to_string()]),
        "SERIAL-A"
    );
    assert_eq!(
        select_first_serial(&[" SERIAL-B".to_string(), "SERIAL-A".to_string()]),
        "SERIAL-A"
    );
}

/// 空候选被丢掉；全空 ⇒ **空串**（⛔ 不许编造，硬约束 5）。
#[test]
fn storage_serial_drops_empty_candidates() {
    assert_eq!(
        select_first_serial(&[
            String::new(),
            "   ".to_string(),
            "\n".to_string(),
            "SERIAL-A".to_string()
        ]),
        "SERIAL-A"
    );
    assert_eq!(select_first_serial(&[]), "");
    assert_eq!(
        select_first_serial(&[String::new(), " ".to_string(), "\t\r\n".to_string()]),
        ""
    );
}

/// 排序是**按字节**的（⛔ 不是按 locale）。
#[test]
fn storage_serial_compares_bytes_not_case() {
    assert_eq!(
        select_first_serial(&["a".to_string(), "B".to_string(), "A".to_string()]),
        "A"
    );
}

// ---------------------------------------------------------------------------
// hddid：Windows 存储描述符的解析（纯函数，用构造的假字节）
// ---------------------------------------------------------------------------

/// 造一个**假**的 Windows 存储描述符（形状照驱动在真机上返回的那个）。
///
/// ⭐ 396 字节、序列号在偏移 90、驱动的前导空格——与内核侧
/// `machine_facts_test.cc` 的 `RealShapedDescriptor()` 同形 ⇒ 两侧测的是同一份形状。
/// ⚠️ 偏移落在缓冲外时**不写序列号**（内核侧那份也是这个形状：
/// 越界的偏移本身就是被测的场景之一）。
fn fake_descriptor(size: usize, serial_offset: u32, serial: &[u8]) -> Vec<u8> {
    let mut buffer = vec![0u8; size];
    buffer[4..8].copy_from_slice(&(size as u32).to_le_bytes());
    buffer[24..28].copy_from_slice(&serial_offset.to_le_bytes());
    let start = serial_offset as usize;
    if start + serial.len() <= size {
        buffer[start..start + serial.len()].copy_from_slice(serial);
    }
    buffer
}

/// 序列号在**偏移处**取（⛔ 偏移不是指针），且前导空格被 trim 掉。
#[test]
fn descriptor_serial_is_read_at_the_offset_and_trimmed() {
    let buffer = fake_descriptor(396, 90, b"        154310E1E661");
    assert_eq!(parse_storage_descriptor(&buffer), "154310E1E661");
}

/// ⭐ 偏移字段是 **DWORD**，⛔ 不是字节 —— 偏移 > 255 时按字节读就会读错。
#[test]
fn descriptor_serial_offset_is_a_32_bit_value() {
    let buffer = fake_descriptor(512, 300, b"SERIAL-PAST-255");
    assert_eq!(parse_storage_descriptor(&buffer), "SERIAL-PAST-255");
}

/// 尾部空白也要 trim 掉。
#[test]
fn descriptor_serial_trailing_space_is_trimmed_too() {
    let buffer = fake_descriptor(396, 90, b"154310E1E661   ");
    assert_eq!(parse_storage_descriptor(&buffer), "154310E1E661");
}

/// 驱动没有序列号（偏移 0）⇒ 空串。
#[test]
fn descriptor_serial_is_empty_when_the_driver_has_none() {
    let buffer = fake_descriptor(396, 0, b"");
    assert_eq!(parse_storage_descriptor(&buffer), "");
}

/// 偏移落在缓冲外 ⇒ 空串。
#[test]
fn descriptor_serial_is_empty_when_the_offset_falls_outside() {
    let buffer = fake_descriptor(396, 396, b"");
    assert_eq!(parse_storage_descriptor(&buffer), "");
    let buffer = fake_descriptor(396, 0xFFFF, b"");
    assert_eq!(parse_storage_descriptor(&buffer), "");
}

/// 描述符被截断（连头都读不全）⇒ 空串。
#[test]
fn descriptor_serial_is_empty_when_the_header_is_truncated() {
    assert_eq!(parse_storage_descriptor(&[0u8; 16]), "");
}

/// 驱动不一定在缓冲内 NUL 结尾 ⇒ 读到缓冲末尾。
#[test]
fn descriptor_serial_may_run_to_the_end_without_a_nul() {
    let buffer = fake_descriptor(128, 100, b"SERIAL-NO-NUL");
    assert_eq!(parse_storage_descriptor(&buffer), "SERIAL-NO-NUL");
}

/// 全是空格 ⇒ 空串。
#[test]
fn descriptor_serial_of_only_spaces_is_empty() {
    let mut buffer = fake_descriptor(128, 100, b"");
    buffer[100..].fill(b' ');
    assert_eq!(parse_storage_descriptor(&buffer), "");
}

/// 头部的 `Size` 字段（小端 DWORD，偏移 4）—— 两趟查询的第一趟靠它问大小。
#[test]
fn descriptor_size_is_read_back() {
    let buffer = fake_descriptor(396, 90, b"154310E1E661");
    assert_eq!(parse_storage_descriptor_size(&buffer), Some(396));
}

/// 头被截断 ⇒ 读不出大小。
#[test]
fn descriptor_size_rejects_a_truncated_header() {
    assert_eq!(parse_storage_descriptor_size(&[0u8; 7]), None);
}
