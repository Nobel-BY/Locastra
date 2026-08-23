# llama.cpp sidecar

`llama-b10357-cuda13.3/` 包含适用于 NVIDIA RTX 50 系列的官方 Windows x64 CUDA 13.3 构建；检测到 NVIDIA 驱动时优先使用。`llama-b10333-vulkan/` 是 NVIDIA/AMD/Intel 通用的 Vulkan/CPU 回退构建。请勿将任何运行目录中的配套 DLL 移出该目录。

开发时也可以设置环境变量覆盖内置版本：

```powershell
$env:LOCALDEPLOY_LLAMA_SERVER = "D:\\llama.cpp\\llama-server.exe"
pnpm tauri dev
```

应用只在 `127.0.0.1` 的随机端口启动服务，不会暴露到局域网。
