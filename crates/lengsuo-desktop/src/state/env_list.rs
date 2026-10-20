//! 环境列表视图状态（对标端语义：环境卡片 + 分组）。
//!
//! 纯逻辑：不发起请求，只维护界面需要的状态与渲染分支判定。
//! 数据由打包时引入的 agent 客户端注入（`apply_entries`）。
//!
//! 判定顺序严格 fail-closed：拿不准时往更不健康的方向靠，不假装一切正常。

use serde_json::Value;

/// 单个环境条目：保留原始 JSON 供渲染取字段，同时抽出界面必需的 `id`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvEntry {
    /// 环境标识（来自 `id` / `env_id` / `uuid`）。
    pub id: String,
    /// 展示名；缺失时由界面决定占位文案。
    pub name: Option<String>,
    /// 原始条目，渲染层按需取字段。
    pub raw: Value,
}

/// 环境视图健康态。判定次序（先匹配先生效）：
/// `Loading` → `Failed` → `Degraded` → `Ready`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvStatus {
    /// 正在拉取（或尚未拉取过）。
    Loading,
    /// 就绪：最近一次加载无错误且无坏条目。
    Ready,
    /// 降级：列表可用，但有 N 条数据解析不出来。
    Degraded,
    /// 失败：最近一次拉取或整体解析失败。
    Failed,
}

/// 环境列表视图状态。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnvListViewState {
    entries: Vec<EnvEntry>,
    invalid: usize,
    selected: Option<String>,
    loading: bool,
    last_error: Option<String>,
}

impl EnvListViewState {
    /// 空列表：尚未拉取，或权威结果就是零条。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 当前条目，按注入顺序。
    #[must_use]
    pub fn entries(&self) -> &[EnvEntry] {
        &self.entries
    }

    /// 解析失败的条目数（fail-closed 的可见性）。
    #[must_use]
    pub fn invalid_count(&self) -> usize {
        self.invalid
    }

    /// 当前选中项。
    #[must_use]
    pub fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    /// 最近一次错误。
    #[must_use]
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// 标记开始加载：进入 `Loading`，但**不清空已有条目**（避免界面闪空）。
    pub fn begin_load(&mut self) {
        self.loading = true;
    }

    /// 注入一批条目。`id` 缺失的条目计入 `invalid`，不静默丢弃。
    pub fn apply_entries(&mut self, raw_entries: Vec<Value>) {
        self.loading = false;
        self.last_error = None;
        self.entries.clear();
        self.invalid = 0;

        for raw in raw_entries {
            match extract_id(&raw) {
                Some(id) => {
                    let name = raw
                        .get("name")
                        .or_else(|| raw.get("title"))
                        .and_then(Value::as_str)
                        .map(str::to_owned);
                    self.entries.push(EnvEntry { id, name, raw });
                }
                None => self.invalid += 1,
            }
        }

        // 选中项若已不在列表中则清空，避免界面引用悬空 id。
        if let Some(sel) = &self.selected {
            if !self.entries.iter().any(|e| &e.id == sel) {
                self.selected = None;
            }
        }
    }

    /// 记录一次加载失败。
    pub fn fail(&mut self, message: impl Into<String>) {
        self.loading = false;
        self.last_error = Some(message.into());
        self.entries.clear();
        self.invalid = 0;
        self.selected = None;
    }

    /// 选中某条目；不存在则不改变选中态（不凭空选中）。
    pub fn select(&mut self, id: &str) {
        if self.entries.iter().any(|e| e.id == id) {
            self.selected = Some(id.to_owned());
        }
    }

    /// 当前健康态。
    #[must_use]
    pub fn status(&self) -> EnvStatus {
        if self.loading {
            return EnvStatus::Loading;
        }
        if self.last_error.is_some() {
            return EnvStatus::Failed;
        }
        if self.invalid > 0 {
            return EnvStatus::Degraded;
        }
        EnvStatus::Ready
    }
}

/// 从原始条目抽取 id，按 `id` → `env_id` → `uuid` 顺序取第一个非空字符串。
fn extract_id(raw: &Value) -> Option<String> {
    ["id", "env_id", "uuid"]
        .iter()
        .filter_map(|k| raw.get(*k))
        .find_map(|v| match v {
            Value::String(s) if !s.is_empty() => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn new_state_is_ready_with_no_entries() {
        let s = EnvListViewState::new();
        assert_eq!(s.status(), EnvStatus::Ready);
        assert!(s.entries().is_empty());
        assert_eq!(s.invalid_count(), 0);
    }

    #[test]
    fn begin_load_switches_to_loading_without_clearing_entries() {
        let mut s = EnvListViewState::new();
        s.apply_entries(vec![json!({"id": "e1"})]);
        s.begin_load();
        assert_eq!(s.status(), EnvStatus::Loading);
        assert_eq!(s.entries().len(), 1);
    }

    #[test]
    fn id_extraction_prefers_id_then_env_id_then_uuid() {
        assert_eq!(extract_id(&json!({"id": "a", "env_id": "b"})).unwrap(), "a");
        assert_eq!(
            extract_id(&json!({"env_id": "b", "uuid": "c"})).unwrap(),
            "b"
        );
        assert_eq!(extract_id(&json!({"uuid": "c"})).unwrap(), "c");
        assert_eq!(extract_id(&json!({"id": 7})).unwrap(), "7");
        assert!(extract_id(&json!({"name": "x"})).is_none());
        assert!(extract_id(&json!({"id": ""})).is_none());
    }

    #[test]
    fn entries_without_id_are_counted_not_silently_dropped() {
        let mut s = EnvListViewState::new();
        s.apply_entries(vec![json!({"id": "e1"}), json!({"name": "no-id"})]);
        assert_eq!(s.entries().len(), 1);
        assert_eq!(s.invalid_count(), 1);
        assert_eq!(s.status(), EnvStatus::Degraded);
    }

    #[test]
    fn fail_clears_everything_and_reports_failed() {
        let mut s = EnvListViewState::new();
        s.apply_entries(vec![json!({"id": "e1"})]);
        s.fail("gateway unreachable");
        assert_eq!(s.status(), EnvStatus::Failed);
        assert!(s.entries().is_empty());
        assert_eq!(s.last_error(), Some("gateway unreachable"));
    }

    #[test]
    fn selection_is_dropped_when_entry_disappears() {
        let mut s = EnvListViewState::new();
        s.apply_entries(vec![json!({"id": "e1"})]);
        s.select("e1");
        assert_eq!(s.selected(), Some("e1"));
        s.apply_entries(vec![json!({"id": "e2"})]);
        assert_eq!(s.selected(), None);
    }

    #[test]
    fn selecting_unknown_id_does_not_change_state() {
        let mut s = EnvListViewState::new();
        s.apply_entries(vec![json!({"id": "e1"})]);
        s.select("nope");
        assert_eq!(s.selected(), None);
    }
}
