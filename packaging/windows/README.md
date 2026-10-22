# Crane Windows packaging

Standalone native packaging for Crane 0.3.0. No Tauri, WebView2 bootstrapper, or Node build step.

- Product: `Crane`, identifier `app.crane.desktop` (preserved from `src-tauri/tauri.conf.json`).
- Binary: `crane.exe` (package `crane-desktop`), per-user install to `%LOCALAPPDATA%\Programs\Crane`.
- Launch points: the interactive installer offers to launch Crane when setup completes and creates per-user Start Menu and desktop shortcuts. Silent installs create the shortcuts but do not launch the application. Uninstall removes both shortcuts.
- Data: `%APPDATA%\app.crane.desktop` (`notes.json`, `backups/`, `settings.json`, `native-migration-v1.json`) is never deleted by install/upgrade/uninstall.
- Upgrade: the legacy 0.1.0 (Tauri) installer registered per-user under uninstall key `Crane` with its program in `%LOCALAPPDATA%\Crane`. The native installer runs that uninstaller silently during install, then removes any leftover legacy key/program files, so an upgrade leaves exactly one registration (`app.crane.desktop`). Takeover verified in the Task 10 cycle and re-verified for 0.3.0 in `docs/migration/windows-acceptance.md`.
- Manifest: `app.manifest` (asInvoker, PerMonitorV2 DPI, long paths). Version `0.3.0.0` must match `crane.nsi` `PRODUCT_VERSION` and `crane-desktop` version.
- Icon: Crane-owned icons are preserved under `assets/icons/` (`crane.svg` source, `crane.ico`, `crane.png`), carried over from the retired `src-tauri/icons/` in Task 10. `crane.nsi` does not embed `crane.ico` yet; until it does, builds use the default NSIS icon and must be reported as unsigned/unbranded.
- Signing: unsigned unless signing credentials and scope are supplied. Never present an unsigned installer as signed.

Build: `powershell -NoProfile -File scripts/build-windows.ps1` (locked release build, layer check, NSIS when `makensis` is available, SHA-256 + `scripts/smoke-windows.ps1` verification). If NSIS is missing the script reports the prerequisite and leaves only the verified `crane.exe` artifact.
