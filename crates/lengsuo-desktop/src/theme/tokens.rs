//! 尺寸令牌与主题切换：间距 / 字号 / 圆角，纯数据无渲染。
//!
//! 命名一律用语义阶梯（`SPACE_SNUG`、`FONT_BODY`、`RADIUS_CARD`），
//! 不用 `spacing_7` 这类序号命名——序号会随版式微调失去意义，语义名
//! 在取值变化时仍然正确。
//!
//! 单位约定：长度单位是**逻辑像素（device-independent px）**，缩放交给
//! 渲染后端；本层不感知 DPI、不感知主题之外的任何环境。

use crate::theme::colors::{Palette, Rgba};

/// 间距阶梯令牌（单位：逻辑像素）。
///
/// 语义阶梯按「紧 → 松」排列，视图按语义选档，不自己发明数值。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spacing {
    /// 元素内最小间隙（图标与文字之间）。
    pub hair: u32,
    /// 紧密堆叠（同一组内相邻行）。
    pub tight: u32,
    /// 常规间隙（表单项之间）。
    pub snug: u32,
    /// 容器内边距（卡片内容区）。
    pub pad: u32,
    /// 区块间距（卡片与卡片之间）。
    pub section: u32,
    /// 页面级留白（大分区的分隔）。
    pub page: u32,
}

/// 间距令牌的权威取值（本层只有一套，不随亮暗变化）。
pub const SPACING: Spacing = Spacing {
    hair: 2,
    tight: 4,
    snug: 8,
    pad: 12,
    section: 20,
    page: 32,
};

/// 字号阶梯令牌（单位：逻辑像素的字体高度）。
///
/// 字号**按用途**命名而非按大小：`Body` 是正文，`Label` 是表单标签，
/// `Caption` 是次要说明，`Mono` 是代码/日志等宽文本。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontScale {
    /// 页面主标题。
    pub title: u32,
    /// 分区标题。
    pub heading: u32,
    /// 表单标签、按钮文字。
    pub label: u32,
    /// 正文（默认字号）。
    pub body: u32,
    /// 次要说明、元信息。
    pub caption: u32,
    /// 等宽场景（命令、日志、路径）。
    pub mono: u32,
}

/// 字号令牌的权威取值。
pub const FONTS: FontScale = FontScale {
    title: 20,
    heading: 16,
    label: 14,
    body: 14,
    caption: 12,
    mono: 13,
};

/// 圆角阶梯令牌（单位：逻辑像素）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Radius {
    /// 无圆角（表格单元、分隔条）。
    pub none: u32,
    /// 小圆角（徽标、标签）。
    pub chip: u32,
    /// 控件圆角（按钮、输入框）。
    pub control: u32,
    /// 容器圆角（卡片、面板）。
    pub card: u32,
    /// 全圆（胶囊、脉动指示点）。
    pub pill: u32,
}

/// 圆角令牌的权威取值。
pub const RADIUS: Radius = Radius {
    none: 0,
    chip: 4,
    control: 6,
    card: 10,
    pill: 999,
};

/// 一个主题的全部令牌。
///
/// 亮暗两套只有 [`Palette`] 的值不同，尺寸令牌共用 —— 尺寸不随主题变化，
/// 避免暗色模式下版式跳动。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tokens {
    pub palette: Palette,
    pub spacing: Spacing,
    pub fonts: FontScale,
    pub radius: Radius,
}

/// 主题模式。默认跟随系统；渲染层拿不到系统偏好时必须显式落到 [`Theme::Light`]，
/// **不允许**用「未设置」表示跟随（那样暗色下会一路用亮色令牌，属于静默降级）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    /// 跟随系统（由渲染层把系统偏好解析成 Light/Dark 后再取令牌）。
    #[default]
    System,
    /// 亮色。
    Light,
    /// 暗色。
    Dark,
}

impl Theme {
    /// 解析为具体主题：`System` 在**本层无系统信息**，按约定回退到亮色。
    ///
    /// 渲染层应在调用前用系统偏好把 `System` 换成 `Light`/`Dark`；
    /// 这里的回退是给纯逻辑测试与保守默认用的，不假装知道系统设置。
    #[must_use]
    pub const fn resolved(self) -> Self {
        match self {
            Self::System => Self::Light,
            other => other,
        }
    }

