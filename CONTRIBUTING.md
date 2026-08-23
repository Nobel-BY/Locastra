# Contributing to Locastra

Thanks for helping make local AI easier to use on Windows.

## Before opening a pull request

1. Keep the product Chinese-first and usable without a terminal.
2. Do not add cloud model calls, telemetry, account requirements, or execute code from model repositories without an explicit design discussion.
3. Preserve the ModelScope-first, Hugging Face-compatible GGUF workflow.
4. Do not commit models, llama.cpp runtime binaries, access tokens, local databases, logs, or personal paths.
5. Add or update tests for behavioral changes.

## Development checks

```powershell
pnpm install
pnpm test
pnpm build
cd src-tauri
cargo fmt --all -- --check
cargo test
```

Desktop runtime setup is documented in `README.md`. UI changes should be checked at 1280×800 and with both light and dark themes. Respect the user's reduced-motion setting.

## Issues and pull requests

Use a focused title, explain the user-facing problem, and include reproduction steps or screenshots where applicable. Keep unrelated refactors out of bug-fix pull requests. By contributing, you agree that your contribution is licensed under the repository's MIT License.
