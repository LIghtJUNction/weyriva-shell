# Weyriva Shell 中文简介

Weyriva Shell（读作 **way-REE-vuh**）是一个以 Arch 为主要目标、零配置的
Niri 桌面环境。Weyriva 自己拥有 Shell、登录界面和会话内锁屏；目标运行时是
独立的 Quickshell 0.3 / QtQuick 实现，不把桌面运行时委托给 Noctalia。

greetd 只在内部负责 VT、PAM 认证和创建会话。它不是可见产品界面，Weyriva
也不会重写 PAM。

> **迁移状态：** 仓库正在从早期 Noctalia 委托脚手架迁移到上述独立架构。
> 一键安装脚本、Niri 配置、本地控制守护进程、初版原生 Quickshell
> Shell/Greeter 源码和仓库检查已经存在；原生桌面 Surface、一体化 Greeter
> 与锁屏、兼容插件执行、最终打包和 XRY 验收并未因此完成。详见
> [兼容与验收表](NOCTALIA_PARITY.md)。

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

- Apple-inspired：按下即反馈、直接操控、空间连续、动画可中断，并提供降动效
  等价反馈；
- Anthropic-inspired：粗而略不规则的近黑手绘线、象牙色非规则承载形和一个
  覆盖全画布的柔和强调色。

它们是设计语言参考，不表示复制、隶属或背书。Weyriva 与 Apple、Anthropic、
Noctalia 均无关联。

## 零配置安装

目标安装方式只有一个：

```bash
./install.sh
```

不提供个性化问卷。Arch 及其衍生发行版是主要目标；Fedora、
Debian/Ubuntu 和 openSUSE 尽量支持。需要其他策略的用户应自行 Fork。

脚本目前已经存在，但依赖与会话链仍在原生迁移范围内。在
[测试与验收](TESTING.md)通过之前，它是集成脚手架，不是生产就绪安装器。
安装过程不得在没有明确运维请求时重启正在使用的图形会话。

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

## Shell、IPC 与插件

仓库当前包含版本化本地 JSON 控制守护进程和 legacy 可执行插件通道。它们是
迁移基础设施，不是桌面渲染器。独立 Quickshell 运行时将拥有原生 Surface IPC。

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
```

本地检查不能代替真实登录、锁屏、按钮、日历、插件、打包与 XRY 实机验收。

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
