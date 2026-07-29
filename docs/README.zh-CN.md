# Weyriva Shell 中文简介

Weyriva Shell 是一套以 Arch Linux 为首要目标、围绕 niri 组合的现代 Wayland 桌面环境。它只支持 Linux 上的 Niri/Wayland，不适用于 Windows 或 macOS。Arch 和 CachyOS 是完整支持路径；Fedora、Debian/Ubuntu 与 openSUSE 通过各自的原生包管理器尽力支持。若仓库缺少必需桌面软件包，安装会在复制任何 Weyriva 配置前停止。

从项目检出目录执行唯一的安装命令：

```bash
./install.sh
```

脚本会安装 Niri、Waybar、fuzzel、mako、swaybg、swaylock、swayidle、Foot、Noto Sans 与 pavucontrol；Arch 系还会安装 gsimplecal，随后自动为目标文件创建带时间戳的备份并替换为 Weyriva 配置。安装过程没有选项或个性化提示，也不会启用或重启 greetd 或图形会话。Weyriva 只提供一套默认方案；需要个性化请 Fork 后自行维护。

`scripts/update.sh` 与 `scripts/uninstall.sh` 仅供从 Git 检出维护项目时使用，仍遵循预览与保留优先的行为。

常用控制命令：

```bash
weyriva status
weyriva diagnose
weyriva diagnose --json
sudo weyriva startup ensure
weyriva ipc call weyriva.info
weyriva ipc call weyriva.notifications.dnd
weyriva ipc call weyriva.panel.toggle
weyriva plugin list
weyriva plugin validate examples/plugins/hello.json
weyriva plugin reload
weyriva ipc call weyriva.niri.outputs
weyriva session lock
weyriva wallpaper set ~/Pictures/wallpaper.png
weyriva wallpaper status
```

`weyriva plugin validate` 在安装前校验插件清单并列出缺失的可执行文件;
`weyriva plugin reload` 让运行中的守护进程重新扫描插件清单,无需重启。
`weyriva.niri.outputs` 与 `weyriva.niri.windows` 通过统一 socket 返回 niri
的显示器与窗口 JSON 状态,便于面板和脚本使用。

`weyriva.notifications.dnd` 切换 mako 勿扰模式(默认绑定 Mod+N),也可用
`--params '{"enabled": true}'` 显式开关;`weyriva.panel.toggle` 隐藏或显示
Waybar(默认绑定 Mod+B),`weyriva.panel.reload` 重载其配置。
Waybar 的时钟、网络、音频和电池均可点击，分别打开日历、NetworkManager、音频控制与电源详情；缺少图形工具时会在 Foot 中显示安全的只读回退信息。
`weyriva wallpaper set` 在用户 XDG 配置下记录自定义壁纸并在用户服务可用时
自动重启壁纸服务,`reset` 恢复自带壁纸。

按 `Mod+Shift+X` 可立即锁屏。固定的空闲服务会在五分钟无操作、睡眠前与会话锁定事件时调用同一个 Weyriva 锁屏命令。

`weyriva diagnose` 只检查 Niri 桌面链路：Niri 与运行时依赖、配置语法、Wayland
会话入口、greetd 登录配置、用户服务和当前 Niri 会话。发现登录链路缺失时返回非零
退出码，适合在 TTY 或脚本中直接使用。

`sudo weyriva startup ensure` 用于确保整条启动链完整：校验 Niri 配置、备份并安装
greetd 配置、备份已识别的旧 Weyriva 用户单元、保留用户自定义覆盖、刷新用户服务
管理器并启用 greetd。该命令不会重启 greetd，也不会中断当前图形会话。

协议、插件安全模型和项目边界请阅读 [IPC](IPC.md)、[插件](PLUGINS.md)、[架构](ARCHITECTURE.md) 与 [路线图](ROADMAP.md)。项目中的珊瑚色、奶油色与墨色 SVG 为原创视觉资产；项目与 Anthropic 不存在隶属、背书或官方设计关系。
