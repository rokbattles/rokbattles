; Included by one architecture wrapper. No external plugins or PATH commands.
!include "LogicLib.nsh"
!include "x64.nsh"
!include "${ROK_CAPTURE_INPUT}\bootstrap.nsh"
Var RokInstallerMutex
Var RokCommandBuffer
Var RokCommandCursor
Var RokStartupInfo
Var RokProcessInfo
Var RokBootstrapProcess
Var RokBootstrapThread
Var RokLaunchResult
Var RokEnvironmentBuffer
Var RokEnvironmentCursor
!define ROK_MAINTENANCE "$PROGRAMFILES64\ROK Battles\.maintenance\rokbattles-capture-maintenance.exe"

!macro ROK_CHECK_EXIT
  ${If} ${Errors}
    SetErrorLevel 1
    Abort
  ${EndIf}
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

!macro ROK_ENV_ENTRY ENTRY
  System::Call 'kernel32::lstrcpyW(p $RokEnvironmentCursor, w "${ENTRY}")'
  System::Call 'kernel32::lstrlenW(p $RokEnvironmentCursor) i.r1'
  IntOp $1 $1 + 1
  IntOp $1 $1 * 2
  IntOp $RokEnvironmentCursor $RokEnvironmentCursor + $1
!macroend

!macro ROK_POWERSHELL ACTION
  ; Stock NSIS runtime strings can be only 1024 characters. Assemble the fixed
  ; command in bounded native memory using generated <=512-character chunks.
  ; No command or script path is accepted from a caller or read from a temp file.
  ClearErrors
  System::Alloc 32768
  Pop $RokCommandBuffer
  System::Alloc 68
  Pop $RokStartupInfo
  System::Alloc 16
  Pop $RokProcessInfo
  System::Alloc 16384
  Pop $RokEnvironmentBuffer
  ${If} $RokCommandBuffer == 0
  ${OrIf} $RokStartupInfo == 0
  ${OrIf} $RokProcessInfo == 0
  ${OrIf} $RokEnvironmentBuffer == 0
    StrCpy $0 1
    Goto rok_ps_cleanup_${ACTION}
  ${EndIf}
  System::Call 'kernel32::lstrcpyW(p $RokCommandBuffer, w "$\"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe$\" -NoLogo -NoProfile -NonInteractive -EncodedCommand ")'
  System::Call 'kernel32::lstrlenW(p $RokCommandBuffer) i.r1'
  IntOp $1 $1 * 2
  IntOp $RokCommandCursor $RokCommandBuffer + $1
  !insertmacro ROK_APPEND_BOOTSTRAP_${ACTION}
  ; Never inherit profiler, CLR, PSModulePath or arbitrary loader environment
  ; from the unelevated process that launched this installer. Entries are sorted
  ; and the zeroed remainder supplies the second terminating UTF-16 NUL.
  System::Call 'ntdll::RtlZeroMemory(p $RokEnvironmentBuffer, i 16384)'
  StrCpy $RokEnvironmentCursor $RokEnvironmentBuffer
  !insertmacro ROK_ENV_ENTRY "COMSPEC=$SYSDIR\cmd.exe"
  !insertmacro ROK_ENV_ENTRY "PATH=$SYSDIR"
  !insertmacro ROK_ENV_ENTRY "PSModulePath=$SYSDIR\WindowsPowerShell\v1.0\Modules"
  !insertmacro ROK_ENV_ENTRY "SystemRoot=$WINDIR"
  !insertmacro ROK_ENV_ENTRY "WINDIR=$WINDIR"
  ; STARTUPINFOW/PROCESS_INFORMATION layouts of the 32-bit NSIS process.
  System::Call 'ntdll::RtlZeroMemory(p $RokStartupInfo, i 68)'
  System::Call 'ntdll::RtlZeroMemory(p $RokProcessInfo, i 16)'
  System::Call '*$RokStartupInfo(i 68)'
  ${DisableX64FSRedirection}
  System::Call 'kernel32::CreateProcessW(w "$SYSDIR\WindowsPowerShell\v1.0\powershell.exe", p $RokCommandBuffer, p 0, p 0, i 0, i 0x08000400, p $RokEnvironmentBuffer, w "$SYSDIR\WindowsPowerShell\v1.0", p $RokStartupInfo, p $RokProcessInfo) i.r0'
  StrCpy $RokLaunchResult $0
  ${EnableX64FSRedirection}
  StrCpy $0 $RokLaunchResult
  ${If} $0 == 0
    StrCpy $0 1
    Goto rok_ps_cleanup_${ACTION}
  ${EndIf}
  System::Call '*$RokProcessInfo(p.r1, p.r2)'
  StrCpy $RokBootstrapProcess $1
  StrCpy $RokBootstrapThread $2
  System::Call 'kernel32::CloseHandle(p $RokBootstrapThread)'
  System::Call 'kernel32::WaitForSingleObject(p $RokBootstrapProcess, i 120000) i.r0'
  ${If} $0 == 0
    System::Call 'kernel32::GetExitCodeProcess(p $RokBootstrapProcess, *i.r0) i.r1'
    ${If} $1 == 0
      StrCpy $0 1
    ${EndIf}
  ${Else}
    ; Only our fixed bootstrap child, retained by its creation handle, is stopped.
    System::Call 'kernel32::TerminateProcess(p $RokBootstrapProcess, i 1)'
    System::Call 'kernel32::WaitForSingleObject(p $RokBootstrapProcess, i 5000)'
    StrCpy $0 1
  ${EndIf}
  System::Call 'kernel32::CloseHandle(p $RokBootstrapProcess)'
  rok_ps_cleanup_${ACTION}:
  System::Free $RokCommandBuffer
  System::Free $RokStartupInfo
  System::Free $RokProcessInfo
  System::Free $RokEnvironmentBuffer
  !insertmacro ROK_CHECK_EXIT
