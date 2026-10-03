# Work Review

个人多设备工作记录：**Windows / macOS 后台采集，一处集中回顾。**

采集端是独立 Rust 程序，没有桌面窗口、托盘或 WebView。它先把活动写入本机 SQLite，再同步到自己的中心服务；中心离线期间继续记录，恢复连接后补传。中心服务内嵌中文网页，可从任意设备的浏览器访问。

```text
Windows 采集端 ─ 本地队列 ─┐
                          ├─ 中心服务 / SQLite / 截图 ─ 浏览器
macOS 采集端 ── 本地队列 ─┘
```

网页提供设备状态、按设备/日期筛选的时间线、窗口和内容搜索、应用时长、截图/OCR 回看、备注编辑与 Markdown 统计导出。主活跃时间合并多个设备的重叠时段，另列设备累计时长。

## 已固定的默认配置

以原应用在 OMEN 上的实际设置为基准：10 秒采样、5 分钟空闲阈值、全天记录、截图/OCR 关闭。保留原隐私规则。截图开启时使用全屏幕、自适应宽度、JPEG 85；本机已同步截图保留 3 天、元数据 30 天，截图空间上限 2048 MB。

未同步数据不参与自动清理。中心记录长期保存，不受采集端的本地清理影响。截图/OCR 都是可选项；仅使用系统原生 OCR，不下载 Python 或模型。

## 构建

需要现有的 Node.js 20.19+、Rust stable 和各平台原生编译工具链。

```bash
npm ci
npm run build
cargo build --release --workspace
```

构建产物在 `target/release/`：`work-review-agent` 与 `work-review-server`，Windows 对应 `.exe`。发行包将程序放在解压目录的根目录，下文 `target/release/` 路径按实际程序位置替换即可。中心二进制已经包含网页，不需要额外安装 Node 或部署 `dist`。单独构建采集端不需要前端：`cargo build --release -p work-review-agent`。

中心可运行在 Windows、macOS 或 Linux；采集端支持 Windows 和 macOS。CI 检查 Windows、Apple Silicon、Intel Mac 和 Linux 中心。

## 启动中心

```bash
work-review-server --data-dir /absolute/path/hub init --bind 0.0.0.0:47831
work-review-server --data-dir /absolute/path/hub credentials
work-review-server --data-dir /absolute/path/hub run
```

Windows 使用实际路径，例如 `--data-dir C:\Users\Meta\Project\Workspaces\work-review\data\hub`。省略 `--bind` 时只监听 `127.0.0.1:47831`；多设备接入需要监听可达地址。修改中心配置文件的 `bind` 后重启即可调整地址。

`credentials` 显示两个不同密钥：`agent_token` 交给采集端，`view_token` 输入网页。记录、统计和截图接口均需要密钥。打开 `http://中心地址:47831` 查看。服务自身不提供 TLS；跨公网部署时通过 HTTPS 反向代理访问，也可在自己的 Tailscale 网络中使用。中心密钥可以由 `WORK_REVIEW_AGENT_TOKEN` / `WORK_REVIEW_VIEW_TOKEN` 环境变量覆盖。

## Windows 采集端

先复制中心的采集密钥，再从 PowerShell 初始化。默认数据目录为 `%LOCALAPPDATA%\work-review-agent`。

```powershell
Get-Clipboard | .\target\release\work-review-agent.exe init --name OMEN --server http://中心地址:47831 --token-stdin
.\target\release\work-review-agent.exe doctor
.\target\release\work-review-agent.exe run
```

如需沿用原应用保存的采集/隐私配置，在初始化命令添加 `--from "$env:APPDATA\work-review\config.json"`。初始化不复制旧模型密钥，也不修改旧配置。

登录后无窗口运行：

```powershell
.\scripts\install-agent.ps1 -BinaryPath .\target\release\work-review-agent.exe
```

脚本复制程序到数据目录的 `bin`，建立当前用户的交互式登录任务，并立即启动。任务支持电池运行、异常重启；日志保存在数据目录。它不使用系统服务的 session 0，也不需要管理员权限。卸载登录任务使用 `install-agent.ps1 -Uninstall`，保留配置与记录。

## macOS 采集端

复制中心采集密钥后初始化。默认数据目录为 `~/Library/Application Support/work-review-agent`。

```bash
pbpaste | ./target/release/work-review-agent init --name MacBook --server http://中心地址:47831 --token-stdin
./target/release/work-review-agent doctor
bash scripts/install-agent.sh "$PWD/target/release/work-review-agent"
```

安装脚本复制程序到稳定路径并注册用户 LaunchAgent，在登录的桌面会话中运行。辅助功能权限用于读取窗口标题和浏览器 URL；只有开启截图才需要屏幕录制权限。先将数据目录 `bin/work-review-agent` 的程序加入系统设置相应权限，再通过 `doctor` 检查。可以用 `doctor --request-permissions` 主动触发系统权限引导，后台不会反复弹窗。

macOS 安装脚本需要已经登录的图形会话。SSH 中仅能完成编译和诊断，系统隐私权限仍由本人在 Mac 本地确认。卸载：`bash scripts/install-agent.sh --uninstall`，配置与历史保留。

## 命令调整

所有采集命令支持 `--data-dir 路径`。配置改动会自动在运行中的采集端重载，设备 UUID 保持不变。

```bash
work-review-agent config
work-review-agent config --set enabled=false
work-review-agent config --set enabled=true
work-review-agent config --set screenshot_interval=10 --set idle_threshold_minutes=5
work-review-agent config --set storage.screenshots_enabled=true --set ocr_enabled=true
work-review-agent config --set device.name=MacBook
work-review-agent status
work-review-agent sync
```

`config` 默认隐藏密钥；替换密钥可以通过 `--token-stdin` 从标准输入读取。`sync` 适合采集进程未运行时手动补传；运行中的采集进程已经拥有同步任务，不允许重复启动同步进程。`run --duration 30` 可用于临时运行验证。

隐私支持应用「正常 / 脱敏 / 忽略」、标题关键词与域名黑名单。脱敏会清除标题、URL、截图和 OCR；忽略不会留下活动记录。配置数组可通过 `config --set 'privacy.excluded_domains=["example.com"]'` 或编辑本机 JSON 设置。

## 导入原有记录

原应用数据目录和数据库保持原样。只有执行导入命令才会把旧记录复制到新采集端队列，随后同步至中心。

```powershell
.\target\release\work-review-agent.exe import-legacy `
  --database 'C:\Users\Meta\Documents\Obsidian\Manage\06记录\workreview\OMEN\workreview.db' `
  --screenshots-root "$env:APPDATA\work-review"
.\target\release\work-review-agent.exe sync
```

导入只读打开原数据库，保留窗口、URL、OCR 和已有目的/备注；截图存在时复制到新目录。旧记录采用结束时间，导入会转换为起始时间。稳定 ID 保证重复导入/上传不重复计数。原配置中的手动待跟进清单仍留在原文件中，集中网页使用每条记录的备注。

## 开发与验证

```bash
npm run dev                 # API 代理至本地 47831 中心
npm test
cargo fmt --all --check
cargo test --workspace
cargo build --workspace      # 浏览器测试使用 debug 中心
npm run test:e2e
```

Windows 浏览器测试使用已有 Edge；其他平台需要 Playwright Chromium。测试数据库与临时服务位于系统临时目录的 `.agents/`，使用测试密钥与合成记录。具体结果见 [验证记录](docs/validation.md)。

代码结构与采集/同步约束见 [设计文档](docs/design/background-hub.md)。原桌面版可从 `lite-phase-2` 分支及 Git 历史恢复。
