//! 附件 chip 行（A12 附件族，13 册 §3.2「粘贴图片显示缩略 chip」）。
//!
//! 图标记 + 文件名 + 移除记号；**空列表不渲染该行**（不留空壳）。
//! 附件路径由调用方注入（[`InputAreaGpui::set_attachments`]），本仓不读盘、
//! 不发上传请求；移除记号只改本地状态，不上报删除意图。
#![cfg(feature = "ui")]

use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::ui::input_area::{chip_label, InputAreaState};

use super::InputAreaGpui;

/// 移除记号的触摸目标边长（无障碍：≥48dp）。
const REMOVE_TARGET_PX: f32 = 48.;

/// 单个附件 chip 的渲染描述（由描述层的附件列表派生，纯函数）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentChip {
    /// 完整路径（移除时回传描述层）。
    pub path: String,
    /// chip 文案（[`chip_label`] 产物）。
    pub label: String,
    /// chip 容器 id。
    pub id: SharedString,
    /// 移除记号 id。
    pub remove_id: SharedString,
}

/// chip 行描述：空列表返回 `None` ⇒ 接线层**不渲染该行**。
#[must_use]
pub fn chip_row(state: &InputAreaState) -> Option<Vec<AttachmentChip>> {
    if state.attachments().is_empty() {
        return None;
    }
    Some(
        state
            .attachments()
            .iter()
            .map(|path| AttachmentChip {
                path: path.clone(),
                label: chip_label(path),
                id: format!("input-attachment-chip-{path}").into(),
                remove_id: format!("input-attachment-remove-{path}").into(),
            })
            .collect(),
    )
}

/// 渲染附件 chip 行（挂在输入框上方，不影响发送/停止与 `@` 快选）。
pub(super) fn render(
    view: &InputAreaGpui,
    theme: &Theme,
    cx: &mut Context<InputAreaGpui>,
) -> Option<AnyElement> {
    let chips = chip_row(&view.state)?;
    Some(
        div()
            .id("input-attachment-row")
            .flex()
            .flex_wrap()
            .gap_1()
            .children(chips.iter().map(|chip| render_chip(chip, theme, cx)))
            .into_any_element(),
    )
}

/// 单个 chip：`theme.muted` 底 + `theme.border` 描边，移除记号 48×48 可点。
fn render_chip(
    chip: &AttachmentChip,
    theme: &Theme,
    cx: &mut Context<InputAreaGpui>,
) -> AnyElement {
    let path = chip.path.clone();
    let remove = cx.listener(
        move |this: &mut InputAreaGpui, _: &ClickEvent, _window: &mut Window, cx| {
            if this.state.remove_attachment(&path) {
                cx.notify();
            }
        },
    );
    div()
        .id(chip.id.clone())
        .flex()
        .flex_row()
        .items_center()
        .min_h(px(REMOVE_TARGET_PX))
        .rounded_sm()
        .border_1()
        .border_color(theme.border)
        .bg(theme.muted)
        // 图标记：附件族统一记号（本卡只有路径，无缩略内容可画）。
        .child(
            div()
                .pl_2()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child("图"),
        )
        .child(
            div()
                .px_1()
                .text_sm()
                .text_color(theme.foreground)
                .child(chip.label.clone()),
        )
        .child(
            div()
                .id(chip.remove_id.clone())
                .flex()
                .w(px(REMOVE_TARGET_PX))
                .h(px(REMOVE_TARGET_PX))
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(theme.muted_foreground)
                .cursor_pointer()
                .on_click(remove)
                .child("×"),
        )
        .into_any_element()
}

impl InputAreaGpui {
    /// 注入附件路径：chip 行随之出现；重复路径按 [`chip_order`] 去重。
    ///
    /// 为什么公开：附件由粘贴等运行时链路上报，无窗口渲染断言也从这里摆状态。
    pub fn set_attachments(
        &mut self,
        paths: impl IntoIterator<Item = String>,
        cx: &mut Context<Self>,
    ) {
        self.state.set_attachments(paths);
        cx.notify();
    }
}
