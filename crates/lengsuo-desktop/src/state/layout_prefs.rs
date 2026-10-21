//! 版式偏好（侧边栏宽度 / 右侧面板宽度 / 折叠态）的**纯逻辑**序列化层。
//!
//! ⛔ 本模块不碰文件：层约定是「纯逻辑，无 IO」。
//! 文件读写由 `ui::gpui_app::prefs_file` 负责（那是唯一做 IO 的地方）。
//!
//! 格式：与 `state::prefs` 同构的**极简可读 JSON**，一个对象、三个键：
//! 例：`{"sidebar_width":260.0,"right_width":320.0,"right_collapsed":false}`
//!
//! 宽度单位是**逻辑像素**（与 `ui::layout` 的尺寸单位一致）。
//! ⛔ 夹取区间不在本模块实现：本模块只负责**解析原始值**，
//! 越界值由调用方经 `SidebarLayout::set_width` / `RightPanelLayout::set_width`
//! 落地（描述层独占夹取逻辑，本模块不重复实现）。

/// 偏好 JSON 的键名：边栏宽度。
pub const SIDEBAR_WIDTH_KEY: &str = "sidebar_width";
/// 偏好 JSON 的键名：右侧面板**展开态**宽度。
pub const RIGHT_WIDTH_KEY: &str = "right_width";
/// 偏好 JSON 的键名：右侧面板是否折叠。
pub const RIGHT_COLLAPSED_KEY: &str = "right_collapsed";

/// 边栏默认宽度（`ui::layout::SIDEBAR_DEFAULT_WIDTH` 的**同源常量**）。
///
/// 之所以定义在状态层：偏好回落需要一份默认值，而状态层不许依赖 `ui` 层
/// （会造成循环依赖）⇒ 常量住在低层，描述层反向引用它。
pub const DEFAULT_SIDEBAR_WIDTH: f32 = 260.0;
/// 右侧面板默认宽度（`ui::layout::RIGHT_PANEL_DEFAULT_WIDTH` 的**同源常量**）。
pub const DEFAULT_RIGHT_WIDTH: f32 = 320.0;

/// 版式偏好的值（纯数据；描述层不依赖 `ui`，避免循环依赖）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutPrefs {
    /// 边栏宽度（逻辑像素）。
    pub sidebar_width: f32,
    /// 右侧面板**展开态**宽度（逻辑像素）。
    ///
    /// 折叠态的实际占位宽度是固定的窄条宽度，不随用户设置变化 ⇒
    /// 这里存的是「展开后恢复到多宽」。
    pub right_width: f32,
    /// 右侧面板是否折叠。
    pub right_collapsed: bool,
}

impl Default for LayoutPrefs {
    fn default() -> Self {
        Self {
            sidebar_width: DEFAULT_SIDEBAR_WIDTH,
            right_width: DEFAULT_RIGHT_WIDTH,
            right_collapsed: false,
        }
    }
}

/// 把版式偏好序列化成偏好 JSON 文本。
///
/// **非有限宽度（NaN / ∞）被替换成默认值** ⇒ 产出永远是**合法 JSON**
/// （`format!("{:?}", f32::NAN)` 会写出 `NaN`，那不是合法 JSON）。
#[must_use]
pub fn encode_layout(prefs: LayoutPrefs) -> String {
    format!(
        "{{\"{SIDEBAR_WIDTH_KEY}\":{:?},\"{RIGHT_WIDTH_KEY}\":{:?},\"{RIGHT_COLLAPSED_KEY}\":{}}}",
        finite_or(prefs.sidebar_width, DEFAULT_SIDEBAR_WIDTH),
        finite_or(prefs.right_width, DEFAULT_RIGHT_WIDTH),
        prefs.right_collapsed
    )
}

/// 从偏好 JSON 文本解析版式偏好。
///
/// **字段级非法值一律回落到默认值**（缺键 / `null` / 非数值 / 非有限）：
/// 只有**整个 JSON 坏掉、或者不是一个对象**才 `Err`。
///
/// 不 panic、不静默吞错：返回 `Result`，调用方决定是记日志还是提示。
/// 回落方向是**保守**的：宽度取默认值、折叠态取「展开」——
/// 宁可给用户一个正常宽度的面板，也不给一个宽度为 0 的窄条。
pub fn decode_layout(text: &str) -> Result<LayoutPrefs, String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("版式偏好 JSON 解析失败：{e}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| "版式偏好 JSON 不是对象".to_owned())?;
    Ok(LayoutPrefs {
        sidebar_width: width_or(object.get(SIDEBAR_WIDTH_KEY), DEFAULT_SIDEBAR_WIDTH),
        right_width: width_or(object.get(RIGHT_WIDTH_KEY), DEFAULT_RIGHT_WIDTH),
        right_collapsed: object
            .get(RIGHT_COLLAPSED_KEY)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
    })
}

