//! profile 同步的**收集规则**：白名单、黑名单、单包上限（**纯逻辑**，不碰 IO）。
//!
//! # 规则写在配置里，不写死在代码里
//!
//! 平台下发什么就收集什么。本模块只提供**默认值**（拍板 1B 的那一套）
//! 与**解析器**，不替平台决定收集什么。
//!
//! ⛔ 但有一块**地板**不是配置说了算：`Cookies` / `Network Cache`
//! **任何情况下都不外带**。配置是远端输入 —— 平台若把 cookies 写进白名单，
//! 也不能真的发出去。内核侧 `profile_bundle.cc` 有同一组常量
//! （`kForbiddenCookies` / `kForbiddenNetwork`）⇒ 两侧的地板是同一块。
//!
//! # 与内核侧的对应关系
//!
//! | 本模块 | 内核侧（`native/profile_bundle.*`） |
//! |---|---|
//! | [`ProfileSyncRules`] | `ProfileSyncConfig` |
//! | [`ProfileSyncRules::default`] | `DefaultProfileSyncConfig()` |
//! | [`ProfileSyncRules::from_config_bundle`] | `ParseProfileSyncConfig()` |
//! | [`match_glob`] | `MatchGlob()` |
//! | [`never_goes_out`] | `kForbiddenCookies` / `kForbiddenNetwork` |
//!
//! ⇒ 两侧对**同一份 JSON** 必须给出**同一个答案**（同样的白名单、同样的
//! 黑名单、同样的上限、同样的「畸形就是畸形」）。
//!
//! # 拍板
//!
//! - **1B**：浏览历史**默认上传** ⇒ 默认白名单含 `*History*`。
//!   ⚠️ 代价：浏览历史默认上云，是隐私事故的高发点；163,840 B/包的体量
//!   会拖慢关闭流程。上线前应补一条隐私提示。
//! - **2A**：上传失败**不阻塞**容器关闭 —— 本模块不参与，见 [`crate::kernel_pipe::upload`]。
//! - **3A**：`data_url` 默认不采 —— 本模块**不产生**任何 `data_url`。

/// 单包上限：**32 MiB**。
///
/// 与内核侧 `kProfileBundleMaxBytes` 是同一个值。超限**截断**并计数：
/// 一个 40 MB 的包不该让容器关不掉。配置只许调低，不许调高。
pub const MAX_PACKAGE_BYTES: u64 = 32 * 1024 * 1024;

/// 配置里的 `profile_sync` 段名。
///
/// 与内核侧 `kProfileSyncConfigKey` 逐字节相同。
pub const PROFILE_SYNC_KEY: &str = "profile_sync";

/// 永远不外带的两类文件（**地板**，不是偏好）。
///
/// 与内核侧 `kForbiddenCookies` / `kForbiddenNetwork` 逐字节相同。
const FORBIDDEN: [&str; 2] = ["*Cookies*", "*Network*"];

/// **地板**：这个路径**任何情况下都不外带**。
///
/// 清单是内核侧产的（那边也有一层地板），客户端这一层再挡一次：
/// 两层地板比一层可靠 —— 清单是远端输入，客户端不该假设它干净。
#[must_use]
pub fn never_goes_out(path: &str) -> bool {
    FORBIDDEN.iter().any(|pattern| match_glob(pattern, path))
}

/// 规则层的失败原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RulesError {
    /// 配置包畸形（不是「部分遵守」，是**拒绝**）。
    MalformedConfig,
}

/// 收集规则（白名单 + 黑名单 + 单包上限）。
///
/// 白名单**写在配置里**，不写死在代码里：平台说什么就收集什么。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileSyncRules {
    /// 必须匹配至少一条才收集（白名单）。
    pub include: Vec<String>,
    /// 匹配任意一条就**不**收集（黑名单，优先于白名单）。
    pub exclude: Vec<String>,
    /// 单包字节上限。`0` 表示用 [`MAX_PACKAGE_BYTES`]。
    pub max_package_bytes: u64,
}

impl Default for ProfileSyncRules {
    /// 默认集合 = 白名单减黑名单 **+ History**（拍板 1B）。
    ///
    /// ⛔ 这不是「代码里的白名单」：它是**配置的默认值**，平台下发别的
    /// 就用别的。内核侧 `DefaultProfileSyncConfig()` 是同一份常量，
    /// 两边靠契约保持一致。
    fn default() -> Self {
        Self {
            include: vec![
                "*Local Storage*".to_string(),
                "*Extension State*".to_string(),
                "*History*".to_string(),
            ],
            exclude: vec![
                "*Cache*".to_string(),
                "*Cookies*".to_string(),
                "*Network*".to_string(),
            ],
            max_package_bytes: MAX_PACKAGE_BYTES,
        }
    }
}

