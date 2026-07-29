# Weyriva Shell 中文简介

Weyriva Shell 是一套以 Arch Linux 为首要目标、围绕 Niri 构建的零配置
Wayland 桌面。它使用上游
[Noctalia v5](https://github.com/noctalia-dev/noctalia) 负责完整桌面 Shell 和
会话内锁屏，并使用 Noctalia Greeter 提供统一视觉的登录界面。

greetd 不会被删除。它隐藏在 Weyriva 登录界面之后，只负责 VT/seat、PAM
认证、用户身份切换和会话创建。Weyriva 不重写 PAM、不启用自动登录，并永久
保留 TTY2 恢复通道。

> **当前状态：** 仓库集成仍在验收中。一键源码安装入口已经存在，但登录链、
> Arch/AUR 打包、官方/社区插件全量矩阵、无障碍、锁屏崩溃恢复，以及 XRY
> 实机视觉和点击证据都必须通过后，才能称为可安装、已部署或完整交付。详见
> [Noctalia 对标验收表](NOCTALIA_PARITY.md)。

## 一体化范围

Noctalia 统一负责：

- 多显示器 bar、组件、托盘、任务栏、媒体、网络、蓝牙、电池和亮度；
- Dock、启动器、控制中心、通知与历史、剪贴板、壁纸、OSD 和截图；
- 设置与热重载、桌面/锁屏组件、空闲策略、锁屏和会话操作；
- `plugin.toml` + 可信 Luau 的原生 v5 插件。

Noctalia Greeter 是可见登录层；greetd 是隐藏的认证/会话 broker；Niri 是
合成器。目标启动链为：

```text
display-manager → greetd → noctalia-greeter-session
→ Weyriva session → Niri/systemd user session
→ weyriva-shell.service + weyriva-ipc.service
```

Waybar、fuzzel、mako、swaybg、swaylock 和 swayidle 不得与 Noctalia 同时
争用同一表面。

## 视觉与交互

- `apple-design` 用于功能层级、系统字体、材料、立即反馈、空间连续性和动效；
- `anthropic-art` 只用于项目自有的壁纸、登录/锁屏插画和空状态；
- 两者都是设计参考，不代表 Apple、Anthropic 或 Noctalia 的隶属或背书；
- 默认使用壁纸动态取色、较忠于源图且降低饱和度的 `soft` 生成器、
  light/dark 和固定时间表的 auto 模式；
- 主题和壁纸采用确定性的 400ms 淡入淡出，不使用随机特效；
- reduced-motion 关闭 Shell 位移动画，并保留短淡入淡出或完全关闭壁纸过渡。

详细规范：

- [设计系统](DESIGN_SYSTEM.md)
- [主题](THEMING.md)
- [动效](MOTION.md)
- [无障碍](ACCESSIBILITY.md)

## 零配置安装

Weyriva 仅支持 Linux/Niri/Wayland。Arch 和 Arch 系是首要路径，
Arch/AUR/systemd 是第一打包与服务目标。Fedora、Debian/Ubuntu 和
openSUSE 只有在原生仓库具备兼容依赖时才尽力支持。

从检出目录执行：

```bash
./install.sh
```

安装器不提问、不提供个性化选项，并在替换受管用户文件前创建时间戳备份。
它不会静默重启当前图形会话。需要不同终端、合成器、认证架构、工作区模型或
整体视觉策略时，请 Fork 并维护自己的发行版。

安装完成不等于实机验收完成。系统登录配置需要权限，必须在应用前审阅；
完整步骤见 [测试与验收](TESTING.md)。

## Vibe-coding 默认操作

```text
Mod+Space       启动器
Mod+Return      Foot 终端
Mod+V           剪贴板历史
Mod+C           控制中心/系统状态
Mod+N           免打扰
Mod+Shift+T     light/dark 手动切换
Mod+W           壁纸
Mod+Shift+E     会话与恢复操作
Mod+Shift+X     锁屏
Print           区域截图
Mod+H/J/K/L     焦点导航
Mod+1/2/3       工作区
```

详见 [开发者体验](DEVELOPER_EXPERIENCE.md)。

## Shell 与插件

`weyriva shell` 始终使用隔离的 Weyriva 配置、状态和数据目录：

```bash
weyriva shell config validate
weyriva shell msg status
weyriva shell msg panel-toggle launcher
weyriva shell msg theme-mode-get
weyriva shell msg color-scheme-get
weyriva shell msg session lock
```

当前 Noctalia v5 插件直接交给已安装的同一引擎：

```bash
weyriva plugin list
weyriva plugin install noctalia/screen_recorder
weyriva plugin disable noctalia/screen_recorder
weyriva plugin enable noctalia/screen_recorder
weyriva plugin update official
```

`plugin install ID` 是 enable/materialize 的零配置别名。Noctalia v5 没有
逐插件 remove；disable 不等于删除。旧 Weyriva JSON 可执行插件只保留在明确
标注的 legacy 通道。Noctalia v4 QML 需要独立 Quickshell companion host，
目前仍未实现、未兼容。详见 [插件](PLUGINS.md)。

Weyriva 自己的本地 JSON IPC 与原生 Noctalia IPC 是两条不同通道：

```bash
weyriva diagnose
weyriva diagnose --json
weyriva ipc call weyriva.info
weyriva ipc call weyriva.niri.outputs
```

详见 [IPC](IPC.md)。

## 开发与验收

```bash
make test
make check
```

本地检查通过不能代替真实登录、PAM、锁屏、按钮、日历、插件和 XRY 实机验收。

开发文档入口：

- [开发指南](DEVELOPMENT.md)
- [架构](ARCHITECTURE.md)
- [会话生命周期](SESSION_LIFECYCLE.md)
- [测试与验收](TESTING.md)
- [Noctalia 对标验收表](NOCTALIA_PARITY.md)
