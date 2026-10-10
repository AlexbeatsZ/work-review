# 验证记录

重构基线为 `88c1332`，本轮把桌面应用替换为后台采集端与集中查看服务。

验证日期：2026-10-03。临时数据和服务均位于各设备系统临时目录的 `.agents/` 下，使用测试密钥；原有配置与数据库未改动，登录启动脚本未注册。

| 检查 | 结果 |
| --- | --- |
| Windows x64 Rust 1.94.1：工作区测试 | 45 项通过 |
| macOS 26.6.2 / Apple Silicon Rust 1.98.1：工作区测试 | 51 项通过，含 Mac 浏览器 AppleScript 编译 |
| Windows / macOS Release 编译 | 采集端与中心服务均通过 |
| `cargo fmt --all --check` | 通过 |
| Node 日期边界 / URL 校验 | 2 项通过 |
| Edge 浏览器端到端测试 | 2 项通过，覆盖设备筛选、时间合并、搜索、截图、备注、导出、无数据、无效密钥及 390px 手机布局 |
| Windows / Mac 临时采集 | 分别生成 2 / 1 条记录；暂停期间不生成记录 |
| 两台实际设备同步至同一临时中心 | 共 2 台设备、3 条记录；两端队列均归零。通过 SSH 临时转发访问，无新增防火墙或常驻网络配置 |
| 原 OMEN 数据库只读导入 | 53,837 条；重复导入新增 0 条。来源路径按旧配置读取 |
| Windows 登录脚本 | PowerShell 语法检查通过；未执行注册 |
| macOS 登录脚本 | `bash -n` 通过；实际 `plutil` 生成并校验含空格和 `&` 路径的 plist；未执行注册 |
| macOS 原生 Vision OCR | 实际识别合成界面图 588 个字符，含引号路径正确传参 |
| Windows 系统原生 OCR | 实际识别同一界面图 401 个字符，含引号路径正确转义 |
| npm 依赖审计 | 0 个已知漏洞 |
| GitHub CI 四个平台 | Windows、Apple Silicon、Intel Mac、Linux 全部通过；Windows / Linux 浏览器测试通过 |

存储与 HTTP 测试覆盖离线队列恢复、重复上传、备注不被重试覆盖、未同步数据不清理、跨日期/小时裁剪、重叠时段合并、分页稳定性、搜索范围、采集/查看密钥分离及无权限截图请求。生命周期测试覆盖全新目录首次启动、暂停、重复进程拒绝和停止状态。

Mac 首次临时运行发现 SQLite 并发首次初始化会报锁定，已调整为先创建 WAL/表，再启动采集与同步循环；回归测试与实际运行均通过。Mac 检查结果为辅助功能已授权、屏幕录制未授权。默认截图关闭，窗口活动采集已实测；若以后开启截图，仍需本人在系统设置中给稳定路径的程序授予屏幕录制权限。

本地构建产物保存在 `artifacts/windows/` 与 `artifacts/macos-arm64/`，另有对应 ZIP / tar.gz 程序包；界面验收图为 `artifacts/dashboard-desktop.png` 和 `artifacts/dashboard-mobile.png`。这些产物不进入 Git。现阶段没有永久部署中心、替换旧应用或注册开机启动。

