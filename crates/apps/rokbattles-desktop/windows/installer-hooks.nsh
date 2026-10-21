!include LogicLib.nsh
!include "Win\RestartManager.nsh"

; Restart Manager can require PROCESS_VM_READ even when PROCESS_TERMINATE is
; allowed. After the update exit wait, recover only a single, positively
; identified instance of this installation owned by our user and session.
; Input: installed executable path. All registers and the caller's stack survive.
Function ROKBTerminateUpdateProcess
  System::Store S
  Pop $R9
  StrCpy $R8 -1 ; Restart Manager session
  StrCpy $R7 0 ; RM_PROCESS_INFO buffer
  StrCpy $R6 0 ; process handle
  StrCpy $R5 0 ; process token
  StrCpy $R4 0 ; installer token
  StrCpy $R3 0 ; process TOKEN_USER buffer
  StrCpy $R2 0 ; installer TOKEN_USER buffer

  System::Call 'RSTRTMGR::RmStartSession(*i .R8, i 0, w) i .r0'
  ${If} $0 != 0
    StrCpy $R8 -1
    Goto rokb_update_cleanup
  ${EndIf}
  !insertmacro RestartManager_RegisterFile $R8 "$R9"
  ${If} $0 != 0
    Goto rokb_update_cleanup
  ${EndIf}
  ; RM_PROCESS_INFO is 668 bytes (DWORD/FILETIME, WCHAR[256], WCHAR[64],
  ; and four 32-bit fields). It contains no pointers, on either architecture.
  System::Alloc 668
  Pop $R7
  ${If} $R7 == 0
    Goto rokb_update_cleanup
  ${EndIf}
  System::Call 'RSTRTMGR::RmGetList(i R8, *i .r1, *i 1 .r2, p R7, *i .r3) i .r0'
  ${If} $0 != 0
  ${OrIf} $2 != 1
  ${OrIf} $3 != ${RmRebootReasonPermissionDenied}
    DetailPrint "Update fallback: resource users changed or are not a single permission-denied process."
    Goto rokb_update_cleanup
  ${EndIf}
  System::Call '*$R7(i .r4, l .R1)'
  DetailPrint "Update fallback: checking process $4."

  System::Call 'kernel32::GetCurrentProcessId() i .r5'
  ${If} $4 == $5
    Goto rokb_update_cleanup
  ${EndIf}
  System::Call 'kernel32::ProcessIdToSessionId(i r4, *i .r6) i .r0'
  ${If} $0 == 0
    Goto rokb_update_cleanup
  ${EndIf}
  System::Call 'kernel32::ProcessIdToSessionId(i r5, *i .r7) i .r0'
  ${If} $0 == 0
  ${OrIf} $6 != $7
    DetailPrint "Update fallback: process is not in the installer's session."
    Goto rokb_update_cleanup
  ${EndIf}

  ; TERMINATE | QUERY_LIMITED_INFORMATION | SYNCHRONIZE. No VM_READ or ALL_ACCESS.
  ; Keep this handle through validation, termination and waiting to avoid PID reuse.
  System::Call 'kernel32::OpenProcess(i 0x101001, i 0, i r4) p .R6'
  ${If} $R6 == 0
    System::Call 'kernel32::GetLastError() i .r0'
    DetailPrint "Update fallback: cannot open process $4 for shutdown (Windows error $0)."
    Goto rokb_update_cleanup
  ${EndIf}
  System::Call 'kernel32::GetProcessTimes(p R6, *l .r6, *l, *l, *l) i .r0'
  ${If} $0 == 0
  ${OrIf} $6 != $R1
    DetailPrint "Update fallback: process identity could not be verified."
    Goto rokb_update_cleanup
  ${EndIf}
  System::Call 'kernel32::QueryFullProcessImageNameW(p R6, i 0, w .r8, *i ${NSIS_MAX_STRLEN}) i .r0'
  ${If} $0 == 0
    Goto rokb_update_cleanup
  ${EndIf}
  System::Call 'kernel32::lstrcmpiW(w r8, w R9) i .r0'
  ${If} $0 != 0
    DetailPrint "Update fallback: process $4 is not the installed application ($8)."
    Goto rokb_update_cleanup
  ${EndIf}

  ; Compare token owners as well as sessions: never shut down another user's app.
  System::Call 'advapi32::OpenProcessToken(p R6, i 8, *p .R5) i .r0'
  ${If} $0 == 0
    Goto rokb_update_cleanup
  ${EndIf}
  System::Call 'advapi32::OpenProcessToken(p -1, i 8, *p .R4) i .r0'
  ${If} $0 == 0
    Goto rokb_update_cleanup
  ${EndIf}
  System::Call 'advapi32::GetTokenInformation(p R5, i 1, p 0, i 0, *i .r1)'
  ${If} $1 <= 0
  ${OrIf} $1 > 65536
    Goto rokb_update_cleanup
  ${EndIf}
  System::Alloc $1
  Pop $R3
  ${If} $R3 == 0
    Goto rokb_update_cleanup
  ${EndIf}
  System::Call 'advapi32::GetTokenInformation(p R5, i 1, p R3, i r1, *i) i .r0'
  ${If} $0 == 0
    Goto rokb_update_cleanup
  ${EndIf}
  System::Call 'advapi32::GetTokenInformation(p R4, i 1, p 0, i 0, *i .r1)'
  ${If} $1 <= 0
  ${OrIf} $1 > 65536
    Goto rokb_update_cleanup
  ${EndIf}
  System::Alloc $1
  Pop $R2
  ${If} $R2 == 0
    Goto rokb_update_cleanup
  ${EndIf}
  System::Call 'advapi32::GetTokenInformation(p R4, i 1, p R2, i r1, *i) i .r0'
  ${If} $0 == 0
    Goto rokb_update_cleanup
  ${EndIf}
  System::Call '*$R3(p .r1)'
  System::Call '*$R2(p .r2)'
  System::Call 'advapi32::EqualSid(p r1, p r2) i .r0'
  ${If} $0 == 0
    DetailPrint "Update fallback: process $4 belongs to another user."
    Goto rokb_update_cleanup
  ${EndIf}

  DetailPrint "Update fallback: stopping verified application process $4."
  System::Call 'kernel32::TerminateProcess(p R6, i 0) i .r0'
  ${If} $0 == 0
    System::Call 'kernel32::GetLastError() i .r0'
    DetailPrint "Update fallback: TerminateProcess returned Windows error $0."
    Goto rokb_update_cleanup
  ${EndIf}
  System::Call 'kernel32::WaitForSingleObject(p R6, i 5000) i .r0'
  ${If} $0 == 0
    DetailPrint "Update fallback: verified application exited."
  ${Else}
    DetailPrint "Update fallback: exit wait returned $0. Using the normal application check."
  ${EndIf}

  rokb_update_cleanup:
  ${If} $R2 != 0
    System::Free $R2
  ${EndIf}
  ${If} $R3 != 0
    System::Free $R3
  ${EndIf}
  ${If} $R4 != 0
    System::Call 'kernel32::CloseHandle(p R4)'
  ${EndIf}
  ${If} $R5 != 0
    System::Call 'kernel32::CloseHandle(p R5)'
  ${EndIf}
  ${If} $R6 != 0
    System::Call 'kernel32::CloseHandle(p R6)'
  ${EndIf}
  ${If} $R7 != 0
    System::Free $R7
  ${EndIf}
  ${If} $R8 != -1
    System::Call 'RSTRTMGR::RmEndSession(i R8)'
  ${EndIf}
  System::Store L
