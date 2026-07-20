# Weyriva Shell 中文简介

Weyriva Shell 是一套以 Arch Linux 为首要目标、围绕 niri 组合的现代 Wayland 桌面环境。当前 0.1.0 是可运行的项目基础：包含 Waybar、fuzzel、mako、greetd/tuigreet 模板、壁纸、systemd 用户服务、IPC 与受信任本地插件机制，但尚不是经过广泛硬件和真实会话验证的完整桌面发行版。

用户安装器默认只预览，不会修改系统：

```bash
./scripts/check.sh
./scripts/install.sh
./scripts/install.sh --apply
```

已有配置默认保留，即使内容完全相同也不会被认领；应用安装后会在 `${XDG_STATE_HOME:-$HOME/.local/state}/weyriva` 记录路径和 SHA-256。更新和卸载只处理仍与记录哈希一致的文件；已修改文件始终保留，过时文件则退出管理。源码用户安装不会复制登录会话条目，可从 TTY 运行 `~/.local/bin/weyriva session start` 测试。greetd 选择需要提供 `/usr/bin/weyriva` 与系统会话条目的系统/AUR 安装；其配置仍须单独审阅并显式应用。

常用控制命令：

```bash
weyriva status
weyriva ipc call weyriva.info
weyriva plugin list
```

协议、插件安全模型和项目边界请阅读 [IPC](IPC.md)、[插件](PLUGINS.md)、[架构](ARCHITECTURE.md) 与 [路线图](ROADMAP.md)。项目中的珊瑚色、奶油色与墨色 SVG 为原创视觉资产；项目与 Anthropic 不存在隶属、背书或官方设计关系。