impl ProfileSyncRules {
    /// 一份**什么都不收**的规则。
    ///
    /// 平台没要求做 profile 同步时用它：白名单空 ⇒ 一个文件都不收
    /// （内核侧 `CollectProfileBundle` 对空白名单的结论一样）。
    ///
    /// ⛔ 不用「不发 `profile_sync` 段」来表示「不收」：内核侧
    /// `ParseProfileSyncConfig` 把**缺段**当失败 ⇒ 那会让内核报错，
    /// 而「不收」不是错误。
    #[must_use]
    pub fn collect_nothing() -> Self {
        Self {
            include: Vec::new(),
            exclude: Vec::new(),
            max_package_bytes: MAX_PACKAGE_BYTES,
        }
    }

    /// 从一份配置包（JSON）里读出 `profile_sync` 段。
    ///
    /// 配置包里**没有**这一段 ⇒ 返回 `Ok(None)`：平台没有要求做 profile
    /// 同步 ⇒ 什么都不收集。段存在但畸形 ⇒ 返回 `Err`。
    ///
    /// ⛔ 解析出来的规则**不会**被悄悄改写：这一段是远端输入，畸形就是
    /// 畸形，不做「部分遵守」。上限只许调低，不许调高。
    ///
    /// # ⚠️ 与内核侧的一处**形状**差异（效果相同）
    ///
    /// 内核侧 `ParseProfileSyncConfig` 把「**缺 `profile_sync` 段**」判成
    /// **失败**（`"the bundle carries no profile_sync section"`），本函数
    /// 返回 `Ok(None)`。
    ///
    /// - **效果相同**：两侧都是「不收集任何文件」。
    /// - **形状不同**：内核 = 错误，客户端 = `Ok(None)`。
    ///
    /// 为什么这样分：内核侧是**接收方**，它拿到一份没有 `profile_sync` 段的
    /// 配置包 ⇒ 那是平台没要求同步 ⇒ 报错会把「没要求」说成「坏了」。
    /// 客户端侧是**发起方**，它自己知道有没有要求同步 ⇒ 缺段就是「不要求」。
    ///
    /// ⛔ 段**存在**但畸形时两侧一致（都拒绝）：「一份读不出来的配置不是
    /// 一份被部分遵守的配置」。
    pub fn from_config_bundle(bundle_json: &str) -> Result<Option<Self>, RulesError> {
        let value: serde_json::Value =
            serde_json::from_str(bundle_json).map_err(|_| RulesError::MalformedConfig)?;
        let Some(section) = value.get(PROFILE_SYNC_KEY) else {
            return Ok(None);
        };
        let object = section.as_object().ok_or(RulesError::MalformedConfig)?;
        // ⛔ 缺 `include` ⇒ 失败（不是「空白名单」）：内核侧同样如此。
        let include = read_patterns(Some(
            object.get("include").ok_or(RulesError::MalformedConfig)?,
        ))?;
        let exclude = read_patterns(object.get("exclude"))?;
        let max_package_bytes = match object.get("max_package_bytes") {
            None => MAX_PACKAGE_BYTES,
            // ⛔ `null` 是畸形，不是「没设」：内核侧的 `GetInt64` 读不出
            // 一个整数 ⇒ 它把这一段判成 malformed。
            Some(value) => {
                let number = value.as_u64().ok_or(RulesError::MalformedConfig)?;
                if number == 0 {
                    MAX_PACKAGE_BYTES
                } else {
                    // 配置可以调低，调不高。
                    number.min(MAX_PACKAGE_BYTES)
                }
            }
        };
        Ok(Some(Self {
            include,
            exclude,
            max_package_bytes,
        }))
    }

    /// 这个路径收不收。
    ///
    /// ⛔ 地板先判：`Cookies` / `Network` 在**白名单之前**就被挡掉，
    /// 所以远端配置把它们写进白名单也没用。
    /// 之后是黑名单优先于白名单。
    #[must_use]
    pub fn allows(&self, path: &str) -> bool {
        if never_goes_out(path) {
            return false;
        }
        if !self.include.iter().any(|pattern| match_glob(pattern, path)) {
            return false;
        }
        !self.exclude.iter().any(|pattern| match_glob(pattern, path))
    }

    /// 生效的上限（配置只许调低）。
    #[must_use]
    pub fn ceiling(&self) -> u64 {
        if self.max_package_bytes == 0 {
            MAX_PACKAGE_BYTES
        } else {
            self.max_package_bytes.min(MAX_PACKAGE_BYTES)
        }
    }
}

/// 读一个模式数组。非数组 ⇒ 畸形。
fn read_patterns(value: Option<&serde_json::Value>) -> Result<Vec<String>, RulesError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let array = value.as_array().ok_or(RulesError::MalformedConfig)?;
    let mut patterns = Vec::with_capacity(array.len());
    for item in array {
        let pattern = item.as_str().ok_or(RulesError::MalformedConfig)?;
        patterns.push(pattern.to_string());
    }
    Ok(patterns)
}