FunctionEnd

; The updater launches NSIS before exiting. Let that exit finish before Tauri
; tries RmShutdown, which can fail when it races the process exiting.
; https://github.com/tauri-apps/tauri/issues/16277
!macro NSIS_HOOK_PREINSTALL
  ${If} $UpdateMode == 1
  ${AndIf} ${FileExists} "$INSTDIR\${MAINBINARYNAME}.exe"
    Push $0
    Push $1
    Push $2
    Push $3
    Push $4
    Push $5

    DetailPrint "Waiting up to 15 seconds for the previous application to exit."
    System::Call 'kernel32::GetTickCount() i .r4'
    ${Do}
      ; Restart Manager can retain exited processes in a session's resource
      ; list. Use a fresh session for each poll so a natural exit is observed.
      ; The full installed path ignores other copies with the same name.
      System::Call 'RSTRTMGR::RmStartSession(*i .r1, i 0, w) i .r0'
      ${If} $0 != 0
        DetailPrint "Update exit wait: RmStartSession returned $0. Using the normal application check."
        ${Break}
      ${EndIf}
      !insertmacro RestartManager_RegisterFile $1 "$INSTDIR\${MAINBINARYNAME}.exe"
      ${If} $0 == 0
        ; A zero-length buffer returns 234 (ERROR_MORE_DATA) while users of
        ; the file remain. Reset its input count to zero on every query.
        System::Call 'RSTRTMGR::RmGetList(i r1, *i .r2, *i 0, p 0, *i .r3) i .r0'
        ${If} $0 == 0
          DetailPrint "Previous application released the executable."
        ${ElseIf} $0 != 234
          DetailPrint "Update exit wait: RmGetList returned $0. Using the normal application check."
        ${EndIf}
      ${Else}
        DetailPrint "Update exit wait: RmRegisterResources returned $0. Using the normal application check."
      ${EndIf}
      ; End every successful session, including query/registration failures.
      System::Call 'RSTRTMGR::RmEndSession(i r1) i .r5'
      ${If} $5 != 0
        DetailPrint "Update exit wait: RmEndSession returned $5."
        ${Break}
      ${EndIf}
      ${If} $0 != 234
        ${Break}
      ${EndIf}

      System::Call 'kernel32::GetTickCount() i .r5'
      IntOp $5 $5 - $4
      ${If} $5 >= 15000
        DetailPrint "Update exit wait timed out (processes: $2, reboot reasons: $3). Using the normal application check."
        ${If} $3 == ${RmRebootReasonPermissionDenied}
          Push "$INSTDIR\${MAINBINARYNAME}.exe"
          Call ROKBTerminateUpdateProcess
        ${EndIf}
        ${Break}
      ${EndIf}
      Sleep 250
    ${Loop}

    Pop $5
    Pop $4
    Pop $3
    Pop $2
    Pop $1
    Pop $0
  ${EndIf}
  ; Always fall through to Tauri's CheckIfAppIsRunning. A timeout or API error
  ; must not bypass its shutdown/error handling or permit a locked-file update.
!macroend
