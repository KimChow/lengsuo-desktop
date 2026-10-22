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
//! | `hddid`（Windows）| ⛔ 本卡未实现读设备层（`DeviceIoControl` 要 `unsafe`，而本仓 crate 根 `#![forbid(unsafe_code)]`）⇒ 先取**空串**；解析层已写好并有单测 |
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
/// - Windows：**占位**（本仓禁 `unsafe`，Win32 设备读层留到下一张卡，见报告）。
/// - macOS：两仓都取不到 ⇒ 空串（⛔ 不许编造）。
fn collect_hddid() -> String {
    #[cfg(windows)]
    {
        // ⛔ 本卡未实现：取物理盘序列号要 `DeviceIoControl`（`unsafe`），
        // 而本仓 crate 根有 `#![forbid(unsafe_code)]`（`lib.rs`）。
        // ⇒ 与 `launch.rs` / `server.rs` 的 Windows 占位同一处置：先取空串
        // （⇒ 握手 fail-closed，期望的失败方式），下一张卡补上。
        //
        // ⭐ 补上时必须与内核侧**选同一块盘**：内核只开 `\\.\PhysicalDrive0`
        // 一个句柄（`WindowsStorageSerial()`）⇒ 本仓也只读 0 号盘，
        // ⛔ 不扫 `PhysicalDrive0..N` 再排序（那会选到不同的盘 ⇒ 语义冲突 M-G）。
        String::new()
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
