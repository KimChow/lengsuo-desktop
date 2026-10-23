//! 本机机器码素材的**真实值采集**（线协议 §1）。
//!
//! # 边界
//!
//! 本模块只做**取值**：四个字段各取什么值、怎么取、取不到怎么办。
//! ⛔ 不改**形状**：JSON 的键序与转义由 [`super::keys::MachineInputs`] 负责。
//! ⛔ 不改**派生**：`machine_code` / `k_machine` 的算法在 [`super::keys`] 里。
//!
//! # ⭐ 取值规则表（8 格，**两侧逐字相同**）
//!
//! 这张表与内核仓 `docs/l3-reports/task-M0.md` §1.1 的那张表**逐字相同**
//! （内核侧的实现见 `native/machine_facts.h` 与
//! `integration/kernel_pipe_client.cc` 的 `CollectMachineFacts`）。
//! 两侧必须采到**同一个值**，否则 `k_machine` 不同 ⇒ 内核解不开密钥信封
//! ⇒ 握手 fail-closed。
//!
//! | 字段 | 平台 | 取什么 | 用什么来源 | 拿不到时 |
//! |---|---|---|---|---|
//! | `os` | Windows | `"windows"` | 编译期平台常量 | 恒有 |
//! | `os` | POSIX | `"linux"` / `"macos"` | 编译期平台常量 | 其它 POSIX 平台 ⇒ **空串** |
//! | `cpuid` | Windows | `vendor/family/model/stepping` 斜杠拼接 | `base::CPU` | 恒有 |
//! | `cpuid` | POSIX | 同上 | `base::CPU` | 恒有 |
//! | `hddid` | Windows | 存储设备**序列号**（trim 后）| `\\.\PhysicalDrive0` 的 `IOCTL_STORAGE_QUERY_PROPERTY` 两趟查询 | **空串** |
//! | `hddid` | POSIX | 存储设备**序列号**（trim 后）| 扫 `/sys/block/*/device/serial` ⇒ 全部非空 → **排序** → 取第一个 | **空串** |
//! | `computername` | Windows | 主机名 | `GetComputerNameW` | **空串** |
//! | `computername` | POSIX | 主机名 | `gethostname()` | **空串** |
//!
//! 三条贯穿全表的规则：
//!
//! 1. **取值里永远不出现版本号**（R2）。
//! 2. **取不到就是空串，不许编造**（硬约束 5）。
//! 3. **取值是机器身份，不是会话指纹**。
//!
//! ## ⭐ 本仓（Rust）的落地方式
//!
//! 上表描述的是**规则**；本仓用什么语言设施拿到同一个值：
//!
//! | 格 | 本仓的落地 |
//! |---|---|
//! | `cpuid` | ⛔ 不链接 Chromium ⇒ 用 `core::arch::x86_64::__cpuid` **复刻** `base::CPU` 的折叠（`fold_cpuid()`，逐字照抄 `base/cpu.cc` 的 `ComputeX86FamilyAndModel`）⇒ 产出**同一形态** |
//! | `hddid`（Windows）| ⭐ 已实现：只开 `\\.\PhysicalDrive0`（与内核侧**同一块盘**）⇒ `IOCTL_STORAGE_QUERY_PROPERTY` 两趟查描述符 ⇒ 按 `SerialNumberOffset` 取值并 `trim()`。`unsafe` **只**在 [`win_storage`] 那一小段（crate 根由 `forbid` 改成 `deny`，见 `lib.rs` 的说明）|
//! | `computername` | ⚠️ **不是逐字同源**：内核 `gethostname()`，本仓 `COMPUTERNAME` / `HOSTNAME` 环境变量 → `/etc/hostname` ⇒ 语义冲突 **M-C** |
//!
//! ⚠️ macOS 的 `hddid` 一格内核侧无可移植来源 ⇒ 两侧都只能是**空串**
//! （语义冲突 **M-J**）。
//!
//! # ⭐ `cpuid` 的折叠规则（R3）
//!
//! 目标形态是 `vendor/family/model/stepping`（如 `GenuineIntel/6/183/1`），
//! 与内核侧的 `base::CPU` **逐字同形**。内核侧那三段数是 Chromium
//! `base/cpu.cc` 的 `ComputeX86FamilyAndModel` 折出来的：
//!
//! - `vendor`：CPUID leaf 0 的 EBX‖EDX‖ECX（**人可读顺序**，不是寄存器顺序）；
//! - `stepping`：leaf 1 的 EAX & 0xf；
//! - `family` / `model`：先取 base 值（`(eax >> 8) & 0xf` / `(eax >> 4) & 0xf`），
//!   再按规则叠加扩展值（family == 0xf 或（family == 0x6 且 vendor 是
//!   `GenuineIntel`）时 `model += ext_model << 4`；family == 0xf 时
//!   `family += ext_family`）。
//!
//! ⛔ **不得**读 `/proc/cpuinfo` 的 `model name`——那是人类可读型号名
//! （`GenuineIntel 13th Gen Intel(R) Core(TM) i9-13900K`），
//! 与 `base::CPU` 的形态**不可能逐字节相同**，4A 因此不成立。
//!
//! # 取不到怎么办
//!
//! ⛔ **取不到就取空串，不许编造**：空串 ⇒ 两侧 `k_machine` 不同 ⇒ 握手
//! fail-closed，这正是期望的失败方式。编一个值会让**从未绑定到本密钥的机器**
//! 也能打开信封。

