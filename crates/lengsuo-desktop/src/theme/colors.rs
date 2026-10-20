//! 颜色语义令牌：亮/暗两套调色板，**键一致、只有值不同**。
//!
//! 本文件只回答「什么颜色代表什么」，不回答「怎么画」；渲染后端
//! （GPUI 或其他）从这里取语义名对应的 [`Rgba`] 值即可。
//!
//! # 状态色的中文约定（涨红跌绿）
//!
//! 数值**上升用红色**（[`Palette::rise`]），**下降用绿色**（[`Palette::fall`]）。
//! 这是中国市场的阅读习惯，与欧美相反；资源用量、配额消耗等展示一律沿用，
//! 不得照搬欧美配色。该约定由测试 `rise_is_red_fall_is_green_in_both_themes`
//! 强制，改反了测试会红。
//!
//! # 键一致性
//!
//! 亮暗两套共用同一个 `Palette` 结构体：漏掉任何一个语义键都是**编译错误**，
//! 不存在「暗色下某处变透明」的运行期漏洞。运行期再由 [`KEYS`] 与
//! [`Palette::entries()`] 互相印证（见模块内测试）。

/// 一组颜色，同名字段承载同一语义。
///
/// 两套主题的键在此结构体里**物理上是同一批字段**——这是键一致性的第一道
/// 保证（编译期）；第二道保证见 `tests::keys_are_identical_between_themes`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// 页面最底层背景。
    pub background: Rgba,
    /// 卡片 / 面板等第一层容器背景。
    pub surface: Rgba,
    /// 容器悬停态背景。
    pub surface_hover: Rgba,
    /// 常规描边。
    pub border: Rgba,
    /// 聚焦描边（输入框、可交互控件获得焦点时）。
    pub border_focus: Rgba,
    /// 主文字。
    pub text_primary: Rgba,
    /// 次要文字（说明、元信息）。
    pub text_secondary: Rgba,
    /// 禁用文字（仅示意不可用，不承载必读信息）。
    pub text_disabled: Rgba,
    /// 叠在 [`Palette::accent`] 色块上的文字。
    pub text_on_accent: Rgba,
    /// 主色（选中态、主按钮）。
    pub accent: Rgba,
    /// 主色悬停态。
    pub accent_hover: Rgba,
    /// **涨红**：数值上升（涨红跌绿约定，见模块文档）。
    pub rise: Rgba,
    /// **跌绿**：数值下降（涨红跌绿约定，见模块文档）。
    pub fall: Rgba,
    /// 成功（操作完成、校验通过）。
    pub success: Rgba,
    /// 警告（接近配额、需要留意，但尚可继续）。
    pub warning: Rgba,
    /// 危险（破坏性操作、失败、错误）。
    pub danger: Rgba,
}

/// 全部语义键的权威清单（与 `Palette` 字段一一对应、顺序一致）。
pub const KEYS: &[&str] = &[
    "background",
    "surface",
    "surface_hover",
    "border",
    "border_focus",
    "text_primary",
    "text_secondary",
    "text_disabled",
    "text_on_accent",
    "accent",
    "accent_hover",
    "rise",
    "fall",
    "success",
    "warning",
    "danger",
];

impl Palette {
    /// 亮色主题。
    pub const LIGHT: Palette = Palette {
        background: Rgba::rgb(0xF5, 0xF6, 0xF8),
        surface: Rgba::rgb(0xFF, 0xFF, 0xFF),
        surface_hover: Rgba::rgb(0xEE, 0xF0, 0xF4),
        border: Rgba::rgb(0xD4, 0xD9, 0xE0),
        border_focus: Rgba::rgb(0x24, 0x56, 0xD6),
        text_primary: Rgba::rgb(0x1B, 0x1F, 0x27),
        text_secondary: Rgba::rgb(0x5A, 0x62, 0x70),
        text_disabled: Rgba::rgb(0xA6, 0xAD, 0xB8),
        text_on_accent: Rgba::rgb(0xFF, 0xFF, 0xFF),
        accent: Rgba::rgb(0x24, 0x56, 0xD6),
        accent_hover: Rgba::rgb(0x1C, 0x46, 0xB4),
        rise: Rgba::rgb(0xD9, 0x3A, 0x3A),
        fall: Rgba::rgb(0x17, 0x8A, 0x50),
        success: Rgba::rgb(0x22, 0x88, 0x3F),
        warning: Rgba::rgb(0xB7, 0x79, 0x1F),
        danger: Rgba::rgb(0xC0, 0x39, 0x2B),
    };

