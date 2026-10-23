//! `InputAreaState` 的 `@` 提及方法：候选集 / 过滤 / 查询解析 / 插入。

use std::collections::HashSet;

use super::InputAreaState;

impl InputAreaState {
    /// 注入候选路径（去重、保持顺序）。
    ///
    /// 候选列表由**调用方**提供（宿主 Glob 结果）；本仓不读盘、不扫描文件系统。
    pub fn set_candidates(&mut self, paths: impl IntoIterator<Item = String>) {
        let mut seen: HashSet<String> = HashSet::new();
        self.candidates = paths
            .into_iter()
            .map(|p| p.trim().to_owned())
            .filter(|p| !p.is_empty() && seen.insert(p.clone()))
            .collect();
    }

    /// 当前候选路径（只读）。
    #[must_use]
    pub fn candidates(&self) -> &[String] {
        &self.candidates
    }

    /// 按查询词过滤候选（大小写不敏感）；空查询返回全部。
    ///
    /// 命中规则：路径**前缀**或路径**子串**包含查询词即命中（`src/a.rs`
    /// 用 `a` 也能命中，方便只想记文件名时快选）。
    #[must_use]
    pub fn filter_candidates(&self, query: &str) -> Vec<String> {
        let q = query.trim().to_lowercase();
        self.candidates
            .iter()
            .filter(|p| q.is_empty() || p.to_lowercase().contains(&q))
            .cloned()
            .collect()
    }

    /// 从草稿里解析 `@` 之后的查询词（到下一个空白为止）；没有 `@` 返回 `None`。
    ///
    /// `@` 必须在词首（草稿开头或前面是空白）才算引用标记，
    /// 否则 `someone@example.com` 这类邮箱不会被误判成文件引用。
    #[must_use]
    pub fn mention_query(&self) -> Option<String> {
        let idx = self.mention_index()?;
        Some(self.draft[idx + 1..].to_owned())
    }

    /// 草稿里 `@` 标记的字节下标（词首的 `@`）。
    ///
    /// 找不到合法的 `@` 标记时返回 `None`。解析与插入共用这一处判定，
    /// 避免「查询解析得到的位置」与「实际替换的位置」不一致。
    #[must_use]
    pub fn mention_index(&self) -> Option<usize> {
        let idx = self.draft.rfind('@')?;
        if idx > 0 {
            let prev = self.draft[..idx].chars().next_back()?;
            if !prev.is_whitespace() {
                return None;
            }
        }
        let tail = &self.draft[idx + 1..];
        if tail.contains(char::is_whitespace) {
            return None;
        }
        Some(idx)
    }

    /// 把候选路径插入草稿：替换掉 `@查询词`，得到 `@路径 `。
    pub fn insert_mention(&mut self, path: &str) {
        let Some(idx) = self.mention_index() else {
            return;
        };
        let mut next = String::with_capacity(self.draft.len() + path.len() + 1);
        next.push_str(&self.draft[..idx]);
        next.push('@');
        next.push_str(path);
        next.push(' ');
        self.draft = next;
    }
}
