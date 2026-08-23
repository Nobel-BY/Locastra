# Locastra · 本地智聊

Locastra 是面向 Windows 的本地 AI 工作台。内部工程标识继续使用 `LocalDeploy`，以兼容已有数据目录和升级路径。

一个面向 Windows 10/11 x64 的开源本地大模型桌面应用。它帮助没有命令行经验的用户检测硬件、搜索 GGUF 模型、从中国大陆友好的来源断点下载，并通过 `llama.cpp` 在本机聊天。

## 下载

普通用户请前往 [GitHub Releases](https://github.com/Nobel-BY/Locastra/releases) 下载 `Locastra_0.9.0_x64-setup.exe`。安装包内置 Vulkan/CPU 与 NVIDIA CUDA 运行组件，不需要单独安装 Python、Node.js 或 llama.cpp。

> 当前 0.9.0 公开测试版尚未进行商业代码签名，Windows SmartScreen 可能提示“未知发布者”。请核对 Release 页面提供的 SHA-256 后再运行。

## 已实现

- 简体中文首次引导与自动硬件检测（CPU、内存、显卡、磁盘、指令集）
- ModelScope OpenAPI / Hugging Face 双源搜索适配器
- 模型中心支持适合本机、15B 以下和热门模型筛选
- 仅展示公开 GGUF 文本模型，排除 mmproj 等不兼容文件
- 获取完整文件大小；将多分片 GGUF 合并为一个版本，按总大小评估并自动整组下载
- 根据可用内存评估“流畅运行 / 可以运行 / 不建议”，默认推荐 Q4_K_M
- ModelScope 国内源；Hugging Face 下载默认依次尝试自定义 HTTPS 镜像、hf-mirror.com 和官方源
- 下载可选择忽略 Windows HTTP/SOCKS 系统代理，便于在开启代理型 VPN 时保持模型直连
- `.part` 断点续传、HTTP Range、暂停/恢复/取消、进度事件、磁盘预检；大文件无总时限并支持同源自动退避重试、连接保活与缓冲写盘
- 文件大小、可选 SHA-256 和 GGUF 魔数校验
- 本地 GGUF 原地导入、模型库、有效性检查和模型加载
- SQLite 保存设置、下载任务、模型信息、对话和消息
- `llama-server` 随机本机端口、自动 GPU 卸载、健康检查与流式聊天
- Markdown 风格代码块、思考内容折叠、停止生成、会话历史
- 对话全文搜索、文件夹、标签、置顶、归档、分支、编辑重试以及 JSON 导入/导出
- 提示词与参数预设；可保存系统提示词、温度、上下文和高级采样参数
- 自定义助手；为不同任务保存独立角色、首选模型和生成参数
- 本地资料库；导入 TXT、Markdown、CSV、JSON 和常见源码，并在对话中按需检索引用
- 文本文件与图片附件；视觉模型可为每个模型配置配套 `mmproj` GGUF
- 多模型横向对比；同一问题依次交给 2–3 个模型，比较回答质量和生成速度
- 每模型运行设置、收藏/备注/排序、目录扫描、重复项检查、模型移动和速度基准测试
- 推测解码；可为主模型配置同词表的小型草稿模型
- Windows 本地语音输入与回答朗读（依赖系统语音包，不上传音频）
- MCP stdio 工具管理；只启动用户明确配置的命令，并采用手动确认调用的安全模式
- 加载模型后显示仅监听 `127.0.0.1` 的 OpenAI 兼容接口地址
- 浅色/深色主题、字号、紧凑布局、托盘驻留和空闲自动卸载
- 全局快速指令中心（`Ctrl+K`）；可搜索并跳转页面、最近对话、项目、助手和本地模型，也可直接切换已安装模型
- 对话标题栏内置模型选择器，无需离开当前对话即可加载或切换本地模型；生成过程中会锁定切换，避免损坏进行中的回答
- 模型库提供容量总览、批量移出和安全释放磁盘；多分片 GGUF 会按同量化、同总片数删除完整分片组
- 设置页展示版本、隐私、模型资产和推理引擎发布状态摘要
- 可选安装 llama.cpp 官方 CUDA 12.4/13.3 组件，校验 SHA-256 并保留 Vulkan 回退
- 导航异常保护与可恢复错误页面，避免界面故障时直接白屏
- 使用 llama.cpp `/tokenize` 的真实 Token 计数；上下文接近上限时可自动总结较早对话，并保留系统提示、最近消息和资料引用。总结可查看、编辑、清除，也可切换为仅提醒或关闭
- 独立运行组件管理：识别内置/已安装/自定义 CUDA、Vulkan 组件，显示版本与能力，可切换并一键回退；GPU 自动加载失败时在容量允许的情况下回退 CPU
- Windows 当前用户 NSIS 安装包、SHA-256 清单及可选 Authenticode 签名流程；应用内更新会同时校验 HTTPS、哈希和 Windows 签名，旧版回退还会校验与当前应用为同一签名发布者
- 本地数据一致性备份、升级前恢复点、分阶段恢复和隐私诊断包导出（诊断包不包含聊天正文）
- 桌面回归测试覆盖主导航白屏保护和生成时手动上滚释放自动跟随
- 浏览器演示模式：不启动 Tauri 也能预览完整界面和模拟数据

## 技术栈

- Tauri 2 / Rust
- React 19 / TypeScript / Vite
- SQLite（rusqlite bundled）
- reqwest + rustls
- llama.cpp `llama-server`

## 开发环境

需要：

- Windows 10 或 11 x64
- Node.js 20+
- pnpm 10+
- Rust stable（MSVC 工具链）
- Microsoft Edge WebView2 Runtime

```powershell
pnpm install
pnpm dev          # 浏览器演示模式
pnpm tauri dev    # 桌面模式
pnpm build        # 前端生产构建
pnpm build:desktop # 生成嵌入前端资源的 Windows Release EXE
pnpm build:installer # 生成 Windows 当前用户 NSIS 安装包
pnpm test         # 前端测试
cd src-tauri
cargo test        # Rust 单元测试
```

## llama.cpp 运行时

官方安装包包含 `llama.cpp b10333` Windows x64 Vulkan/CPU 与 `b10357` CUDA 13.3 运行包。为避免让源码仓库膨胀并触发 GitHub 单文件大小限制，这些二进制文件不提交到 Git；从源码构建前运行：

```powershell
pwsh -File scripts/Setup-LlamaRuntimes.ps1 -Runtime All
```

脚本只从 `ggml-org/llama.cpp` 的固定 Release 标签下载官方资产，并使用 GitHub 提供的 SHA-256 digest 校验。运行目录随后位于：

```text
src-tauri/binaries/llama-b10333-vulkan/
src-tauri/binaries/llama-b10357-cuda13.3/
```

请让 `llama-server.exe` 与同包 DLL 保持在同一目录。开发或排错时也可以用环境变量指定其他构建：

```powershell
$env:LOCALDEPLOY_LLAMA_SERVER = "D:\llama.cpp\llama-server.exe"
pnpm tauri dev
```

应用只让服务监听 `127.0.0.1` 随机端口，不会向局域网开放。

## 数据位置

默认数据位于 `%LOCALAPPDATA%\LocalDeploy`：

```text
LocalDeploy/
├── localdeploy.db       # 设置、模型索引和聊天历史
└── models/              # 默认模型目录，可在首次引导中更改
```

下载中的模型使用 `.part` 后缀，校验通过后才改为 `.gguf`。导入已有 GGUF 时不会复制原文件。

## 安全与隐私

- 不执行模型仓库中的 Python、JavaScript 或动态库
- 只允许导入扩展名和文件头均有效的 GGUF
- 自定义镜像必须使用 HTTPS
- 聊天正文不会写入运行日志
- 对话和模型数据仅保存在本机
- 退出应用时终止 `llama-server`

## Windows 发布

正式发布脚本位于 `scripts/Build-WindowsRelease.ps1`。它会执行回归测试、构建 NSIS、可选签名并生成 SHA-256；发布密钥不会保存在仓库中。正式渠道需要配置代码签名证书、HTTPS 更新清单地址和签名后的安装包。未签名的本地构建仅用于测试，不会通过应用内更新的安全校验。

## 参与贡献

欢迎提交问题、界面改进、模型源适配和运行时兼容性修复。提交前请阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 与 [SECURITY.md](SECURITY.md)。Locastra 源码采用 MIT License；随安装包分发的 llama.cpp、CUDA 运行库及其他第三方组件遵循各自许可证，详见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。

## 当前边界

当前版本的资料库采用轻量本地文本切片与关键词检索，不包含向量数据库、OCR 或复杂文档解析。图片理解需要模型本身支持视觉输入并配置匹配的 `mmproj`；语音依赖 Windows 已安装的离线语音包。MCP 工具由用户手动调用，模型不会自主执行工具。

当前版本仍不包含云端模型、模型训练、账号系统或 ARM64。受限及私人模型仓库暂不支持令牌登录。仓库已具备安装、签名和更新流水线，但正式签名证书与更新服务器属于发布方私有基础设施，不随源码或测试构建提供。

全站搜索依赖两个模型平台的公开 API；平台字段变化或网络限制会以中文错误提示呈现，不会静默执行仓库代码。