/// 匹配一个 glob 模式。
///
/// `*` 匹配任意长度的字节串（`/` 也算），`?` 匹配**一个**字节。
/// 比较是**按字节**且**区分大小写**的：profile 的目录名在两个平台
/// 上拼写一致，而按 locale 比较会让两仓算出的清单不一样。
///
/// ⚠️ 与内核侧 `MatchGlob` 是同一套算法（逐字节、贪心回溯）。
#[must_use]
pub fn match_glob(pattern: &str, text: &str) -> bool {
    let pattern = pattern.as_bytes();
    let text = text.as_bytes();
    let (mut p, mut t) = (0usize, 0usize);
    let mut star: Option<usize> = None;
    let mut star_end = 0usize;
    while t < text.len() {
        if p < pattern.len() && (pattern[p] == b'?' || pattern[p] == text[t]) {
            p += 1;
            t += 1;
        } else if p < pattern.len() && pattern[p] == b'*' {
            star = Some(p);
            p += 1;
            star_end = t;
        } else if let Some(star_at) = star {
            p = star_at + 1;
            star_end += 1;
            t = star_end;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == b'*' {
        p += 1;
    }
    p == pattern.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 规则速写。
    fn rules(include: &[&str], exclude: &[&str]) -> ProfileSyncRules {
        ProfileSyncRules {
            include: include.iter().map(|s| (*s).to_string()).collect(),
            exclude: exclude.iter().map(|s| (*s).to_string()).collect(),
            max_package_bytes: MAX_PACKAGE_BYTES,
        }
    }

    // -- 1B：默认集合含 History --------------------------------------------

    #[test]
    fn the_default_set_contains_history() {
        // 拍板 1B：History 默认上传。
        let rules = ProfileSyncRules::default();
        assert!(
            rules.include.iter().any(|p| p == "*History*"),
            "默认白名单必须含 History：{:?}",
            rules.include
        );
        assert!(rules.allows("Default/History"));
        // ⚠️ 1B 的代价：浏览历史默认上云（隐私事故高发点）。
        // 这里只把「默认集合确实包含 History」钉死。
    }

    #[test]
    fn the_history_scope_is_still_bounded_by_the_patterns() {
        // ⛔ History 的收集范围仍受通配规则约束，不是无脑全量。
        let rules = ProfileSyncRules::default();
        assert!(!rules.allows("Default/Sessions/Tabs"));
        assert!(!rules.allows("Default/Bookmarks"));
    }

    #[test]
    fn the_default_lists_match_the_kernel_side_one() {
        // ⛔ 两侧的默认常量必须逐字节相同（内核侧 `DefaultProfileSyncConfig`）。
        let rules = ProfileSyncRules::default();
        assert_eq!(
            rules.include,
            vec![
                "*Local Storage*".to_string(),
                "*Extension State*".to_string(),
                "*History*".to_string(),
            ]
        );
        assert_eq!(
            rules.exclude,
            vec![
                "*Cache*".to_string(),
                "*Cookies*".to_string(),
                "*Network*".to_string(),
            ]
        );
        assert_eq!(rules.max_package_bytes, MAX_PACKAGE_BYTES);
    }

    // -- 白名单命中 / 黑名单排除 --------------------------------------------

    #[test]
    fn the_white_list_hits_and_the_black_list_excludes() {
        let rules = ProfileSyncRules::default();
        assert!(rules.allows("Default/Local Storage/leveldb/000003.log"));
        assert!(rules.allows("Default/Extension State/1.dat"));
        // 黑名单优先于白名单。
        assert!(!rules.allows("Default/Local Storage/Cache/x.db"));
    }

    #[test]
    fn cookies_and_network_cache_never_go_out_under_any_circumstance() {
        // ⛔ 地板：即便远端配置把它们写进白名单也不带。
        let mut rules = rules(&["*"], &[]);
        assert!(rules.allows("Default/Sessions/Tabs"), "夹具前提");
        assert!(!rules.allows("Default/Cookies/db.sqlite"));
        assert!(!rules.allows("Default/Network/Cache/data_1"));
        assert!(!rules.allows("Default/Network/Network Persistent State"));
        // 地板也直接挡 `Network Cache` 这个名字。
        assert!(never_goes_out("Default/Network Cache/x"));
        rules.include.push("*Cookies*".to_string());
        assert!(!rules.allows("Default/Cookies/db.sqlite"));
    }

    #[test]
    fn an_empty_white_list_collects_nothing() {
        // 与内核侧 `AnEmptyWhiteListCollectsNothing` 同一结论。
        let rules = ProfileSyncRules::collect_nothing();
        assert!(!rules.allows("Default/History"));
        assert!(!rules.allows("Default/Local Storage/x"));
    }

    // -- 配置解析：与内核侧 `ParseProfileSyncConfig` 同一答案 ----------------

    #[test]
    fn the_section_the_runner_sends_parses_back_into_the_same_rules() {
        // 两侧对同一份 JSON 必须给出同一个答案（内核侧
        // `ParseProfileSyncConfig` 是这份载荷的消费者）。
        let bundle = r#"{"profile_sync":{"include":["*History*","*Local Storage*"],"exclude":["*Cache*","*Cookies*","*Network*"],"max_package_bytes":1048576}}"#;
        let rules = ProfileSyncRules::from_config_bundle(bundle)
            .expect("应能解析")
            .expect("应有 profile_sync 段");
        assert_eq!(rules.include, vec!["*History*", "*Local Storage*"]);
        assert_eq!(rules.exclude, vec!["*Cache*", "*Cookies*", "*Network*"]);
        assert_eq!(rules.max_package_bytes, 1_048_576);
    }

    #[test]
    fn the_ceiling_is_never_raised_by_the_configuration() {
        // 配置只许调低，不许调高（内核侧同一条规则）。
        let bundle = r#"{"profile_sync":{"include":["*History*"],"max_package_bytes":999999999}}"#;
        let rules = ProfileSyncRules::from_config_bundle(bundle)
            .expect("应能解析")
            .expect("应有 profile_sync 段");
        assert_eq!(rules.ceiling(), MAX_PACKAGE_BYTES);
        // 调低是可以的。
        let bundle = r#"{"profile_sync":{"include":["*History*"],"max_package_bytes":1024}}"#;
        let rules = ProfileSyncRules::from_config_bundle(bundle)
            .expect("应能解析")
            .expect("应有 profile_sync 段");
        assert_eq!(rules.ceiling(), 1024);
    }

    #[test]
    fn a_bundle_without_the_profile_sync_section_collects_nothing() {
        // 平台没要求做 profile 同步 ⇒ 不收集（不是「用默认规则」）。
        let rules =
            ProfileSyncRules::from_config_bundle(r#"{"something_else":1}"#).expect("应能解析");
        assert!(rules.is_none());
        // ⛔ 缺段**不是**「用默认规则」：默认规则含 History（1B），
        // 那会把平台没要求的东西发出去。
        let nothing = ProfileSyncRules::collect_nothing();
        assert!(!nothing.allows("Default/History"));
    }

    #[test]
    fn a_malformed_section_is_refused_not_partially_obeyed() {
        // 畸形就是畸形，不做「部分遵守」。
        for bundle in [
            "{not json",
            r#"{"profile_sync":{"include":"not an array"}}"#,
            r#"{"profile_sync":{"exclude":["*Cache*"]}}"#,
            r#"{"profile_sync":{"include":["*History*"],"max_package_bytes":-1}}"#,
            r#"{"profile_sync":{"include":["*History*"],"max_package_bytes":null}}"#,
            r#"{"profile_sync":{"include":["*History*"],"exclude":"not an array"}}"#,
            r#"{"profile_sync":"not an object"}"#,
        ] {
            assert!(
                ProfileSyncRules::from_config_bundle(bundle).is_err(),
                "畸形配置包 {bundle} 必须被拒绝"
            );
        }
    }

    // -- glob ---------------------------------------------------------------

    #[test]
    fn the_glob_matching_is_byte_wise_and_case_sensitive() {
        assert!(match_glob("*History*", "Default/History/x"));
        assert!(
            !match_glob("*History*", "Default/history"),
            "大小写敏感 ⇒ 不匹配"
        );
        assert!(match_glob("a?c", "abc"));
        assert!(!match_glob("a?c", "ac"));
        // `*` 跨 `/`（内核侧同一语义：`*` 匹配任意长度的字节串，`/` 也算）。
        assert!(match_glob("*Local Storage*", "Default/Local Storage/x/y"));
        assert!(match_glob("*.log", "a/b.log"));
    }

    #[test]
    fn the_glob_agrees_with_the_kernel_side_cases() {
        // 与内核侧 `MatchGlob.MatchesThePatternsTheListsUse` 同一组结论。
        assert!(match_glob(
            "*Local Storage*",
            "Default/Local Storage/leveldb/0001.log"
        ));
        assert!(match_glob("*Local Storage*", "Local Storage"));
        assert!(match_glob("History", "History"));
        assert!(!match_glob("History", "Default/History"));
        assert!(match_glob("file?.dat", "file1.dat"));
        assert!(!match_glob("file?.dat", "file12.dat"));
        assert!(match_glob("", ""));
        assert!(!match_glob("", "x"));
        assert!(match_glob("*", ""));
        assert!(match_glob("*", "anything"));
    }
}
