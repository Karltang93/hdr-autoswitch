; HDR Auto-Switch's deliberately current-user, in-place NSIS installer.
; Template inputs/registration layout were checked against Tauri CLI 2.11.4:
; crates/tauri-bundler/src/bundle/windows/nsis/{installer.nsi,utils.nsh}.
; Adapted Tauri packaging conventions: see LICENSE-Tauri.txt (MIT).
; Retain only safe Tauri shortcut helpers. The dangerous process macro is
; removed at preprocessing time; an accidental invocation is a compile error.
; Never run an old uninstaller: shipped v1 NSIS kills by basename.

Unicode true
ManifestDPIAware true
RequestExecutionLevel user
AllowRootDirInstall false
SetCompressor /SOLID lzma

!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"
!include "Win\COM.nsh"
!include "Win\Propkey.nsh"
!include "utils.nsh"
!macroundef CheckIfAppIsRunning

!define PRODUCTNAME "{{product_name}}"
!define MANUFACTURER "{{manufacturer}}"
!define MAINBINARYNAME "{{main_binary_name}}"
!define MAINBINARYSRCPATH "{{main_binary_path}}"
!define BUNDLEID "{{bundle_id}}"
!define PRODUCTKEY "Software\${MANUFACTURER}\${PRODUCTNAME}"
!define UNINSTKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCTNAME}"
!define WEBVIEW2APPGUID "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"

!if "{{install_mode}}" != "currentUser"
  !error "HDR's ownership-safe NSIS template supports the shipped currentUser mode only"
!endif
!if "${PRODUCTNAME}" != "HDR Auto-Switch"
  !error "Review legacy installed-product identity before renaming this product"
!endif
!if "${MANUFACTURER}" != "soptik"
  !error "Review legacy installed-product identity before changing its publisher"
!endif
!if "${MAINBINARYNAME}" != "tauri-app"
  !error "Review predecessor and startup ownership before changing the main binary name"
!endif

Name "${PRODUCTNAME}"
OutFile "{{out_file}}"
InstallDir "$LOCALAPPDATA\${PRODUCTNAME}"
VIProductVersion "{{version_with_build}}"
VIAddVersionKey "ProductName" "${PRODUCTNAME}"
VIAddVersionKey "FileDescription" "${PRODUCTNAME}"
VIAddVersionKey "FileVersion" "{{version}}"
VIAddVersionKey "ProductVersion" "{{version}}"
VIAddVersionKey "LegalCopyright" "{{copyright}}"
BrandingText "${PRODUCTNAME}"