    /// 暗色主题。
    pub const DARK: Palette = Palette {
        background: Rgba::rgb(0x15, 0x18, 0x1E),
        surface: Rgba::rgb(0x1E, 0x22, 0x2A),
        surface_hover: Rgba::rgb(0x26, 0x2B, 0x35),
        border: Rgba::rgb(0x3A, 0x41, 0x50),
        border_focus: Rgba::rgb(0x6C, 0x9B, 0xFF),
        text_primary: Rgba::rgb(0xE8, 0xEB, 0xF0),
        text_secondary: Rgba::rgb(0xA3, 0xAC, 0xBB),
        text_disabled: Rgba::rgb(0x5C, 0x65, 0x72),
        text_on_accent: Rgba::rgb(0x10, 0x14, 0x1B),
        accent: Rgba::rgb(0x6C, 0x9B, 0xFF),
        accent_hover: Rgba::rgb(0x87, 0xAE, 0xFF),
        rise: Rgba::rgb(0xF2, 0x5A, 0x5A),
        fall: Rgba::rgb(0x46, 0xB8, 0x7A),
        success: Rgba::rgb(0x5B, 0xC4, 0x89),
        warning: Rgba::rgb(0xE0, 0xA8, 0x3C),
        danger: Rgba::rgb(0xE8, 0x61, 0x4F),
    };

    /// 展开为 `(键, 值)` 序列，顺序与 [`KEYS`] 一致。
    ///
    /// 渲染层若需要按键名取色（如持久化的用户偏好），走这里而不是
    /// 反射字段；新增字段时必须同步维护本方法与 [`KEYS`]，否则测试失败。
    #[must_use]
    pub fn entries(&self) -> Vec<(&'static str, Rgba)> {
        vec![
            ("background", self.background),
            ("surface", self.surface),
            ("surface_hover", self.surface_hover),
            ("border", self.border),
            ("border_focus", self.border_focus),
            ("text_primary", self.text_primary),
            ("text_secondary", self.text_secondary),
            ("text_disabled", self.text_disabled),
            ("text_on_accent", self.text_on_accent),
            ("accent", self.accent),
            ("accent_hover", self.accent_hover),
            ("rise", self.rise),
            ("fall", self.fall),
            ("success", self.success),
            ("warning", self.warning),
            ("danger", self.danger),
        ]
    }
}