use super::keys::MachineInputs;

/// 采集本机的机器码素材（四个字段）。
#[must_use]
pub fn collect_machine_inputs() -> MachineInputs {
    MachineInputs {
        os: collect_os(),
        cpuid: collect_cpuid(),
        hddid: collect_hddid(),
        computername: collect_computername(),
    }
}

/// 操作系统标识：`windows` / `linux` / `macos`（**小写、不带版本**）。
///
/// ⭐ **不带版本号**（R2）：内核一升级补丁版本，`k_machine` 就变
/// ⇒ 已绑定的密钥全部失效。
///
/// ⭐ 与内核侧的 `fp::PlatformOsTag` **逐字同形**：三个受支持平台给三个
/// 小写标识，**其它平台给空串**。
/// ⛔ 不是 Rust 的平台名（`std::env::consts::OS` 在 FreeBSD 上是
/// `"freebsd"`）—— 那一格内核侧给空串，两侧会算出不同的 `k_machine`。
fn collect_os() -> String {
    match std::env::consts::OS {
        "linux" => "linux",
        "windows" => "windows",
        "macos" => "macos",
        _ => "",
    }
    .to_string()
}

/// 计算机名：环境变量优先，退到 `/etc/hostname`。
fn collect_computername() -> String {
    for key in ["COMPUTERNAME", "HOSTNAME"] {
        if let Some(value) = env_non_empty(key) {
            return value;
        }
    }
    if let Ok(text) = std::fs::read_to_string("/etc/hostname") {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    String::new()
}

/// CPU 标识：`vendor/family/model/stepping`（十进制，斜杠拼接）。
///
/// ⭐ 折叠规则见模块文档：与内核侧的 `base::CPU` **逐字同形**（R3）。
fn collect_cpuid() -> String {
    #[cfg(target_arch = "x86_64")]
    {
        // `__cpuid` 在 Rust 1.99 里是**安全函数**（纯读 CPUID 指令）⇒ 本仓
        // 的 `#![forbid(unsafe_code)]` 不受影响。
        let leaf0 = core::arch::x86_64::__cpuid(0);
        // leaf 0 的 EAX 是「最大 leaf 号」；小于 1 ⇒ 没有 leaf 1。
        // Chromium 的 `base::CPU` 在这种情况下同样把 family/model/stepping 留成 0。
        let eax = if leaf0.eax < 1 {
            0
        } else {
            core::arch::x86_64::__cpuid(1).eax
        };
        // ⭐ 人可读顺序是 EBX‖EDX‖ECX（Chromium `base/cpu.cc` 明确换过 ECX/EDX）。
        let vendor = vendor_from_cpuid(leaf0.ebx, leaf0.edx, leaf0.ecx);
        fold_cpuid(&vendor, eax)
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        // 非 x86_64：内核侧 `base::CPU` 给的是 `unknown/0/0/0`
        // （`cpu_vendor_` 初值 `"unknown"`，family/model/stepping 初值 0）。
        // ⇒ 逐字同形，握手才对得上。
        UNKNOWN_CPUID.to_string()
    }
}

/// 非 x86_64 架构上 `base::CPU` 的取值（**逐字同形**）。
#[cfg(not(target_arch = "x86_64"))]
const UNKNOWN_CPUID: &str = "unknown/0/0/0";

/// CPUID leaf 0 的三个寄存器 ⇒ 人可读的 vendor 串。
///
/// ⚠️ **顺序是 EBX‖EDX‖ECX**，不是寄存器的自然顺序（ECX/EDX 要换位）。
/// Chromium 的 `base::CPU` 取的是交换后 16 字节数组的**后 12 字节**
/// ⇒ 与这里等价（都截到第一个 NUL）。
pub(crate) fn vendor_from_cpuid(ebx: u32, edx: u32, ecx: u32) -> String {
    let bytes = [
        ebx as u8,
        (ebx >> 8) as u8,
        (ebx >> 16) as u8,
        (ebx >> 24) as u8,
        edx as u8,
        (edx >> 8) as u8,
        (edx >> 16) as u8,
        (edx >> 24) as u8,
        ecx as u8,
        (ecx >> 8) as u8,
        (ecx >> 16) as u8,
        (ecx >> 24) as u8,
    ];
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

/// 把 CPUID leaf 1 的 EAX 折成 `vendor/family/model/stepping`。
///
/// ⭐ 折叠规则**逐字照抄** Chromium `base/cpu.cc` 的 `ComputeX86FamilyAndModel`
/// （内核侧 `base::CPU` 用的就是它）。
///
/// - `family` = `(eax >> 8) & 0xf`，`model` = `(eax >> 4) & 0xf`；
/// - `family == 0xf` 或（`family == 0x6` 且 vendor 是 `GenuineIntel`）时
///   `model += ((eax >> 16) & 0xf) << 4`；
/// - `family == 0xf` 时 `family += (eax >> 20) & 0xff`；
/// - `stepping` = `eax & 0xf`。
pub(crate) fn fold_cpuid(vendor: &str, eax: u32) -> String {
    let mut family = (eax >> 8) & 0xf;
    let mut model = (eax >> 4) & 0xf;
    if family == 0xf || (family == 0x6 && vendor == "GenuineIntel") {
        model += ((eax >> 16) & 0xf) << 4;
    }
    if family == 0xf {
        family += (eax >> 20) & 0xff;
    }
    let stepping = eax & 0xf;
    format!("{vendor}/{family}/{model}/{stepping}")
}

/// 存储设备序列号（**取用前 `trim()`**）。
///
/// - Linux：扫 `/sys/block/*/device/serial`，全部非空者**按字节序升序**取第一个。
/// - Windows：只开 `\\.\PhysicalDrive0`（与内核侧**同一块盘**）⇒ 两趟查
///   `IOCTL_STORAGE_QUERY_PROPERTY` ⇒ [`parse_storage_descriptor`] 取值。
/// - macOS：两仓都取不到 ⇒ 空串（⛔ 不许编造）。
fn collect_hddid() -> String {
    #[cfg(windows)]
    {
        // ⭐ 与内核侧**选同一块盘**：内核只开 `\\.\PhysicalDrive0` 一个句柄
        // （`WindowsStorageSerial()`）⇒ 本仓也只读 0 号盘，
        // ⛔ 不扫 `PhysicalDrive0..N` 再排序（那会选到不同的盘 ⇒ 语义冲突 M-G）。
        // 任何失败 ⇒ 空串 ⇒ 握手 fail-closed（期望的失败方式）。
        win_storage::storage_serial()
    }
    #[cfg(not(windows))]
    {
        select_sys_block_serial()
    }
}

/// 扫 `/sys/block/*/device/serial`，取**按字节序升序的第一个**非空值。
///
/// ⭐ **多盘选择规则必须与内核侧逐字相同**（R1：扫描范围 / 排序 / 取第一个）：
/// 客户端扫 `/sys/block` 下的**每一个**条目（⛔ 不按设备类型过滤），
/// 读它的 `device/serial`，`trim()` 后非空者入候选；候选**按字节序升序排序**
/// 后取第一个。
/// ⇒ 多盘机器上两侧若不照抄，会选到不同的盘 ⇒ 又一处跨机对不上。
fn select_sys_block_serial() -> String {
    let mut candidates: Vec<String> = Vec::new();
    let Ok(entries) = std::fs::read_dir("/sys/block") else {
        return String::new();
    };
    for entry in entries.flatten() {
        let path = entry.path().join("device").join("serial");
        if let Ok(text) = std::fs::read_to_string(&path) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                candidates.push(trimmed.to_string());
            }
        }
    }
    select_first_serial(&candidates)
}

