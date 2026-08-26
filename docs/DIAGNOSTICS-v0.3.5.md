# v0.3.5 诊断版本 — 构建、功能与 Windows 手动验收说明

分支：`fix/diagnostics-v0.3.5` · 版本 `0.3.5`

## 功能总览（对应需求）

1. **原生后端自检与禁止正式 mock**
   - 新增 `native_ping` 命令：返回版本、commit SHA、构建配置、平台、PID 与时间戳。
   - 前端启动时握手；正式构建（`import.meta.env.PROD`）中非 Tauri 环境**不再静默进入前端 mock**，所有动作 API 直接抛错（`PROD_MOCK_DISABLED_MESSAGE`），启动/停止按钮被禁用。开发模式（`npm run dev` 浏览器预览）保留模拟数据仅供界面调试。

2. **F8/F10 全局热键诊断**
   - `diagnostics.rs::HotkeyTracker` 按角色（启动/停止）分别记录 `parse` / `parse.failed` / `register` / `register.failed` / `unregister` / `rollback` / `restore` / `is_registered` 全轨迹，含**原始错误**。
   - 前端诊断面板“热键”组逐项显示配置、解析、注册、is_registered 状态与完整原始错误（等宽字体、不截断）。

3. **SendInput 主动输入自检**
   - `input::run_probes()` 依次运行三种探针：扫描码 W（`KEYEVENTF_SCANCODE`+0x11）、虚拟键 W（`wVk=0x57`）、相对鼠标移动（+1,0 与 -1,0，净位移为零）。
   - 每项记录：SendInput 请求数、调用次数、返回数、LastError（系统消息+错误码）、前台进程名、前台进程完整性级别（TokenIntegrityLevel）、W 类探针的 GetAsyncKeyState 观察值、耗时。
   - 探针仅由用户在诊断面板中显式触发（“重新检测”/“导出诊断包”），不引入驱动输入、低级绕过或反作弊规避。

4. **结构化日志与环形缓冲**
   - `diagnostics.rs::Hub`：JSONL 事件（`ts/level/category/event/message/error/details`）追加到日志目录 `diagnostics.jsonl`（超过 4 MB 滚动为 `.old.jsonl`），同时保留 1024 条内存环形缓冲。
   - 事件类别：`backend` / `config` / `hotkey` / `input` / `sequence` / `export`。

5. **诊断面板**
   - 底部“诊断”按钮打开，含后端、热键、输入、环境四组；支持重新检测、复制全部原始错误、导出诊断包。

6. **导出 ZIP 诊断包（用户下载目录）**
   - `export_diagnostics_package` 生成 `d2-morgeth-kick-diagnostics-v0.3.5-<时间戳>.zip`，内含：
     `summary.txt`、`system.json`、`build.json`、`hotkeys.json`、`input-probes.json`、`runtime-events.jsonl`、`config-sanitized.json`。
   - 脱敏：日志与所有路径中的用户主目录前缀替换为 `<USER>`；配置本身只含校准参数与按键绑定（无个人信息），脱敏说明写入 `config-sanitized.json`。

7. **runtime-state 错误完整保留**
   - 收到 `status: "error"` 的 `runtime-state` 事件时，前端自动把完整 `message` 保存到错误横幅与诊断面板“最近运行时错误”，不再只在顶部状态栏截断显示；`runAction` 失败时直接展示后端原始错误，不再替换为通用提示。

8. **兼容性**：动作序列与默认参数（时序、瞄点、键位）完全未改；输入方式仍为 SendInput/相对移动，无新增绕过手段。

9. **测试与构建检查**
   - Rust 单元测试：`cargo test --manifest-path src-tauri/Cargo.toml`（37 项：环形缓冲、JSONL、日期算法、热键轨迹、路径脱敏、ZIP 导出内容、SendOutcome、完整性标签等）。
   - 前端：`npm run build`（`tsc && vite build`）。
   - 格式与静态检查：`cargo fmt --check`、`cargo clippy --all-targets`。

10. **构建信息与签名安装包**
    - `build.rs` 嵌入 `GIT_COMMIT`（`git rev-parse HEAD`，CI 签出后自动携带）、`BUILD_PROFILE`、`TARGET`、`BUILD_TIMESTAMP`，经 `build_info()` 进入 `build.json` / `system.json` / `summary.txt` 与前端显示。
    - 签名 NSIS 安装包复用现有 `.github/workflows/release.yml`（tauri-action + `TAURI_SIGNING_PRIVATE_KEY` secret）：推送 `v0.3.5` 标签即生成签名安装包、`latest.json` 与 updater 签名（draft release）。

## Windows 手动验收清单（建议顺序）

前置：已安装 v0.3.5（`src-tauri/target/release/bundle/nsis/` 下的安装包），Destiny 2 以窗口/无边框运行。

| # | 步骤 | 预期结果 |
|---|------|----------|
| 1 | 启动程序，观察底部状态栏 | 显示“参数已就绪，按 F8 启动”，启动/停止按钮可用 |
| 2 | 打开“诊断”→ 后端组 | 状态“可用”，显示 v0.3.5、完整 commit SHA、构建配置、平台与握手延迟 |
| 3 | 热键组 | F8/F10 各显示：解析=成功、注册=成功、is_registered=true，无错误块 |
| 4 | 将 Destiny 2 置于前台，点击“重新检测” | 输入组出现 3 项探针结果：请求数 2 / 返回数 2、LastError=无、前台进程=destiny2.exe、完整性级别（如 Medium/High） |
| 5 | 切回桌面（explorer 前台）再“重新检测” | 前台进程变为 explorer.exe，其余字段正常 |
| 6 | 点击“复制错误” | 剪贴板得到“未发现错误记录。”或全部原始错误 |
| 7 | 点击“导出诊断包” | 下载目录出现 ZIP；解压后核对 7 个文件齐全，summary.txt 含版本/commit，路径均为 `<USER>` 开头 |
| 8 | 检查日志文件 | `%APPDATA%` 应用配置目录下日志目录（诊断面板“环境”组给出路径）存在 `diagnostics.jsonl`，每行一个 JSON 事件 |
| 9 | 制造热键冲突：把停止热键改为与启动相同后保存 | 保存报错；诊断面板热键组显示完整原始错误（“启动热键和停止热键不能相同”）；改回 F10 后状态恢复 |
| 10 | 运行时错误保留 | 触发一次动作错误（如配置校验失败），确认错误横幅显示完整原始错误，且诊断面板“最近运行时错误”保留同一文本 |
| 11 | 正式构建 mock 禁令 | 直接从 `dist` 以浏览器打开 index.html（无 Tauri）：启动/停止按钮禁用，显示“原生后端不可用…禁止静默进入前端模拟模式” |
| 12 | 回滚验证 | 在按键设置中把 F8 改为 F9 保存后改回：热键组事件轨迹含 unregister/register/is_registered 序列 |

## 签名发布流程（GitHub Actions）

1. 合并/推送 `fix/diagnostics-v0.3.5` 到 main；
2. 推送标签 `v0.3.5` → `.github/workflows/release.yml` 自动：
   前端构建检查 → `cargo test` → tauri-action 生成**签名 NSIS 安装包**（`D2-Morgeth-Kick-v0.3.5-Windows-x64-setup.exe`）、`latest.json` 与 updater 签名，产出 draft release；
3. 在 release 页面核对安装包与签名后发布，应用内更新即生效。
