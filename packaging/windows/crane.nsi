; Crane native Windows installer (standalone NSIS, no Tauri/WebView2).
; Build via scripts/build-windows.ps1 which stages crane.exe beside this script.
; Identity preserved from legacy Tauri bundle: product Crane, identifier app.crane.desktop.
Unicode true
SetCompressor /SOLID lzma

!define PRODUCT_NAME "Crane"
!define PRODUCT_VERSION "0.3.0"
!define PRODUCT_ID "app.crane.desktop"
!define EXE_NAME "crane.exe"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_ID}"
!include "MUI2.nsh"
!include "StrFunc.nsh"
${Using:StrFunc} StrStr

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

!define MUI_FINISHPAGE_RUN "$INSTDIR\${EXE_NAME}"
!define MUI_FINISHPAGE_RUN_TEXT "Launch Crane"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "Install"
  ; Upgrade takeover: the legacy (Tauri) installer registered per-user under
  ; the product-name key "Crane" with its program in $LOCALAPPDATA\Crane
  ; (verified against the retained 0.1.0 installer). Run its uninstaller
  ; silently first so an upgrade replaces that registration instead of
  ; leaving a second entry and a stale binary. User data is not touched:
  ; the legacy uninstaller only removes its own program files.
  ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Crane" "UninstallString"
  StrCmp $0 "" legacy_takeover_done
    ExecWait '"$LOCALAPPDATA\Crane\uninstall.exe" /S _?=$LOCALAPPDATA\Crane'
    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Crane"
    Delete "$LOCALAPPDATA\Crane\crane.exe"
    Delete "$LOCALAPPDATA\Crane\uninstall.exe"
    RMDir "$LOCALAPPDATA\Crane"
  legacy_takeover_done:

  ; Refuse to overwrite a running instance silently; user must close Crane first.
  running_check:
  nsExec::ExecToStack 'tasklist /FI "IMAGENAME eq ${EXE_NAME}" /FO CSV /NH'
  Pop $0
  Pop $1
  ${StrStr} $2 $1 '"${EXE_NAME}"'
  StrCmp $2 "" running_check_done
  IfSilent running_check_abort
  MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "Crane is currently running. Close it, then click Retry to continue." IDRETRY running_check
  running_check_abort:
  SetErrorLevel 1
  Abort
  running_check_done:

  SetOutPath "$INSTDIR"
  File /oname=${EXE_NAME} "..\..\artifacts\windows\${EXE_NAME}"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayName" "${PRODUCT_NAME}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayVersion" "${PRODUCT_VERSION}"
  WriteRegStr HKCU "${UNINST_KEY}" "Publisher" "Crane contributors"
  WriteRegStr HKCU "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\${EXE_NAME}"
  WriteRegStr HKCU "${UNINST_KEY}" "UninstallString" "$INSTDIR\uninstall.exe"

  SetShellVarContext current
  CreateDirectory "$SMPROGRAMS\Crane"
  CreateShortCut "$SMPROGRAMS\Crane\Crane.lnk" "$INSTDIR\${EXE_NAME}"
  CreateShortCut "$DESKTOP\Crane.lnk" "$INSTDIR\${EXE_NAME}"
SectionEnd

Section "Uninstall"
  SetShellVarContext current
  Delete "$DESKTOP\Crane.lnk"
  Delete "$SMPROGRAMS\Crane\Crane.lnk"
  RMDir "$SMPROGRAMS\Crane"
  Delete "$INSTDIR\${EXE_NAME}"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "${UNINST_KEY}"
  ; User data ($APPDATA\app.crane.desktop: notes, backups, settings) is preserved.
SectionEnd
