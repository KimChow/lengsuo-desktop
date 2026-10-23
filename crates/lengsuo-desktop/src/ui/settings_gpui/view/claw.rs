//! 「Claw 设置」面板的渲染描述（**描述层**：无 GPUI 依赖 ⇒ 无 `ui` feature 也能测）。
//!
//! 文案**逐字**取自文案全表的 `settings.claw` 命名空间，只收只读展示用的
//! 标题/说明/状态；交互类文案（注册/解绑/扫码/报错提示）不进本层。
//!
//! 硬约定：**只读**、**fail-closed**（未下发就显示「未下发」，⛔ 不退化成空列表）、
//! ⛔ 不许自己编中文、⛔ 不伪造服务端数据、⛔ 不做授权推理、⛔ 不持有凭据。
/// 面板标题（与 `PANELS` 清单中的 title 一致，接线层不另造标题）。
pub const TITLE: &str = "Claw 设置";
/// 服务端尚未下发内容时的提示（fail-closed：不伪造数据、不退化成空列表）。
pub const NOT_DELIVERED: &str = "服务端尚未下发内容。";
/// 「远程通道」区块标题（settings.claw.wecomConnection）。
pub const SECTION_TITLE: &str = "远程通道";
/// 「远程通道」区块说明（settings.claw.wecomConnectionDesc）。
pub const SECTION_DESC: &str = "连接第三方消息通道，即可远程指挥电脑端 WorkBuddy 执行任务。";
/// 已知通道（穷举：服务端下发的是 id，中文文案由本层映射，⛔ 不收自由文本）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelId {
    WeCom,
    WecomAibot,
    WecomBot,
    WeixinBot,
    WechatKf,
    WechatMp,
    Dingtalk,
    Feishu,
    Slack,
    Discord,
    Telegram,
    Qq,
    Yuanbao,
}
/// 通道连接状态（穷举，文案映射见 [`ChannelState::as_copy`]）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelState {
    Connected,
    Registered,
    Failed,
    ViewOnly,
}
impl ChannelState {
    /// 状态文案（settings.claw.connected / registered / failed / viewOnly）。
    pub fn as_copy(self) -> &'static str {
        match self {
            Self::Connected => "已连接",
            Self::Registered => "已注册",
            Self::Failed => "失败",
            Self::ViewOnly => "仅查看",
        }
    }
}
/// 只读状态行。
pub struct Row {
    /// 通道集成标题（settings.claw.*Integration）。
    pub label: &'static str,
    /// 集成说明（settings.claw.*IntegrationDesc）。
    pub desc: &'static str,
    /// 连接状态（settings.claw.connected 等）。
    pub value: &'static str,
}
/// 面板主体：未下发 ⇒ 「未下发」提示；有内容 ⇒ 只读行清单。
pub enum Body {
    /// fail-closed：服务端未下发（含下发为空）⇒ 显示 [`NOT_DELIVERED`]。
    NotDelivered,
    /// 已下发 ⇒ 行清单。
    Rows(Vec<Row>),
}
/// 由服务端下发的通道状态得到面板主体；`channels` 为空 ⇒ 未下发（fail-closed）。
pub fn body(channels: &[(ChannelId, ChannelState)]) -> Body {
    if channels.is_empty() {
        return Body::NotDelivered;
    }
    Body::Rows(
        channels
            .iter()
            .map(|&(id, state)| {
                let (label, desc) = catalog(id);
                Row {
                    label,
                    desc,
                    value: state.as_copy(),
                }
            })
            .collect(),
    )
}
/// 通道集成的标题 + 说明（settings.claw.*Integration / *IntegrationDesc）。
fn catalog(id: ChannelId) -> (&'static str, &'static str) {
    match id {
        ChannelId::WeCom => (
            "企业微信集成",
            "支持企业微信一键连通，快速接收和处理来自企业微信的消息。",
        ),
        ChannelId::WecomAibot => (
            "企业微信机器人",
            "在企业微信群「@机器人」下发任务，全员共享执行结果，适合团队协作。",
        ),
        ChannelId::WecomBot => ("企微机器人集成", "注册企微机器人以接收和回复消息。"),
        ChannelId::WeixinBot => (
            "微信助理",
            "通过微信直接下发指令，结果实时回传至微信聊天窗口。",
        ),
        ChannelId::WechatKf => (
            "微信客服号",
            "与微信助理功能一致，微信扫码即可绑定，远程遥控 WorkBuddy 干活。",
        ),
        ChannelId::WechatMp => ("微信小程序集成", "接入微信小程序，用户可通过小程序与 AI 对话。"),
        ChannelId::Dingtalk => (
            "钉钉机器人集成",
            "接入钉钉机器人，成员可在群聊中直接与 WorkBuddy 对话，高效处理指令。",
        ),
        ChannelId::Feishu => (
            "飞书集成",
            "接入飞书机器人，成员可在群聊中直接与 WorkBuddy 对话，高效处理指令。",
        ),
        ChannelId::Slack => ("Slack 集成", "注册 Slack 应用，通过 Slack 接收和回复消息。"),
        ChannelId::Discord => ("Discord 集成", "注册 Discord 机器人以接收和回复消息。"),
        ChannelId::Telegram => ("Telegram 集成", "注册 Telegram Bot 以接收和回复消息。"),
        ChannelId::Qq => ("QQ 机器人", "QQ 聊天窗口直接下达指令，随时查看执行进度。"),
        ChannelId::Yuanbao => ("元宝机器人集成", "连接元宝机器人以接收和回复消息。"),
    }
}