/// 从候选序列号里按规则挑一个：**逐个 trim → 丢空 → 按字节序升序 → 取第一个**。
///
/// ⭐ R1 的核心：**排序 → 取第一个**（⛔ 不是「目录顺序的第一个」，
/// ⛔ 也不是「最长的一个」——两侧必须逐字照抄这条）。
/// ⭐ 规则与内核侧的 `fp::SelectStorageSerial` **逐字同形**。
#[must_use]
pub fn select_first_serial(candidates: &[String]) -> String {
    let mut usable: Vec<String> = candidates
        .iter()
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
        .collect();
    usable.sort_unstable();
    usable.into_iter().next().unwrap_or_default()
}

/// 从 Windows 的存储设备描述符里取序列号。
///
/// ⭐ `SerialNumberOffset` 是**缓冲区内的字节偏移**（⛔ 不是指针），
/// 且它是 **DWORD（4 字节小端）**，位置在描述符头的 **偏移 24**：
/// `Version(0) Size(4) DeviceType(8) DeviceTypeModifier(12)
/// RemovableMedia(16) CommandQueueing(20) VendorIdOffset(21)
/// ProductIdOffset(22) ProductRevisionOffset(23) SerialNumberOffset(24)
/// BusType(28) RawPropertiesLength(32) RawDeviceProperties(36)`。
/// ⛔ 只读一个字节在本机实测值（90）上**碰巧**也对（90 < 256）——
/// 但偏移 > 255 时就错 ⇒ 本仓按 DWORD 读。
/// ⭐ 内核侧把这两个偏移钉在 SDK 的真实结构上
/// （`static_assert(offsetof(STORAGE_DEVICE_DESCRIPTOR, SerialNumberOffset) == 24)`），
/// 本仓的偏移与之**逐字相同** ⇒ 两侧读的是同一个字段。
///
/// 序列号是零结尾的串，**驱动带前导空格** ⇒ 取用前 `trim()`。
///
/// ⭐ 这层是**纯函数**（不碰设备）⇒ Linux CI 也能用**构造的假描述符字节**测；
/// 读设备那层（`DeviceIoControl`）在 Windows 上才有意义。
#[must_use]
pub fn parse_storage_descriptor(descriptor: &[u8]) -> String {
    /// `STORAGE_DEVICE_DESCRIPTOR.SerialNumberOffset` 字段的字节偏移。
    const SERIAL_OFFSET_FIELD: usize = 24;
    if descriptor.len() < SERIAL_OFFSET_FIELD + 4 {
        return String::new();
    }
    let offset = u32::from_le_bytes([
        descriptor[SERIAL_OFFSET_FIELD],
        descriptor[SERIAL_OFFSET_FIELD + 1],
        descriptor[SERIAL_OFFSET_FIELD + 2],
        descriptor[SERIAL_OFFSET_FIELD + 3],
    ]) as usize;
    // 0 = 驱动没有序列号可报；越界 = 描述符被截断。
    if offset == 0 || offset >= descriptor.len() {
        return String::new();
    }
    let tail = &descriptor[offset..];
    let end = tail.iter().position(|b| *b == 0).unwrap_or(tail.len());
    trim_ascii(&tail[..end])
}