{{#if installer_icon}}
!define MUI_ICON "{{installer_icon}}"
{{/if}}
{{#if uninstaller_icon}}
!define MUI_UNICON "{{uninstaller_icon}}"
{{/if}}
{{#if uninstaller_sign_cmd}}
!uninstfinalize '{{uninstaller_sign_cmd}}'
{{/if}}

Var HdrUiLevel
Var HdrPassive
Var HdrNoShortcut
Var HdrArguments

!define MUI_PAGE_CUSTOMFUNCTION_PRE HdrSkipPassive
!insertmacro MUI_PAGE_WELCOME
{{#if license}}
!define MUI_PAGE_CUSTOMFUNCTION_PRE HdrSkipPassive
!insertmacro MUI_PAGE_LICENSE "{{license}}"
{{/if}}
!define MUI_PAGE_CUSTOMFUNCTION_PRE HdrSkipPassive
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_NOAUTOCLOSE
!define MUI_FINISHPAGE_RUN
!define MUI_FINISHPAGE_RUN_FUNCTION HdrRun
!define MUI_PAGE_CUSTOMFUNCTION_PRE HdrSkipPassive
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
{{#each languages}}
!insertmacro MUI_LANGUAGE "{{this}}"
{{/each}}

Function .onInit
  SetShellVarContext current
  ${If} ${RunningX64}
    !if "{{arch}}" == "x64"
      SetRegView 64
    !else if "{{arch}}" == "arm64"
      SetRegView 64
    !else
      SetRegView 32
    !endif
  ${EndIf}
  ReadRegStr $0 HKCU "${PRODUCTKEY}" ""
  ${If} $0 != ""
    StrCpy $INSTDIR $0
  ${EndIf}
  StrCpy $HdrUiLevel 5
  ${If} ${Silent}
    StrCpy $HdrUiLevel 2
  ${EndIf}
  ${GetOptions} $CMDLINE "/P" $HdrPassive
  ${IfNot} ${Errors}
    StrCpy $HdrPassive 1
    StrCpy $HdrUiLevel 2
  ${EndIf}
  ${GetOptions} $CMDLINE "/NS" $HdrNoShortcut
  ${IfNot} ${Errors}
    StrCpy $HdrNoShortcut 1
  ${EndIf}
FunctionEnd

Function HdrSkipPassive
  ${If} $HdrPassive == 1
    Abort
  ${EndIf}
FunctionEnd

Section "Verify orderly handoff" HdrGuard
  ; This helper exits before Tauri/plugins/config/HDR initialization. Do not
  ; replace it with an invocation of the installed v1 binary.
  InitPluginsDir
  SetOutPath "$PLUGINSDIR"
  File "/oname=hdr-install-guard.exe" "${MAINBINARYSRCPATH}"
  ClearErrors
  ExecWait '"$PLUGINSDIR\hdr-install-guard.exe" --hdr-installer-preflight nsis "$INSTDIR\${MAINBINARYNAME}.exe" $HdrUiLevel' $0
  ${If} ${Errors}
    StrCpy $0 1
  ${EndIf}
  ${If} $0 != 0
    DetailPrint "Installation blocked. Quit HDR Auto-Switch from its tray menu, then retry. No process was terminated."
    SetErrorLevel 1
    Quit
  ${EndIf}
SectionEnd

Section "WebView2 runtime" HdrWebView
  ; Retain the shipped default WebView bootstrap behavior, after our guard.
  ${If} ${RunningX64}
    ReadRegStr $0 HKLM "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\${WEBVIEW2APPGUID}" "pv"
  ${Else}
    ReadRegStr $0 HKLM "SOFTWARE\Microsoft\EdgeUpdate\Clients\${WEBVIEW2APPGUID}" "pv"
  ${EndIf}
  ${If} $0 == ""
    ReadRegStr $0 HKCU "SOFTWARE\Microsoft\EdgeUpdate\Clients\${WEBVIEW2APPGUID}" "pv"
  ${EndIf}
  ${If} $0 == ""
    !if "{{install_webview2_mode}}" == "downloadBootstrapper"
      NSISdl::download "https://go.microsoft.com/fwlink/p/?LinkId=2124703" "$PLUGINSDIR\MicrosoftEdgeWebview2Setup.exe"
      Pop $0
      ${If} $0 != "success"
        Abort "Unable to download Microsoft WebView2. No application files were replaced."
      ${EndIf}
      ExecWait '"$PLUGINSDIR\MicrosoftEdgeWebview2Setup.exe" {{webview2_installer_args}} /install' $0
      ${If} $0 != 0
        Abort "Microsoft WebView2 installation failed. No application files were replaced."
      ${EndIf}
    !else if "{{install_webview2_mode}}" != "skip"
      !error "Review WebView2 packaging before changing the shipped bootstrap mode"
    !endif
  ${EndIf}
SectionEnd

Section "HDR Auto-Switch" HdrInstall
  ; Recheck after the potentially long WebView download and directly before
  ; replacement. Retry never means permission to force termination.
  ClearErrors
  ExecWait '"$PLUGINSDIR\hdr-install-guard.exe" --hdr-installer-preflight nsis "$INSTDIR\${MAINBINARYNAME}.exe" $HdrUiLevel' $0
  ${If} ${Errors}
    StrCpy $0 1
  ${EndIf}
  ${If} $0 != 0
    SetErrorLevel 1
    Quit
  ${EndIf}

  SetOutPath "$INSTDIR"
  SetOverwrite on
  ClearErrors
  File "${MAINBINARYSRCPATH}"
  ${If} ${Errors}
    Abort "The application executable could not be replaced. The upgrade was not completed."
  ${EndIf}
  {{#each resources_dirs}}
  CreateDirectory "$INSTDIR\\{{this}}"
  {{/each}}
  {{#each resources}}
  File /a "/oname={{this.[1]}}" "{{no-escape @key}}"
  {{/each}}
  {{#each binaries}}
  File /a "/oname={{this}}" "{{no-escape @key}}"
  {{/each}}
  ; A sharing lock on only uninstall.exe must not leave the v1 killer in place
  ; while the installer reports success or offers to launch the new app.
  ClearErrors
  WriteUninstaller "$INSTDIR\uninstall.exe"
  ${If} ${Errors}
    SetErrorLevel 1
    Abort "The safe uninstaller could not be written. This upgrade did not complete. Do not run the old uninstaller; close the file lock and retry this installer."
  ${EndIf}
  WriteRegStr HKCU "${PRODUCTKEY}" "" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTKEY}" "MainBinaryName" "${MAINBINARYNAME}.exe"
  WriteRegStr HKCU "${UNINSTKEY}" "DisplayName" "${PRODUCTNAME}"
  WriteRegStr HKCU "${UNINSTKEY}" "Publisher" "${MANUFACTURER}"
  WriteRegStr HKCU "${UNINSTKEY}" "DisplayVersion" "{{version}}"
  WriteRegStr HKCU "${UNINSTKEY}" "DisplayIcon" '$\"$INSTDIR\${MAINBINARYNAME}.exe$\"'
  WriteRegStr HKCU "${UNINSTKEY}" "InstallLocation" '$\"$INSTDIR$\"'
  WriteRegStr HKCU "${UNINSTKEY}" "UninstallString" '$\"$INSTDIR\uninstall.exe$\"'
  WriteRegDWORD HKCU "${UNINSTKEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTKEY}" "NoRepair" 1
  WriteRegDWORD HKCU "${UNINSTKEY}" "EstimatedSize" "{{estimated_size}}"

  ; Verify the newly written ownership metadata instead of reporting success
  ; after a denied/partial registry write. A failed check also prevents launch.
  ClearErrors
  ExecWait '"$PLUGINSDIR\hdr-install-guard.exe" --hdr-installer-preflight nsis "$INSTDIR\${MAINBINARYNAME}.exe" $HdrUiLevel' $0
  ${If} ${Errors}
    StrCpy $0 1
  ${EndIf}
  ${If} $0 != 0
    SetErrorLevel 1
    Quit
  ${EndIf}

  ${If} $HdrNoShortcut != 1
    CreateShortcut "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    !insertmacro SetLnkAppUserModelId "$SMPROGRAMS\${PRODUCTNAME}.lnk"
    CreateShortcut "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    !insertmacro SetLnkAppUserModelId "$DESKTOP\${PRODUCTNAME}.lnk"
  ${EndIf}
  ${If} $HdrPassive == 1
    SetAutoClose true
  ${EndIf}
SectionEnd

Function HdrRun
  Exec '"$INSTDIR\${MAINBINARYNAME}.exe"'
FunctionEnd

Function .onInstSuccess
  ${If} $HdrPassive == 1
  ${OrIf} ${Silent}
    ${GetOptions} $CMDLINE "/R" $0
    ${IfNot} ${Errors}
      ${GetOptions} $CMDLINE "/ARGS" $HdrArguments
      Exec '"$INSTDIR\${MAINBINARYNAME}.exe" $HdrArguments'
    ${EndIf}
  ${EndIf}
FunctionEnd

Function un.onInit
  SetShellVarContext current
  ${If} ${RunningX64}
    !if "{{arch}}" == "x64"
      SetRegView 64
    !else if "{{arch}}" == "arm64"
      SetRegView 64
    !else
      SetRegView 32
    !endif
  ${EndIf}
  StrCpy $HdrUiLevel 5
  ${If} ${Silent}
    StrCpy $HdrUiLevel 2
  ${EndIf}
FunctionEnd

Section "Uninstall"
  ; Only the v2 uninstaller uses this command; v1 is never invoked by an upgrade.
  ; Keep the executable and ownership metadata until the check/owned startup
  ; retirement succeeds. Never delete a generic Run value here.
  ClearErrors
  ExecWait '"$INSTDIR\${MAINBINARYNAME}.exe" --hdr-installer-uninstall nsis "$INSTDIR\${MAINBINARYNAME}.exe" $HdrUiLevel' $0
  ${If} ${Errors}
    StrCpy $0 1
  ${EndIf}
  ${If} $0 != 0
    DetailPrint "Uninstall blocked: orderly exit/owned startup retirement could not be verified."
    SetErrorLevel 1
    Quit
  ${EndIf}
  Delete "$INSTDIR\${MAINBINARYNAME}.exe"
  {{#each resources}}
  Delete "$INSTDIR\\{{this.[1]}}"
  {{/each}}
  {{#each binaries}}
  Delete "$INSTDIR\\{{this}}"
  {{/each}}
  Delete "$INSTDIR\uninstall.exe"
  !insertmacro IsShortcutTarget "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
  Pop $0
  ${If} $0 == 1
    Delete "$SMPROGRAMS\${PRODUCTNAME}.lnk"
  ${EndIf}
  !insertmacro IsShortcutTarget "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
  Pop $0
  ${If} $0 == 1
    Delete "$DESKTOP\${PRODUCTNAME}.lnk"
  ${EndIf}
  DeleteRegKey HKCU "${UNINSTKEY}"
  DeleteRegValue HKCU "${PRODUCTKEY}" ""
  DeleteRegKey /ifempty HKCU "${PRODUCTKEY}"
  {{#each resources_ancestors}}
  RMDir "$INSTDIR\\{{this}}"
  {{/each}}
  RMDir "$INSTDIR"
  ; Neither legacy nor v2 user settings are removed by this installer.
SectionEnd
