# Third-party notices

Locastra source code is distributed under the MIT License. The Windows installer also contains third-party runtime components governed by their own terms.

## llama.cpp / ggml

- Project: https://github.com/ggml-org/llama.cpp
- License: MIT
- Bundled builds: `b10333` Vulkan/CPU and `b10357` CUDA

Copyright (c) 2023-2026 The ggml authors. The full upstream license is available at https://github.com/ggml-org/llama.cpp/blob/master/LICENSE.

## NVIDIA CUDA runtime libraries

The optional NVIDIA build contains CUDA runtime and cuBLAS redistributable binary libraries supplied with the official llama.cpp Windows release. These files are not licensed under Locastra's MIT License.

- CUDA Toolkit license: https://docs.nvidia.com/cuda/eula/index.html
- NVIDIA trademarks and product names belong to NVIDIA Corporation.

## Tauri, React and Rust/JavaScript dependencies

Locastra uses open-source dependencies listed in `package.json`, `pnpm-lock.yaml`, `src-tauri/Cargo.toml`, and `src-tauri/Cargo.lock`. Each dependency remains subject to its own license. Binary distributions must preserve all notices required by those licenses.

Locastra is not affiliated with or endorsed by ModelScope, Hugging Face, NVIDIA, Microsoft, or the llama.cpp project.
