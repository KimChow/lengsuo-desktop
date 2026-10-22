# lengsuo-desktop

长梭桌面端 **界面仓**（GPUI 渲染层 + 视图逻辑）。

> **文档不入本仓**：`docs/` 全部只保留在本地开发工作区（用于开发进展与验收记录），
> 由独立 git 仓同步到 GitLab 私有仓；**不随本仓库分发，也不保留在本仓 git 历史中**。
> 需要查阅请找开发方索取。

## 本仓的边界

**只做界面。** 不实现协议、不实现运行时、不实现业务语义。

```text
┌─────────────────────────────────────────┐
│ 本仓 · lengsuo-desktop                   │
│   state/    视图状态机（纯逻辑）           │
│   theme/    设计令牌 + 交互语义            │
│   ui/       GPUI 视图                     │
└────────────────┬────────────────────────┘
                 │ 打包时引入（依赖注入）
┌────────────────▼────────────────────────┐
│ 打包产物                                  │
│   lengsuo-agent（运行时 + 网关客户端）      │
│   lengsuo-network（网络数据面 + 浏览器组件） │
└─────────────────────────────────────────┘
```

- **不搬 agent 的代码进本仓**：协议客户端、运行时归 `lengsuo-agent`，
  桌面应用打包时作为依赖/子进程整合进来。
- **界面不判定权限**：本仓只承载服务端下发的可见性标记，不做授权推理。
  权限判定以服务端为权威。界面禁用 ≠ 授权。
- **不持有凭据**：token 等由打包时引入的客户端管理，界面层不读取、不落盘。

## 目录

| 路径 | 内容 |
|---|---|
| `crates/lengsuo-desktop/src/state/` | 视图状态：环境列表 / 组织域 / 会话流 |
| `crates/lengsuo-desktop/src/theme/` | 设计令牌：命令面板 / 工具名人话映射 |
| `crates/lengsuo-desktop/src/ui/` | GPUI 渲染层（阶段二，`feature = "ui"`） |

## 构建与门禁

```bash
python scripts/check_line_limit.py
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
```

单文件 **400 行软目标 / 700 行硬上限**（含空行、注释、测试）。
生产代码禁 `unsafe`。异步读写测试必须有超时。

## 已知前置任务

**Windows 中文输入法（IME）修复**是渲染层可交付的前提条件：

- Zed issue #56327：`WM_KEYDOWN` 路径不检查 IME 组合态 → 汉字重复插入/崩溃
- Zed issue #64850：IME 定位只给一个点 + `CFS_CANDIDATEPOS` 无排除区 → 候选框遮挡输入

正解为 `CFS_EXCLUDE` + `rcArea` 传 caret 矩形。**修复前不得声称中文输入可用。**

## 许可

Apache-2.0 / MIT 依赖合规；禁 GPL/AGPL/SSPL 链接依赖。
本仓自身为 `LicenseRef-Proprietary`（源码可查看 ≠ 开源授权）。