    /// 取该主题的令牌。
    #[must_use]
    pub const fn tokens(self) -> Tokens {
        let palette = match self {
            // `System` 与 `Light` 同一支：`resolved()` 的语义在此处强制。
            Self::System | Self::Light => Palette::LIGHT,
            Self::Dark => Palette::DARK,
        };
        Tokens {
            palette,
            spacing: SPACING,
            fonts: FONTS,
            radius: RADIUS,
        }
    }
}

impl Default for Tokens {
    fn default() -> Self {
        Theme::default().tokens()
    }
}

/// 涨跌方向。UI 用它选色，**不自己拼颜色**。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    /// 数值上升。
    Rise,
    /// 数值下降。
    Fall,
    /// 持平 / 无变化。
    Flat,
}

impl Tokens {
    /// 按涨跌方向取状态色：上升→[`Palette::rise`]（红），下降→[`Palette::fall`]（绿）。
    ///
    /// 中文约定「涨红跌绿」，与欧美相反。渲染层不得自行判断方向配色。
    #[must_use]
    pub const fn trend_color(&self, trend: Trend) -> Rgba {
        match trend {
            Trend::Rise => self.palette.rise,
            Trend::Fall => self.palette.fall,
            Trend::Flat => self.palette.text_secondary,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spacing_ladder_is_strictly_increasing() {
        let s = SPACING;
        let ladder = [s.hair, s.tight, s.snug, s.pad, s.section, s.page];
        assert!(
            ladder.windows(2).all(|w| w[0] < w[1]),
            "间距必须严格递增：{ladder:?}"
        );
    }

    #[test]
    fn font_ladder_orders_title_above_body_above_caption() {
        let f = FONTS;
        assert!(
            f.title > f.heading && f.heading > f.body,
            "标题阶梯必须递减"
        );
        assert!(f.body > f.caption, "正文必须大于次要说明");
        assert_eq!(f.body, f.label, "正文与控件标签同级，避免同一屏字号漂移");
        assert!(
            f.mono >= f.caption && f.mono <= f.heading,
            "等宽字号落在正文附近"
        );
    }

    #[test]
    fn radius_ladder_is_non_decreasing_and_pill_is_capped() {
        let r = RADIUS;
        assert!(r.none < r.chip && r.chip < r.control && r.control < r.card);
        assert!(r.pill > r.card, "胶囊圆角必须大于卡片圆角");
        assert!(
            r.pill >= 180,
            "胶囊圆角取足够大的值，具体数值由渲染层按高度裁剪"
        );
    }

    #[test]
    fn sizes_are_shared_by_both_themes() {
        let light = Theme::Light.tokens();
        let dark = Theme::Dark.tokens();
        assert_eq!(light.spacing, dark.spacing, "尺寸令牌不随主题变化");
        assert_eq!(light.fonts, dark.fonts, "字号令牌不随主题变化");
        assert_eq!(light.radius, dark.radius, "圆角令牌不随主题变化");
        assert_ne!(light.palette, dark.palette, "两套主题的调色板必须不同");
    }

    #[test]
    fn system_theme_resolves_to_light_not_to_blank() {
        assert_eq!(
            Theme::System.resolved(),
            Theme::Light,
            "无系统信息时回退到亮色，而非留空"
        );
        assert_eq!(Theme::System.tokens(), Theme::Light.tokens());
        assert_eq!(Theme::Dark.resolved(), Theme::Dark);
    }

    #[test]
    fn trend_colors_follow_the_rise_red_fall_green_convention() {
        for theme in [Theme::Light, Theme::Dark] {
            let t = theme.tokens();
            let (rise, fall, flat) = (
                t.trend_color(Trend::Rise),
                t.trend_color(Trend::Fall),
                t.trend_color(Trend::Flat),
            );
            assert_eq!(rise, t.palette.rise, "上升必须用 rise 色");
            assert_eq!(fall, t.palette.fall, "下降必须用 fall 色");
            assert!(rise.r > rise.g, "{theme:?} 上升色必须偏红");
            assert!(fall.g > fall.r, "{theme:?} 下降色必须偏绿");
            assert_eq!(
                flat, t.palette.text_secondary,
                "持平用次要文字色，不与涨跌争视觉"
            );
            assert_ne!(rise, fall, "涨跌必须可区分");
        }
    }

    #[test]
    fn default_tokens_are_the_default_theme() {
        assert_eq!(Tokens::default(), Theme::default().tokens());
    }
}