!macroend

!macro ROK_LOCK_INSTALLER
  ; Tauri's NSIS executable/System plugin are 32-bit on both Windows targets.
  ; The namespace is admin-owned; a pre-existing object is checked by native code
  ; before NSIS acquires it. Retain ownership until the installer process exits.
  System::Call 'advapi32::ConvertStringSecurityDescriptorToSecurityDescriptorW(w "O:BAD:P(A;;GA;;;SY)(A;;GA;;;BA)", i 1, *p.r1, p 0) i.r0'
  ${If} $0 == 0
    SetErrorLevel 1
    Abort
  ${EndIf}
  System::Call '*(i 12, p r1, i 0) p.r2'
  System::Call 'kernel32::CreateMutexExW(p r2, w "Global\ROKBattles.Capture.InstallSession.v1", i 0, i 0x00120001) p.r3'
  System::Free $2
  System::Call 'kernel32::LocalFree(p r1)'
  ${If} $3 == 0
    SetErrorLevel 1
    Abort
  ${EndIf}
  StrCpy $RokInstallerMutex $3
  ClearErrors
  ExecWait '"${ROK_MAINTENANCE}" validate-installer-lock' $0
  !insertmacro ROK_CHECK_EXIT
  System::Call 'kernel32::WaitForSingleObject(p $RokInstallerMutex, i 0) i.r0'
  ${If} $0 != 0
  ${AndIf} $0 != 128
    MessageBox MB_ICONSTOP "Another ROK Battles installer is still running. Finish it before retrying."
    SetErrorLevel 1
    Abort
  ${EndIf}
!macroend

!macro ROK_START_SESSION
  !insertmacro ROK_POWERSHELL EXISTING
  SetOutPath "$INSTDIR\.maintenance"
  !insertmacro ROK_LOCK_INSTALLER
  ClearErrors
  Exec '"${ROK_MAINTENANCE}" session'
  IfErrors 0 +3
    SetErrorLevel 1
    Abort
  ClearErrors
  ExecWait '"${ROK_MAINTENANCE}" prepared' $0
  !insertmacro ROK_CHECK_EXIT
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro CheckIfAppIsRunning "${MAINBINARYNAME}.exe" "${PRODUCTNAME}"
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
  ClearErrors
  ExecWait '"${ROK_MAINTENANCE}" commit' $0
  !insertmacro ROK_CHECK_EXIT
  !insertmacro ROK_POWERSHELL PREPROMOTE
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
  !insertmacro CheckIfAppIsRunning "${MAINBINARYNAME}.exe" "${PRODUCTNAME}"
  ${If} $INSTDIR != "$PROGRAMFILES64\ROK Battles"
    SetErrorLevel 1
    Abort
  ${EndIf}
  !insertmacro ROK_START_SESSION
  ClearErrors
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