/// 8bit RGBA 颜色值（纯数据，不绑定任何渲染后端）。
///
/// 语义层所有令牌一律不透明（alpha=255）；半透明叠加属于「怎么画」，
/// 由渲染层决定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    /// 不透明颜色。
    #[must_use]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// WCAG 相对亮度（sRGB 线性化），取值 0.0..=1.0。
    #[must_use]
    pub fn relative_luminance(self) -> f32 {
        fn channel(c: u8) -> f32 {
            let c = f32::from(c) / 255.0;
            if c <= 0.040_45 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        0.212_6 * channel(self.r) + 0.715_2 * channel(self.g) + 0.072_2 * channel(self.b)
    }

    /// WCAG 对比度（1.0..=21.0），用于文字可读性的测试断言。
    #[must_use]
    pub fn contrast(self, other: Rgba) -> f32 {
        let (hi, lo) = (self.relative_luminance(), other.relative_luminance());
        let (hi, lo) = (hi.max(lo), hi.min(lo));
        (hi + 0.05) / (lo + 0.05)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(p: &Palette) -> Vec<(&'static str, Rgba)> {
        p.entries()
    }

    #[test]
    fn keys_are_identical_between_themes() {
        let light = pairs(&Palette::LIGHT);
        let dark = pairs(&Palette::DARK);
        assert_eq!(light.len(), dark.len(), "两套主题键数量必须一致");
        for (i, ((lk, _), (dk, _))) in light.iter().zip(dark.iter()).enumerate() {
            assert_eq!(lk, dk, "第 {i} 个键亮暗不一致：{lk} vs {dk}");
        }
    }

    #[test]
    fn every_palette_field_is_exported_by_entries() {
        // 不带 `..` 的解构：`Palette` 新增字段而忘了在这里登记时，本测试**编译失败**。
        // 再用登记结果与 `entries()` 逐项比对，防止字段加了、导出的键没跟上。
        let Palette {
            background,
            surface,
            surface_hover,
            border,
            border_focus,
            text_primary,
            text_secondary,
            text_disabled,
            text_on_accent,
            accent,
            accent_hover,
            rise,
            fall,
            success,
            warning,
            danger,
        } = Palette::LIGHT;
        let destructured = [
            ("background", background),
            ("surface", surface),
            ("surface_hover", surface_hover),
            ("border", border),
            ("border_focus", border_focus),
            ("text_primary", text_primary),
            ("text_secondary", text_secondary),
            ("text_disabled", text_disabled),
            ("text_on_accent", text_on_accent),
            ("accent", accent),
            ("accent_hover", accent_hover),
            ("rise", rise),
            ("fall", fall),
            ("success", success),
            ("warning", warning),
            ("danger", danger),
        ];
        assert_eq!(
            destructured.as_slice(),
            Palette::LIGHT.entries().as_slice(),
            "entries() 必须导出结构体的每个字段，键名与值都要对上"
        );
    }

    #[test]
    fn keys_match_the_authoritative_key_list() {
        let light = pairs(&Palette::LIGHT);
        assert_eq!(light.len(), KEYS.len(), "entries() 与 KEYS 数量不一致");
        for ((k, _), expected) in light.iter().zip(KEYS.iter()) {
            assert_eq!(k, expected, "entries() 的键顺序必须与 KEYS 一致");
        }
        let dark = pairs(&Palette::DARK);
        assert_eq!(dark.len(), KEYS.len(), "暗色键数量与 KEYS 不一致");
    }

    #[test]
    fn keys_have_no_duplicates() {
        let mut keys: Vec<&str> = pairs(&Palette::LIGHT).into_iter().map(|(k, _)| k).collect();
        keys.sort_unstable();
        let before = keys.len();
        keys.dedup();
        assert_eq!(before, keys.len(), "语义键不得重复");
    }

    #[test]
    fn rise_is_red_fall_is_green_in_both_themes() {
        for (name, p) in [("LIGHT", Palette::LIGHT), ("DARK", Palette::DARK)] {
            assert!(
                p.rise.r > p.rise.g + 30 && p.rise.r > p.rise.b + 30,
                "{name} 的 rise 必须是明确的红色（涨红跌绿约定）"
            );
            assert!(
                p.fall.g > p.fall.r + 30,
                "{name} 的 fall 必须是明确的绿色（涨红跌绿约定）"
            );
        }
    }

    #[test]
    fn semantic_colors_keep_their_hue_family() {
        for (name, p) in [("LIGHT", Palette::LIGHT), ("DARK", Palette::DARK)] {
            assert!(
                p.danger.r > p.danger.g && p.danger.r > p.danger.b,
                "{name} danger 偏红"
            );
            assert!(
                p.success.g > p.success.r && p.success.g > p.success.b,
                "{name} success 偏绿"
            );
            assert!(
                p.warning.r > p.warning.g && p.warning.g > p.warning.b,
                "{name} warning 应为琥珀色（r>g>b）"
            );
            assert!(
                p.accent.b > p.accent.r,
                "{name} accent 应偏蓝，与涨/跌色区分开"
            );
        }
    }

    #[test]
    fn all_tokens_are_opaque() {
        for (name, p) in [("LIGHT", Palette::LIGHT), ("DARK", Palette::DARK)] {
            for (k, c) in p.entries() {
                assert_eq!(c.a, 255, "{name} 的 {k} 必须不透明；半透明是渲染层的事");
            }
        }
    }

    #[test]
    fn body_text_meets_readability_contrast_on_background() {
        for (name, p) in [("LIGHT", Palette::LIGHT), ("DARK", Palette::DARK)] {
            let primary = p.background.contrast(p.text_primary);
            let secondary = p.background.contrast(p.text_secondary);
            assert!(
                primary >= 4.5,
                "{name} text_primary/background 对比度 {primary:.2} < 4.5"
            );
            assert!(
                secondary >= 4.5,
                "{name} text_secondary/background 对比度 {secondary:.2} < 4.5"
            );
            let disabled = p.background.contrast(p.text_disabled);
            assert!(
                disabled < secondary,
                "{name} text_disabled 对比度({disabled:.2})不得高于次要文字({secondary:.2})，\
                 否则文字层级失效"
            );
        }
    }

    #[test]
    fn luminance_math_matches_reference_values() {
        let black = Rgba::rgb(0, 0, 0);
        let white = Rgba::rgb(255, 255, 255);
        assert!(
            (white.contrast(black) - 21.0).abs() < 1e-3,
            "黑白对比度是 WCAG 的标定值 21"
        );
        assert!(black.relative_luminance().abs() < f32::EPSILON);
    }
}