核心重构提交 `d06a7d7` 的四平台 [GitHub CI](https://github.com/AlexbeatsZ/work-review/actions/runs/37105005866) 全部通过。随后只更新本文与项目/任务日志的验收结果。

Mac 临时验证目录已删除。Windows 临时目录 `C:\Users\Meta\AppData\Local\Temp\.agents\work-review-verify-20261003` 的目录清理被执行策略拒绝（`blocked by policy`），保留了测试数据库和源码归档；测试密钥文件以及临时中心/已配对采集端配置已逐项删除。所有临时采集端、中心及 SSH 转发已结束。

macOS 原有 Objective-C 依赖 `block 0.1.6` 有 Rust 未来兼容警告，当前构建通过。屏幕截图实际采集不属于默认关闭配置的验收范围；图像归档、尺寸/多屏坐标和受保护的网页回看已有测试。

精简量以 `88c1332` 的运行时源码为基线，统计 `src/`、Rust `src/` 中的 `.rs/.js/.svelte/.css`，排除 JS 测试文件、依赖、生成文件与构建产物：69 个文件 / 42,192 行 → 24 个文件 / 9,639 行，减少约 77.2%。后续小幅维护可能使计数变化。

## 实际部署验收：2026-10-04

用户随后授权 OMEN 和 Mac 常驻采集、ROG 集中处理。以下结果为实际部署状态，取代上文源代码验收阶段的「未注册服务」描述。详细路径与回退方法见 [deployment](deployment.md)。

| 检查 | 结果 |
| --- | --- |
| ROG 中心 | `WorkReviewHub` 为 SYSTEM 开机任务，实际服务进程位于 session 0；只监听 `100.106.169.46:47831`，健康检查通过 |
| 访问范围 | 现有 Tailscale 网络；防火墙仅开放该地址 TCP 47831，来源 `100.64.0.0/10` |
| OMEN 采集 | `WorkReviewAgent-Meta` 为登录用户的 Limited 任务；交互 session 1 内生成新记录并上传 |
| Mac 采集 | 用户 GUI LaunchAgent 运行已授权项目 `bin/work-review-agent`；已生成 15 条新记录，ROG API 确认新记录已入库。后续记录继续增加 |
| Mac 权限与升级 | 重用现有本地代码签名证书；用户重新添加辅助功能授权。证书与固定 identifier 验证通过，签名不匹配的更新在停止原采集进程前被拒绝；匹配签名的更新沿用已授权程序路径 |
| 启动与重启 | 两个 Windows 任务、Mac LaunchAgent 重新安装/重启后运行成功，设备 UUID 与数据目录保持不变；没有强制重启电脑或注销 |
| 旧记录 | OMEN 当前旧数据库只读导入并上传 54,006 条；原程序、配置、数据库保留，旧桌面自启动入口已保存并退役 |
| 默认配置 | 10 秒采样、5 分钟闲置、截图/OCR 关闭；既有隐私规则保持 |
| 网络回归 | 继承不可达 HTTP/HTTPS/ALL_PROXY 的 CLI 子进程仍能直接连中心；Windows 与 Mac 测试通过，没有改全局代理 |
| Windows 工作区 | 45 项测试通过；Release 编译、格式检查与安装脚本语法检查通过。移除了仅用于旧 Mac 脚本坐标解析的测试 |
| macOS arm64 工作区 | 52 项测试通过；Release 编译和安装脚本 `bash -n` 通过 |
| 实际 Edge 前端 | 登录成功，显示 OMEN/Mac 两台设备；两端过去一天的已上传记录查询成功，无页面 JS 错误 |

ROG 的数据目录限制 Meta/SYSTEM/Administrators 访问，查看密钥单独保存在 OMEN 私有 `data/rog-view-key.txt` 中；密钥不进入 Git、命令参数或验收输出。截图保持关闭，没有申请屏幕录制权限。Mac 的 System Events 窗口读取在 GUI 后台进程中反复出现超时，已替换为 NSWorkspace/AXUIElement 原生读取；Core Foundation 字符串类型与 Unicode 边界测试通过。新版本重用原签名证书和已授权路径，更新后权限保持有效。

部署提交 `61eee42` 的 [四平台 CI](https://github.com/AlexbeatsZ/work-review/actions/runs/37157374619) 全部通过；后续原生 Mac 采集修正已通过本地 Windows/Mac 工作区检查。

原生采集部署后连续观察 12 次、约 2 分钟：每次均为 `recording`，期间新增 11 条记录，未出现窗口读取错误。ROG 再次确认两端最新记录已入库。签名升级过程未重新申请权限。

原生采集提交 `051d920` 的 CI 中，Rust/编译检查通过，但 Linux 浏览器测试暴露搜索 debounce 期间旧设备查询仍能更新列表的时序问题。加入 150ms 延迟复现后，本地得到同一失败；搜索输入变化时立即使旧查询失效，退出连接时取消 debounce，重新构建中心后 2 项浏览器测试通过。

最终代码提交 `261af74` 的 [四平台 CI](https://github.com/AlexbeatsZ/work-review/actions/runs/37158285137) 全部通过，包含 Windows/Linux 浏览器回归。修正后的 Windows Release 中心已重新部署至 ROG；实际页面登录、两设备状态与最新记录查询再次通过。macOS arm64 中心构建产物也已刷新，正式 Mac 仍只运行采集端。

Mac 部署临时目录及临时权限/签名辅助任务已清理。本机与 ROG 临时采集/查看密钥副本已逐项删除。Windows 本机部署临时目录的文件清理被自动审批拒绝，返回 `blocked by policy`；剩余无密钥脚本和源码归档保留，正式项目数据与后台进程不受影响。

## OMEN 无窗口自启动修复：2026-10-09

登录任务 `WorkReviewAgent-Meta` 直接启动 Scoop PowerShell 7.6.6，带有 `-WindowStyle Hidden`，但同一启动时刻出现 Windows Terminal 1.24.12741.0，窗口标题就是该 `pwsh.exe` 路径。采集端实际处于 `recording`。把同一任务换成 Windows PowerShell 5.1 后仍出现同类窗口，确认仅换 shell 不能解决控制台委派。

任务改为 GUI 子系统的 `run-agent.exe`，以 `UseShellExecute=false` / `CreateNoWindow=true` 启动原采集端。实际运行安装脚本并重启任务后，启动器和采集端都没有主窗口，Windows Terminal 进程消失；任务维持用户交互式 Limited 权限、电池策略、单实例、无限运行时间和一分钟失败重试。原设备 UUID、数据目录和采集配置沿用。

`pwsh -NoProfile -File scripts/test-agent-launcher.ps1` 通过：真实控制台测试子进程的 `GetConsoleWindow()` 为零，带空格和中文的数据目录参数准确传入，stdout/stderr 追加保留，子进程退出码 23 准确返回，缺失子程序返回 1 并记录启动错误。测试临时目录已清理；安装器脚本语法与 `git diff --check` 通过。没有改 Rust 采集/存储逻辑，未重复运行跨平台工作区测试，也没有强制注销或重启。

本次检查中，采集仍正常新增本地记录，但配置的 ROG 中心连接报错，未验证当前远端上传成功；待同步记录保留在本机。此项与终端窗口启动问题分开记录。

## ROG 中心同步恢复：2026-10-10

ROG SYSTEM 任务处于 Ready / result 1，开机启动日志报 `10049`（绑定地址不可用），Tailscale 在本次检查时已在线。手动启动又报 `10048`（地址已占用）。原 47831 及若干相邻端口的实际绑定探针都失败；netstat、Bound 枚举和 active/persistent 排除端口表未找到匹配占用者或配置项，具体来源尚未证实。40001 在 ROG 的 Tailscale 地址上实际绑定成功。

中心迁移到 `http://100.106.169.46:40001/`，同步修改 OMEN/Mac 的 `server_url` 和现有 `WorkReviewHub-Tailscale` 规则的端口过滤器。规则仍只允许 `100.64.0.0/10` 访问该 Tailscale 地址；原设备 UUID、密钥、数据库、采集配置、Mac 稳定路径及签名保持。旧中心程序保存在 ROG 的 `data/hub/bin/work-review-server.previous.exe`，新 Release 程序已由同一 SYSTEM 任务运行。

服务启动逻辑只对 `AddrNotAvailable` 每五秒重试，覆盖开机时网络适配器尚未就绪的情况；端口冲突和权限错误仍正常退出。`cargo test -p work-review-server --locked` 的 2 项绑定策略测试及 2 项 HTTP 测试通过，Release 编译、Rust 格式和 diff 检查通过。真实新程序以测试数据目录绑定非本地 `192.0.2.123:0` 时持续运行超过六秒，记录了两次等待，验证实际入口没有提前退出；测试目录已删除。

实际恢复验证：中心健康接口和网页均为 HTTP 200，认证设备接口确认两个原 UUID 和新鲜心跳。OMEN 的约 4600 条积压持续下降并实测归零，之后中心查询确认收到 `2026-10-10 22:00:13 +08:00` 的新记录（查询时距当前 39 秒）。Mac 同步错误为空、pending 0，处于锁屏状态；本次验证了同步与心跳，没有重新测试解锁后的前台窗口采集。OMEN 的无窗口启动器仍在运行，没有 Windows Terminal 窗口。没有强制重启或注销，冷启动网络时序由实际程序的非本地地址测试覆盖。