/// 读 Windows 存储描述符头里的 `Size` 字段（**小端 DWORD**，偏移 4）。
///
/// ⭐ 驱动**不会**回 `ERROR_INSUFFICIENT_BUFFER` 报告所需大小，而是**成功返回**
/// 并把真实大小写进头部的 `Size` 字段 ⇒ 先用小缓冲问大小，再按大小读。
#[must_use]
pub fn parse_storage_descriptor_size(header: &[u8]) -> Option<u32> {
    /// `STORAGE_DEVICE_DESCRIPTOR.Size` 字段的字节偏移。
    const SIZE_FIELD: usize = 4;
    if header.len() < SIZE_FIELD + 4 {
        return None;
    }
    Some(u32::from_le_bytes([
        header[SIZE_FIELD],
        header[SIZE_FIELD + 1],
        header[SIZE_FIELD + 2],
        header[SIZE_FIELD + 3],
    ]))
}

/// 去掉首尾的 ASCII 空白（与内核侧 `TrimAscii` 逐字同形）。
fn trim_ascii(bytes: &[u8]) -> String {
    let is_space = |b: u8| matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c);
    let start = bytes
        .iter()
        .position(|b| !is_space(*b))
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|b| !is_space(*b))
        .map_or(start, |i| i + 1);
    String::from_utf8_lossy(&bytes[start..end]).into_owned()
}

