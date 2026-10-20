; Crane native Windows installer (standalone NSIS, no Tauri/WebView2).
; Build via scripts/build-windows.ps1 which stages crane.exe beside this script.
; Identity preserved from legacy Tauri bundle: product Crane, identifier app.crane.desktop.
Unicode true
SetCompressor /SOLID lzma

!define PRODUCT_NAME "Crane"
!define PRODUCT_VERSION "0.2.0"
!define PRODUCT_ID "app.crane.desktop"
!define EXE_NAME "crane.exe"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_ID}"

Name "${PRODUCT_NAME}"
OutFile "..\..\artifacts\windows\Crane_${PRODUCT_VERSION}_x64-setup.exe"
InstallDir "$LOCALAPPDATA\Programs\Crane"
InstallDirRegKey HKCU "${UNINST_KEY}" "InstallLocation"
RequestExecutionLevel user
ManifestSupportedOS Win10
VIProductVersion "${PRODUCT_VERSION}.0"
VIAddVersionKey "ProductName" "${PRODUCT_NAME}"
VIAddVersionKey "FileDescription" "A calm, private Markdown notebook"
VIAddVersionKey "FileVersion" "${PRODUCT_VERSION}"
VIAddVersionKey "ProductVersion" "${PRODUCT_VERSION}"

Page directory
Page instfiles
UninstPage uninstConfirm
UninstPage instfiles

Section "Install"
  ; Refuse to overwrite a running instance silently; user must close Crane first.
  nsExec::ExecToStack 'tasklist /FI "IMAGENAME eq ${EXE_NAME}" /NH'
  Pop $0
  Pop $1
  StrCmp $1 "" running_check_done
  ; tasklist prints the image name when running; require manual close.
  MessageBox MB_OKCANCEL|MB_ICONEXCLAMATION "Crane is currently running. Close it, then click OK to continue." IDOK running_check_done
  Abort
  running_check_done:

  SetOutPath "$INSTDIR"
  File /oname=${EXE_NAME} "..\..\target\release\${EXE_NAME}"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayName" "${PRODUCT_NAME}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayVersion" "${PRODUCT_VERSION}"
  WriteRegStr HKCU "${UNINST_KEY}" "Publisher" "Crane contributors"
  WriteRegStr HKCU "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\${EXE_NAME}"
  WriteRegStr HKCU "${UNINST_KEY}" "UninstallString" "$INSTDIR\uninstall.exe"
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\${EXE_NAME}"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "${UNINST_KEY}"
  ; User data ($APPDATA\app.crane.desktop: notes, backups, settings) is preserved.
SectionEnd
