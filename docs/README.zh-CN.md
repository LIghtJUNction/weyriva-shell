# Weyriva Shell 中文简介

Weyriva Shell（读作 **way-REE-vuh**）是一个以 Arch 为主要目标、零配置的
Niri 桌面环境。Weyriva 自己拥有 Shell、登录界面和会话内锁屏。生产架构
由 Rust 实现 `weyriva` CLI、常驻守护进程、启动/会话控制、诊断和插件控制
面，并由 Rust 实现隔离的 `weyriva-luau-host`；Quickshell 0.3 / QtQuick
只负责 UI 呈现，不把桌面运行时委托给 Noctalia。

greetd 只在内部负责 VT、PAM 认证和创建会话。它不是可见产品界面，Weyriva
也不会重写 PAM。

> **当前状态：** 生产控制面、启动/会话命令、诊断、插件生命周期与 Luau
> Host 已由 Rust workspace 实现。一键安装器和本地 AUR 配方都会构建并
> 打包 `weyriva` 与 `weyriva-luau-host`；这只是仓库实现，不代表 AUR
> 已发布或 XRY 已部署这次切换。精确的 `noctalia-v5-luau/1` API 3 单
> Launcher Provider 切片已通过本地测试，包含固定版本官方 Kaomoji 证据，
> Provider 分类也已传到 QML。XRY 有已批准的 UI iteration 3
> Shell/Greeter 预览，并保留此前部署的控制面里程碑，但尚未部署本次全
> Rust 切换。其他五种 Entry、API 4–19、v4 QML Host、完整 Surface、
> 干净打包证据和完整 XRY 验收仍未完成。详见
> [兼容与验收表](NOCTALIA_PARITY.md)。

插件产品名称是 **Weyriva Plugins**。`v5` 只属于上游兼容 profile 标识，
不是 Weyriva 的产品版本。Python 可以作为仓库测试工具，但不是生产运行时
依赖，也不是插件语言。

## 产品范围

Weyriva 的确定目标包括：

- 登录、桌面、认证锁屏、休眠、注销和故障恢复；
- Bar、托盘、Launcher、日历、控制中心、通知、剪贴板、壁纸、OSD、设置、
  截图和桌面组件；
- Weyriva 原生控制协议与版本化插件兼容层；
- 无安装问卷、无个性化选择的固定默认配置。

文档统一使用四种状态：

| 状态 | 含义 |
|---|---|
| **已实现** | 仓库中存在实现，并有对应本地检查 |
| **迁移中** | 架构已经确定，但实现或集成尚未完成 |
| **计划** | 已确认范围，但暂无足够实现证据 |
| **已验证** | 已在声明的真实环境中执行，并留下证据 |

“已实现”不等于“已验证”。本地测试不能证明 PAM、真实 Wayland 输入、安全
锁屏、插件 UI 或 XRY 行为。

## 视觉与交互

Weyriva 保留两个项目自有的设计参考：

- Apple-inspired 功能层：按下即反馈、严格的来源屏幕归属、按触发点区分的
  可中断动效，以及降动效等价反馈；
- Anthropic-inspired 只用于环境背景、品牌时刻、Greeter/Lock 构图和真正
  的空状态。

日常界面使用聚焦的命令面板、紧凑且对齐触发点的工具弹层、语义化明暗主题和
较大的结构化工作区；不使用包住所有面板的统一承载形、默认卡片网格或无效控件。

它们是设计语言参考，不表示复制、隶属或背书。Weyriva 与 Apple、Anthropic、
Noctalia 均无关联。

## 零配置安装

安装入口只有一个：

```bash
./install.sh
```

不提供个性化问卷。Arch 及其衍生发行版是主要目标；Fedora、
Debian/Ubuntu 和 openSUSE 尽量支持。需要其他策略的用户应自行 Fork。

脚本会先解析依赖，再构建两个 locked Rust release 二进制并验证命令面，
随后安装 Shell、Greeter、用户服务和固定默认配置。Arch 包管理是主要路径；
`dnf`、`apt` 和 `zypper` 为尽力支持。干净机器、AUR 发布和非 Arch
发行版的实测仍是生产就绪前置条件。安装器不会重启 greetd 或原本未运行的
用户服务。

## 目标快捷键

```text
Mod+Space       Launcher
Mod+Return      终端
Mod+V           剪贴板历史
Mod+C           控制中心
Mod+N           勿扰模式
Mod+Shift+T     明暗主题
Mod+W           壁纸
Mod+Shift+E     会话与恢复
Mod+Shift+X     锁屏
Print           区域截图
Mod+H/J/K/L     焦点移动
Mod+1/2/3       工作区
```

这些是产品交互合同，不是“按钮已可用”的声明。每个 Surface 都必须通过
鼠标、键盘、焦点和可见状态验收。

## Weyriva Plugins

已通过本地测试的 Rust `weyriva` 插件核心是有序固定源、安全不可变安装与
状态、生命周期、Host 会话、受限动作和 Unix IPC 的唯一目标所有者。每个已
支持 Entry 运行在独立且有边界的 Rust Luau Host 进程中；QML 不加载插件
代码，Launcher 只渲染验证后的结果。本地安装器与打包元数据已经使用这套
Rust 控制面，但已安装机器和 XRY 行为仍需独立证据。

当前兼容声明严格限定为：API 3、单一 Launcher Provider Entry，并已用自有
Fixture 和固定版本的官方 Kaomoji 插件在本地验证，Provider 分类也已传到
QML。它不代表其他五种 Noctalia Entry、API 4–19 或 v4 QML profile 已兼容。

Noctalia 只作为固定提交的公开行为与插件 ABI 参考。Weyriva 必须自行实现兼容
行为；能列出 catalog 或解析 manifest 不等于插件兼容。

- [插件总览](PLUGINS.md)
- [插件兼容合同](plugins/compatibility-contract.md)
- [Noctalia v5 Luau 兼容 profile](plugins/noctalia-v5-luau.md)
- [Noctalia v4 QML 兼容 profile](plugins/noctalia-v4-qml.md)

## 开发与验收

```bash
make test
make check
./scripts/check.sh
```

Make 目标执行 locked Rust 检查；`scripts/check.sh` 另外覆盖仓库策略、
安装器、Shell、配置、QML 与可用的系统工具。Python 只用于测试工具。本地
检查不能代替真实登录、锁屏、按钮、日历、打包与 XRY 实机验收。

主要文档：

- [架构](ARCHITECTURE.md)
- [开发](DEVELOPMENT.md)
- [会话生命周期](SESSION_LIFECYCLE.md)
- [设计系统](DESIGN_SYSTEM.md)
- [主题](THEMING.md)
- [动效](MOTION.md)
- [无障碍](ACCESSIBILITY.md)
- [开发者体验](DEVELOPER_EXPERIENCE.md)
- [IPC](IPC.md)
- [插件](PLUGINS.md)
- [测试](TESTING.md)
- [路线图](ROADMAP.md)
- [兼容与验收表](NOCTALIA_PARITY.md)
