# D2 Morgath Kick

[![GitHub Pages](https://img.shields.io/badge/产品门户-GitHub%20Pages-1769e0?logo=github)](https://migo-ovo.github.io/D2-Morgath-Kick/)
[![Latest release](https://img.shields.io/github/v/release/MIGO-OvO/D2-Morgath-Kick?display_name=tag&sort=semver)](https://github.com/MIGO-OvO/D2-Morgath-Kick/releases/latest)
[![Windows](https://img.shields.io/badge/Windows-10%20%7C%2011-1769e0?logo=windows11&logoColor=white)](https://github.com/MIGO-OvO/D2-Morgath-Kick/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

D2 Morgath Kick 是一款 Windows 动作校准工具。它基于原项目重构，优化了底层架构，安装体积更小。新的 GUI 把分辨率、灵敏度、瞄准偏移和动作等待集中在一个窗口里，可调整的参数也更多。

这不是自动刷取工具。当前版本不识别 Boss 或玩家状态，也不会自动循环、点击地图或收集掉落。程序只按保存的参数执行一次序列。F10 会中止当前步骤并释放已按下的键位。

[打开产品门户](https://migo-ovo.github.io/D2-Morgath-Kick/) · [下载最新 Windows 安装包](https://github.com/MIGO-OvO/D2-Morgath-Kick/releases/latest/download/D2-Morgath-Kick-Windows-x64-setup.exe)

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/app-dark.png">
  <img src="docs/screenshots/app-light.png" alt="D2 Morgath Kick 动作校准台">
</picture>

## 功能

- 自动读取 Destiny 2 客户区尺寸，也可手动指定分辨率。
- 按 15 / 1.00 参考设置换算 ADS 与腰射移动量。
- 调整首次 ADS、虚空箭落点和冲刺方向。
- 单独修改七项动作等待时间。
- F8 全局启动，F10 全局中止。
- 中止、报错或退出时释放已按下的键盘与鼠标输入。
- 独立的置顶 Overlay，只显示运行状态、当前阶段和快捷键。
- 主窗口支持浅色和深色模式，默认跟随 Windows。
- 窄窗口自动切换为单面板校准界面。

## 运行条件

- 64 位 Windows 10 或 Windows 11
- WebView2 Runtime，大多数 Windows 10/11 系统已经安装
- Destiny 2 使用无边框或窗口模式，以便程序读取客户区尺寸
- 游戏与 D2 Morgath Kick 使用相同的 Windows 完整性级别

序列依赖游戏内按键、FOV、灵敏度和配装。首次使用前请逐项核对设置，并在可控场景中校准。不同 FOV 或鼠标加速会改变实际落点。

## 安装

从 [Latest Release](https://github.com/MIGO-OvO/D2-Morgath-Kick/releases/latest) 下载 `D2-Morgath-Kick-Windows-x64-setup.exe`，运行安装程序后从开始菜单启动 D2 Morgath Kick。

Windows 可能会在首次运行未签名安装包时显示 SmartScreen 提示。请核对下载地址是否属于本仓库，再决定是否继续。

## 使用

1. 启动 Destiny 2，并确认窗口模式、FOV、灵敏度和按键设置。
2. 打开 D2 Morgath Kick，检查自动识别到的分辨率。
3. 按实际情况调整瞄准偏移与等待时间。设置会自动保存。
4. 让 Destiny 2 回到前台，按 F8 执行一次动作序列。
5. 需要停止时按 F10。Overlay 会同步显示当前状态。

固定键位包括 W/A/S/D、Shift、Space、E、2、X、C、F、G 和鼠标右键。当前版本不提供键位重映射。

## 校准项目

| 分组 | 可调内容 | 说明 |
| --- | --- | --- |
| 显示与灵敏度 | 分辨率、视角灵敏度、ADS 修正 | 用于换算鼠标移动量 |
| 瞄准偏移 | 首次 ADS、虚空箭、冲刺方向 | 显示基准值、微调值和最终应用值 |
| 动作时序 | 七项等待参数 | 每项限制在程序允许的范围内 |

## 开发

需要 Node.js、npm、Rust 和 Windows C++ 构建工具。

```powershell
git clone https://github.com/MIGO-OvO/D2-Morgath-Kick.git
cd D2-Morgath-Kick
npm install
npm run tauri dev
```

只构建前端：

```powershell
npm run build
```

运行 Rust 测试：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
```

构建 Windows 安装包：

```powershell
npm run tauri build
```

NSIS 安装包会出现在 `src-tauri/target/release/bundle/nsis/`。

## 项目结构

```text
D2-Morgath-Kick/
├── src/                    # React 主窗口、Overlay 与前端 API
├── src-tauri/              # Rust 动作引擎、配置、输入与 Tauri 设置
├── portal/                 # GitHub Pages 门户页
├── docs/screenshots/       # README 使用的明暗主题截图
├── .github/workflows/      # GitHub Pages 部署工作流
├── PRODUCT.md              # 产品事实与范围
├── DESIGN.md               # 设计变量、组件和主题规则
├── LICENSE                 # MIT License
└── README.md
```

## 报告问题

请在 [GitHub Issues](https://github.com/MIGO-OvO/D2-Morgath-Kick/issues) 提交问题，并附上：

- Windows 版本和系统缩放比例
- Destiny 2 窗口模式与分辨率
- 复现步骤和实际结果
- 错误提示或截图，注意先遮住个人信息

不要在 Issue 中粘贴账号凭据、访问令牌或其他私密数据。

## 贡献

欢迎提交修复和文档改进。请从新分支开始，保持改动范围清楚，并在 Pull Request 中写明验证命令。涉及动作时序的改动需要同时补充或更新 Rust 测试。

## 许可与声明

项目使用 [MIT License](LICENSE)。

D2 Morgath Kick 是非官方开源项目，与 Bungie 无关联。Destiny 2 及相关名称归其权利人所有。使用本工具前，请自行确认适用的游戏条款并承担使用风险。

## 维护者

GitHub: [@MIGO-OvO](https://github.com/MIGO-OvO)