/// 取一个宽度字段：缺键 / 非数值 / 非有限 ⇒ 回落默认值。
///
/// 非有限的情况真实存在：JSON 里合法的 `1e39` 解析成 `f64` 是有限的，
/// 但转成 `f32` 会溢出成 `∞` ⇒ 这里必须再查一次 `is_finite()`。
fn width_or(value: Option<&serde_json::Value>, fallback: f32) -> f32 {
    value
        .and_then(serde_json::Value::as_f64)
        .map(|width| width as f32)
        .filter(|width| width.is_finite())
        .unwrap_or(fallback)
}

/// 非有限宽度 ⇒ 回落默认值。
fn finite_or(width: f32, fallback: f32) -> f32 {
    if width.is_finite() {
        width
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    // 显式列出符号：不用 `use super::*`（父模块的 `use` 会把测试属性宏带进来遮蔽 `#[test]`）。
    use super::{
        decode_layout, encode_layout, LayoutPrefs, DEFAULT_RIGHT_WIDTH, DEFAULT_SIDEBAR_WIDTH,
        RIGHT_COLLAPSED_KEY, RIGHT_WIDTH_KEY, SIDEBAR_WIDTH_KEY,
    };

    #[test]
    fn encode_produces_readable_json_with_all_three_keys() {
        let text = encode_layout(LayoutPrefs::default());
        assert!(text.contains(SIDEBAR_WIDTH_KEY), "缺边栏宽度键：{text}");
        assert!(text.contains(RIGHT_WIDTH_KEY), "缺右栏宽度键：{text}");
        assert!(text.contains(RIGHT_COLLAPSED_KEY), "缺折叠态键：{text}");
        assert!(text.contains("260.0"), "默认边栏宽度应可见：{text}");
        assert!(text.contains("320.0"), "默认右栏宽度应可见：{text}");
    }

    #[test]
    fn round_trip_keeps_all_three_fields() {
        let prefs = LayoutPrefs {
            sidebar_width: 300.5,
            right_width: 421.25,
            right_collapsed: false,
        };
        let decoded = decode_layout(&encode_layout(prefs)).expect("自己编码的文本应能解回来");
        assert_eq!(decoded, prefs);
    }

    #[test]
    fn round_trip_keeps_the_collapsed_flag() {
        for collapsed in [true, false] {
            let prefs = LayoutPrefs {
                right_collapsed: collapsed,
                ..LayoutPrefs::default()
            };
            let decoded = decode_layout(&encode_layout(prefs)).expect("折叠态应能往返");
            assert_eq!(decoded.right_collapsed, collapsed);
        }
    }

    #[test]
    fn missing_keys_fall_back_to_defaults_without_erroring() {
        let decoded = decode_layout("{}").expect("缺键不算坏文件");
        assert_eq!(decoded, LayoutPrefs::default());
        assert_eq!(decoded.sidebar_width, DEFAULT_SIDEBAR_WIDTH);
        assert_eq!(decoded.right_width, DEFAULT_RIGHT_WIDTH);
        assert!(!decoded.right_collapsed, "折叠态默认展开");
    }

    #[test]
    fn broken_json_is_an_error_not_a_panic() {
        let result = decode_layout("not json at all");
        assert!(result.is_err(), "坏 JSON 应报错而不是 panic");
        let result = decode_layout("[1, 2, 3]");
        assert!(result.is_err(), "不是对象的 JSON 应报错");
    }

    #[test]
    fn null_and_string_widths_fall_back_to_defaults() {
        let decoded = decode_layout(
            "{\"sidebar_width\":null,\"right_width\":\"三百\",\"right_collapsed\":null}",
        )
        .expect("字段非法不算坏文件");
        assert_eq!(
            decoded.sidebar_width, DEFAULT_SIDEBAR_WIDTH,
            "null ⇒ 默认值"
        );
        assert_eq!(decoded.right_width, DEFAULT_RIGHT_WIDTH, "字符串 ⇒ 默认值");
        assert!(!decoded.right_collapsed, "非布尔折叠态 ⇒ 展开");
    }

    #[test]
    fn non_finite_widths_fall_back_to_defaults() {
        // `1e39` 在 JSON 里合法，`f64` 也有限，但转 `f32` 会溢出成 `∞`。
        let decoded = decode_layout("{\"sidebar_width\":1e39,\"right_width\":-1e39}")
            .expect("溢出不算坏文件");
        assert_eq!(
            decoded.sidebar_width, DEFAULT_SIDEBAR_WIDTH,
            "溢出 ⇒ 默认值"
        );
        assert_eq!(decoded.right_width, DEFAULT_RIGHT_WIDTH, "溢出 ⇒ 默认值");

        // NaN 不是合法 JSON：整个文件解析失败 ⇒ 报错（不静默吞错）。
        assert!(decode_layout("{\"sidebar_width\":NaN}").is_err());

        // 编码侧兜底：非有限宽度被替换成默认值 ⇒ 产出仍是合法 JSON。
        let text = encode_layout(LayoutPrefs {
            sidebar_width: f32::NAN,
            right_width: f32::INFINITY,
            right_collapsed: false,
        });
        let decoded = decode_layout(&text).expect("兜底后的文本应能解回来");
        assert_eq!(decoded.sidebar_width, DEFAULT_SIDEBAR_WIDTH);
        assert_eq!(decoded.right_width, DEFAULT_RIGHT_WIDTH);
    }
}
