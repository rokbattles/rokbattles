; Included by one architecture wrapper. No external plugins or PATH commands.
!include "LogicLib.nsh"
!include "x64.nsh"
!include "${ROK_CAPTURE_INPUT}\bootstrap.nsh"
!define ROK_MAINTENANCE "$PROGRAMFILES64\ROK Battles\.maintenance\rokbattles-capture-maintenance.exe"

!macro ROK_CHECK_EXIT
  ${If} $0 == 3010
    SetRebootFlag true
    MessageBox MB_ICONSTOP "ROK Battles capture is still in use. Restart Windows, then run this installer again. No active driver was replaced."
    SetErrorLevel 3010
    Abort
  ${ElseIf} $0 != 0
    MessageBox MB_ICONSTOP "ROK Battles maintenance could not complete safely. Close the background agent and run this installer again. Capture remains unavailable until repair succeeds."
    SetErrorLevel 1
    Abort
  ${EndIf}
!macroend

!macro ROK_POWERSHELL ACTION
  ; NSIS is a 32-bit process. Disable redirection only around the explicit native
  ; system PowerShell invocation; restore it immediately even when the command fails.
  ${DisableX64FSRedirection}
  ExecWait '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -NonInteractive -EncodedCommand ${ROK_BOOTSTRAP_${ACTION}}' $0
  ${EnableX64FSRedirection}
  !insertmacro ROK_CHECK_EXIT
!macroend

!macro ROK_START_SESSION
  !insertmacro ROK_POWERSHELL EXISTING
  Exec '"${ROK_MAINTENANCE}" session'
  IfErrors 0 +3
    SetErrorLevel 1
    Abort
  ExecWait '"${ROK_MAINTENANCE}" prepared' $0
  !insertmacro ROK_CHECK_EXIT
!macroend

!macro NSIS_HOOK_PREINSTALL
  ; Reject command-line/custom install destinations; never turn user text into an
  ; elevated executable, service ImagePath, staging directory or command argument.
  ${If} $INSTDIR != "$PROGRAMFILES64\ROK Battles"
    MessageBox MB_ICONSTOP "Capture requires the fixed Program Files\ROK Battles installation."
    SetErrorLevel 1
    Abort
  ${EndIf}
  !insertmacro ROK_POWERSHELL ROOT
  IfFileExists "${ROK_MAINTENANCE}" rok_maintenance_existing
    ; Only a genuinely new installation may bootstrap a maintenance image.
    IfFileExists "$INSTDIR\rokbattles-capture-helper.exe" rok_maintenance_missing
    IfFileExists "$INSTDIR\.capture-ready-v1" rok_maintenance_missing
    IfFileExists "$INSTDIR\.capture-maintenance" rok_maintenance_missing
    SetOutPath "$INSTDIR\.maintenance"
    SetOverwrite off
    File /oname=rokbattles-capture-maintenance.exe "${ROK_CAPTURE_INPUT}\rokbattles-capture-maintenance.exe"
    SetOverwrite on
    !insertmacro ROK_POWERSHELL INITIAL
    Goto rok_maintenance_existing
  rok_maintenance_missing:
    MessageBox MB_ICONSTOP "The protected maintenance component is missing. This installation needs an explicit full repair; no temporary elevated helper will be run."
    SetErrorLevel 1
    Abort
  rok_maintenance_existing:
  !insertmacro ROK_START_SESSION
  ; The OLD trusted session owns NativeOpen before any new maintenance bytes or
  ; app files are staged. It remains alive across both Tauri hooks.
  SetOutPath "$INSTDIR\.maintenance"
  File /oname=rokbattles-capture-maintenance.next.exe "${ROK_CAPTURE_INPUT}\rokbattles-capture-maintenance.exe"
  File /oname=payload.json "${ROK_CAPTURE_INPUT}\payload.json"
  SetOutPath "$INSTDIR"
  SetOverwrite on
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ExecWait '"${ROK_MAINTENANCE}" commit' $0
  !insertmacro ROK_CHECK_EXIT
  ; The gate/client pin the old image until exit. Wait for both handles to close
  ; before atomically promoting the verified next maintenance version.
  StrCpy $1 0
  rok_maintenance_promote:
    System::Call 'kernel32::MoveFileExW(w "$INSTDIR\.maintenance\rokbattles-capture-maintenance.next.exe", w "${ROK_MAINTENANCE}", i 9) i.r0'
    ${If} $0 == 0
      IntOp $1 $1 + 1
      ${If} $1 < 50
        Sleep 100
        Goto rok_maintenance_promote
      ${EndIf}
      SetErrorLevel 3010
      SetRebootFlag true
      Abort
    ${EndIf}
  !insertmacro ROK_POWERSHELL FINAL
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ${If} $INSTDIR != "$PROGRAMFILES64\ROK Battles"
    SetErrorLevel 1
    Abort
  ${EndIf}
  !insertmacro ROK_START_SESSION
  ExecWait '"${ROK_MAINTENANCE}" uninstall' $0
  !insertmacro ROK_CHECK_EXIT
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Only private installer state, after both exact owned SCM services were deleted.
  Delete "$INSTDIR\.capture-ready-v1"
  Delete "$INSTDIR\.capture-maintenance"
  RMDir /r "$INSTDIR\.maintenance"
  RMDir "$INSTDIR"
!macroend
