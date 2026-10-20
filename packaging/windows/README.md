# Crane Windows packaging

Standalone native packaging for Crane 0.2.0. No Tauri, WebView2 bootstrapper, or Node build step.

- Product: `Crane`, identifier `app.crane.desktop` (preserved from `src-tauri/tauri.conf.json`).
- Binary: `crane.exe` (package `crane-desktop`), per-user install to `%LOCALAPPDATA%\Programs\Crane`.
- Data: `%APPDATA%\app.crane.desktop` (`notes.json`, `backups/`, `settings.json`, `native-migration-v1.json`) is never deleted by install/upgrade/uninstall.
- Manifest: `app.manifest` (asInvoker, PerMonitorV2 DPI, long paths). Version `0.2.0.0` must match `crane.nsi` `PRODUCT_VERSION` and `crane-desktop` version.
- Icon: no Crane-owned icon is tracked yet (`assets/icons/` is empty). Add a licensed `.ico` there and reference it in `crane.nsi` before claiming branded-installer parity; builds without it use the default NSIS icon and must be reported as unsigned/unbranded.
- Signing: unsigned unless signing credentials and scope are supplied. Never present an unsigned installer as signed.

Build: `powershell -NoProfile -File scripts/build-windows.ps1` (locked release build, layer check, NSIS when `makensis` is available, SHA-256 + `scripts/smoke-windows.ps1` verification). If NSIS is missing the script reports the prerequisite and leaves only the verified `crane.exe` artifact.