/// 取一个非空的环境变量。
fn env_non_empty(key: &str) -> Option<String> {
    let value = std::env::var(key).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

// ---------------------------------------------------------------------------
// Windows：读 `\\.\PhysicalDrive0` 的存储序列号
// ---------------------------------------------------------------------------

/// **唯一**打开的盘：`\\.\PhysicalDrive0`。
///
/// ⭐ **与内核侧 `WindowsStorageSerial()` 开同一块盘**（它也只开 0 号盘）
/// ⇒ ⛔ 不许扫 `PhysicalDrive0..N` 再排序：那会选到**另一块**盘
/// ⇒ 两侧 `hddid` 不同 ⇒ `k_machine` 不同 ⇒ 握手 fail-closed（语义冲突 M-G）。
///
/// ⚠️ 这个原始串字面量与普通串 `"\\\\.\\PhysicalDrive0"` 是**同一个串**
/// （都是两个反斜杠 + 点 + 反斜杠 + `PhysicalDrive0`），见 `machine_tests.rs`。
pub const PHYSICAL_DRIVE_PATH: &str = r"\\.\PhysicalDrive0";

/// 描述符输出缓冲的**首趟**字节数。
///
/// ⭐ 真实 `STORAGE_DEVICE_DESCRIPTOR` 约 396 字节，但**没有**保证的上限
/// ⇒ 给足 1024；驱动报回的 `Size` 比它大时再按报告的大小读第二趟
/// （见 [`second_pass_len`]）。
/// ⚠️ 上一轮实测：只给 36 字节的缓冲会让描述符越界。
pub const DESCRIPTOR_BUFFER_LEN: usize = 1024;

/// 描述符缓冲的**上限**（第二趟的防线）。
///
/// ⛔ 驱动报回的 `Size` 是**未校验的外部输入**：照抄分配
/// （`vec![0u8; 0xFFFF_FFFF]`）会直接 OOM abort。超过上限 ⇒ 放弃 ⇒
/// 返回空串（fail-closed，正是期望的失败方式）。
pub const MAX_DESCRIPTOR_LEN: usize = 64 * 1024;

/// 首趟缓冲读完后，**还需不需要**按驱动报告的大小再读一趟。
///
/// - 报回的大小 ≤ 首趟缓冲 ⇒ `None`（首趟里就是完整描述符）；
/// - 报回的大小更大、且不超 [`MAX_DESCRIPTOR_LEN`] ⇒ `Some(该大小)`；
/// - 读不出大小 / 大小超上限 ⇒ `None`（不再读，由调用方判空）。
///
/// ⭐ 纯函数（不碰设备）⇒ Linux 上也能测。
#[must_use]
pub fn second_pass_len(first_pass_len: usize, reported: Option<u32>) -> Option<usize> {
    let reported = reported? as usize;
    if reported > first_pass_len && reported <= MAX_DESCRIPTOR_LEN {
        Some(reported)
    } else {
        None
    }
}

/// Windows 存储设备序列号的 **FFI 段** —— 本 crate 里**唯一**允许 `unsafe` 的地方。
///
/// # 为什么这一段需要 `unsafe`
///
/// 读物理盘序列号**只能**走 `DeviceIoControl`。内核侧本来就是这么做的
/// （`WindowsStorageSerial()`）⇒ 两边对称才能取到**同一个值**。
/// 2026-10-24 主理人裁定：crate 根的 `forbid` 改成 `deny`，例外**只**开到这一段。
///
/// # 作用范围（为什么不能再小 / 再大）
///
/// - 本模块**只有** FFI：三个 `extern` 声明 + 七个常量 + 一个 `#[repr(C)]`
///   输入结构 + 两个调用函数。模块内**没有**任何别的逻辑
///   ⇒ 模块边界就是**审计边界**：读 `#[allow(unsafe_code)]` 这一行就知道
///   `unsafe` 最远能到哪。
/// - 模块**之外**（`machine.rs` 其余部分、整个 crate）仍是 `deny`
///   ⇒ 任何未标注的 `unsafe` 照旧是**编译错误**。
/// - 两个调用函数都必须在这个模块里：一个开句柄、一个发 IOCTL，都直接触
///   `unsafe`；逐函数各标一次只会让例外**散开**、更容易漏标新加的调用。
///
/// ⛔ 不许把这个 `allow` 挪到 `machine.rs` 顶部或 `lib.rs`。
#[cfg(windows)]
#[allow(unsafe_code)]
mod win_storage {
    use std::ffi::c_void;
    use std::mem::size_of;

    // `CreateFileW` / `DeviceIoControl` / `CloseHandle` 的宿主 DLL。
    //
    // ⭐ 手写声明，⛔ 不引 `windows-sys` / `winapi` / `windows` crate（本卡零新依赖）。
    // ⚠️ 这里用 `//` 而不是 `///`：rustdoc 不给 `extern` 块生成文档
    // ⇒ `///` 会触发 `unused_doc_comments`（`-D warnings` 下直接失败）。
    #[link(name = "kernel32")]
    extern "system" {
        fn CreateFileW(
            lp_file_name: *const u16,
            dw_desired_access: u32,
            dw_share_mode: u32,
            lp_security_attributes: *mut c_void,
            dw_creation_disposition: u32,
            dw_flags_and_attributes: u32,
            h_template_file: *mut c_void,
        ) -> *mut c_void;

        fn DeviceIoControl(
            h_device: *mut c_void,
            dw_io_control_code: u32,
            lp_in_buffer: *mut c_void,
            n_in_buffer_size: u32,
            lp_out_buffer: *mut c_void,
            n_out_buffer_size: u32,
            lp_bytes_returned: *mut u32,
            lp_overlapped: *mut c_void,
        ) -> i32;

        fn CloseHandle(h_object: *mut c_void) -> i32;
    }

    /// `GENERIC_READ`。
    const GENERIC_READ: u32 = 0x8000_0000;
    /// `FILE_SHARE_READ | FILE_SHARE_WRITE` —— ⛔ 独占打开会让别的程序读不到盘。
    const FILE_SHARE_READ_WRITE: u32 = 0x0000_0003;
    /// `OPEN_EXISTING`。
    const OPEN_EXISTING: u32 = 3;
    /// `CreateFileW` 失败的返回值。
    const INVALID_HANDLE_VALUE: *mut c_void = -1isize as *mut c_void;
    /// `IOCTL_STORAGE_QUERY_PROPERTY`（`CTL_CODE(0x2d, 0x500, METHOD_BUFFERED, FILE_ANY_ACCESS)`）。
    const IOCTL_STORAGE_QUERY_PROPERTY: u32 = 0x002D_1400;
    /// `StorageDeviceProperty`。
    const STORAGE_DEVICE_PROPERTY: u32 = 0;
    /// `PropertyStandardQuery`。
    const PROPERTY_STANDARD_QUERY: u32 = 0;

    /// `STORAGE_PROPERTY_QUERY`（SDK 结构，⛔ 字段顺序不许改）。
    ///
    /// ⚠️ `AdditionalParameters` 是 **1 字节数组**，但 `repr(C)` 会补到 4 字节对齐
    /// ⇒ `size_of::<Self>() == 12`，与内核侧的 `sizeof(STORAGE_PROPERTY_QUERY)` 相同。
    /// ⛔ 输入缓冲必须**真的传**：不传时 `DeviceIoControl` 直接失败
    /// （上一轮实测 `GetLastError() == 24`，`ERROR_BAD_LENGTH`）。
    #[repr(C)]
    struct StoragePropertyQuery {
        property_id: u32,
        query_type: u32,
        additional_parameters: [u8; 1],
    }

    /// 打开 [`super::PHYSICAL_DRIVE_PATH`] 并读它的存储序列号；任何失败 ⇒ 空串。
    pub(super) fn storage_serial() -> String {
        // ⚠️ 宽字符串**必须** NUL 结尾。
        let mut path: Vec<u16> = super::PHYSICAL_DRIVE_PATH.encode_utf16().collect();
        path.push(0);
        // SAFETY: `path` 是 NUL 结尾的宽字符串；以只读共享方式打开；句柄在
        // 下面由 `CloseHandle` 关掉，不泄漏。
        let handle = unsafe {
            CreateFileW(
                path.as_ptr(),
                GENERIC_READ,
                FILE_SHARE_READ_WRITE,
                core::ptr::null_mut(),
                OPEN_EXISTING,
                // 物理盘不需要任何文件属性 ⇒ 0（⛔ 不影响取到的序列号）。
                0,
                core::ptr::null_mut(),
            )
        };
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            return String::new();
        }
        let serial = read_descriptor_serial(handle);
        // SAFETY: `handle` 由上面的 `CreateFileW` 成功返回，且只关一次。
        unsafe { CloseHandle(handle) };
        serial
    }

    /// 两趟查描述符并解析出序列号（两趟的判据见 [`super::second_pass_len`]）。
    fn read_descriptor_serial(handle: *mut c_void) -> String {
        let mut buffer = vec![0u8; super::DESCRIPTOR_BUFFER_LEN];
        if !query(handle, &mut buffer) {
            return String::new();
        }
        let reported = super::parse_storage_descriptor_size(&buffer);
        if let Some(size) = super::second_pass_len(buffer.len(), reported) {
            let mut bigger = vec![0u8; size];
            if !query(handle, &mut bigger) {
                return String::new();
            }
            buffer = bigger;
        }
        super::parse_storage_descriptor(&buffer)
    }

    /// 一次 `IOCTL_STORAGE_QUERY_PROPERTY` 调用；失败 ⇒ `false`。
    fn query(handle: *mut c_void, buffer: &mut [u8]) -> bool {
        let input = StoragePropertyQuery {
            property_id: STORAGE_DEVICE_PROPERTY,
            query_type: PROPERTY_STANDARD_QUERY,
            additional_parameters: [0],
        };
        let mut returned: u32 = 0;
        // SAFETY: `input` 是活的、repr(C) 的 `STORAGE_PROPERTY_QUERY`，长度如实
        // 传给 `n_in_buffer_size`；`buffer` 是本函数的独占切片，长度如实传给
        // `n_out_buffer_size`；`handle` 由调用方保证是刚打开的有效句柄。
        // `DeviceIoControl` 只写 `buffer` 与 `returned`（都归本函数所有）。
        let ok = unsafe {
            DeviceIoControl(
                handle,
                IOCTL_STORAGE_QUERY_PROPERTY,
                (&input as *const StoragePropertyQuery)
                    .cast_mut()
                    .cast::<c_void>(),
                size_of::<StoragePropertyQuery>() as u32,
                buffer.as_mut_ptr().cast::<c_void>(),
                buffer.len() as u32,
                &mut returned,
                core::ptr::null_mut(),
            )
        };
        ok != 0
    }
}
