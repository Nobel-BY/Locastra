# Locastra Windows release pipeline

`scripts/Build-WindowsRelease.ps1` builds the current-user NSIS installer, runs the frontend regression suite, optionally signs the installer, verifies the Authenticode signature, writes SHA-256, and creates `latest.json` for the in-app updater.

Formal publishing requires two private release inputs that are intentionally not stored in this repository:

- `LOCASTRA_CERT_THUMBPRINT`: the SHA-1 thumbprint of a Windows code-signing certificate available to `signtool.exe`.
- `LOCASTRA_UPDATE_BASE_URL`: the HTTPS directory where the signed installer and `latest.json` are published.

Build the app with `LOCASTRA_UPDATE_MANIFEST_URL=https://your-domain.example/locastra/latest.json` so the About page can check that source. Locastra downloads into a `.part` file, verifies SHA-256 and a valid Authenticode signature, and only then starts the NSIS updater. A failed download or verification does not touch the installed version.

Settings also accepts a previous Locastra installer for rollback. The application requires both installers to have a valid Authenticode signature from the same certificate thumbprint and creates a timestamped database backup before launching the downgrade. Unsigned development builds intentionally cannot use this path.

User data is migrated by additive SQLite columns and Serde defaults. Before an upgrade, users can create a consistent database backup from Settings → Update, backup and diagnostics.

The 0.3.0 application/runtime compatibility baseline is pinned to llama.cpp `b10357` for downloadable CUDA components and `b10333` for the bundled Vulkan fallback. Runtime downloads never follow GitHub's moving `latest` alias; changing this baseline requires a Locastra release and regression pass.
