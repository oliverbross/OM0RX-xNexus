; SPDX-License-Identifier: GPL-3.0-only
; A silent test executable, never a product installer. All paths are compile-time
; bound to a fresh test directory. It calls the real admission functions only.
Unicode true
RequestExecutionLevel user
SilentInstall silent
AutoCloseWindow true
Name "xNexus installer guard tests"
OutFile "${OUT_FILE}"
!addplugindir "${PLUGIN_ROOT}"
!define XN_TEST_NO_UNINSTALL
!include "${GUARD_INCLUDE}"
Section
  ${GetOptions} $CMDLINE "/HOLD" $0
  ${IfNot} ${Errors}
    FileOpen $0 "${TEST_ROOT}\hold.ready" w
    FileWrite $0 "ready"
    FileClose $0
    StrCpy $1 0
    hold_loop:
      IfFileExists "${TEST_ROOT}\hold.stop" hold_done
      IntOp $1 $1 + 1
      IntCmp $1 600 hold_done hold_wait hold_done
      hold_wait:
      Sleep 100
      Goto hold_loop
    hold_done:
    SetErrorLevel 0
    Quit
  ${EndIf}
  ClearErrors
  ${GetOptions} $CMDLINE "/SCENARIO=" $0
  ${If} ${Errors}
    SetErrorLevel 99
    Quit
  ${EndIf}
  StrCpy $9 "${TEST_ROOT}\$0.ini"
  ReadINIStr $XNPath "$9" "Test" "Path"
  ReadINIStr $XNRequireOwned "$9" "Test" "RequireOwned"
  ReadINIStr $8 "$9" "Test" "Mode"
  ${If} $8 == "closed"
    ReadINIStr $XNProcessName "$9" "Test" "ProcessName"
    Call XNCheckClosed
  ${ElseIf} $8 == "locations"
  ${OrIf} $8 == "cancel"
    StrCpy $XNSelected $XNPath
    ReadINIStr $XNRetained "$9" "Test" "Retained"
    Call XNValidateLocations
    ${If} $8 == "cancel"
    ${AndIf} $XNResult = 0
      StrCpy $XNResult 2
      StrCpy $XNReason "Cancelled before any product operation."
    ${EndIf}
  ${Else}
    Call XNValidateRoot
  ${EndIf}
  FileOpen $0 "$9.result" w
  FileWrite $0 "$XNResult$\r$\n$XNReason$\r$\n"
  FileClose $0
  SetErrorLevel $XNResult
SectionEnd
