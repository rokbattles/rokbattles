!include LogicLib.nsh
!include "Win\RestartManager.nsh"

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

    ; Use the full installed path so another copy with the same name is ignored.
    System::Call 'RSTRTMGR::RmStartSession(*i .r1, i 0, w) i .r0'
    ${If} $0 == 0
      !insertmacro RestartManager_RegisterFile $1 "$INSTDIR\${MAINBINARYNAME}.exe"
      ${If} $0 == 0
        DetailPrint "Waiting up to 20 seconds for the previous application to exit."
        System::Call 'kernel32::GetTickCount() i .r4'
        ${Do}
          ; A zero-length buffer returns 234 (ERROR_MORE_DATA) while users of
          ; the file remain. Reset its input count to zero on every query.
          System::Call 'RSTRTMGR::RmGetList(i r1, *i .r2, *i 0, p 0, *i .r3) i .r0'
          ${If} $0 == 0
            DetailPrint "Previous application released the executable."
            ${Break}
          ${ElseIf} $0 != 234
            DetailPrint "Update exit wait: RmGetList returned $0. Using the normal application check."
            ${Break}
          ${EndIf}

          System::Call 'kernel32::GetTickCount() i .r5'
          IntOp $5 $5 - $4
          ${If} $5 >= 20000
            DetailPrint "Update exit wait timed out (processes: $2, reboot reasons: $3). Using the normal application check."
            ${Break}
          ${EndIf}
          Sleep 250
        ${Loop}
      ${Else}
        DetailPrint "Update exit wait: RmRegisterResources returned $0. Using the normal application check."
      ${EndIf}
      System::Call 'RSTRTMGR::RmEndSession(i r1) i .r0'
      ${If} $0 != 0
        DetailPrint "Update exit wait: RmEndSession returned $0."
      ${EndIf}
    ${Else}
      DetailPrint "Update exit wait: RmStartSession returned $0. Using the normal application check."
    ${EndIf}

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
