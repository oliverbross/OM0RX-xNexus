; SPDX-License-Identifier: GPL-3.0-only
; Read-only admission checks shared by the real installer and disposable harness.
; This prevents accidental mixed installs, not hostile same-user filesystem races.
!ifndef XNEXUS_OWNERSHIP_INCLUDED
!define XNEXUS_OWNERSHIP_INCLUDED
!include LogicLib.nsh
!include FileFunc.nsh
!define XN_OWNER_FILE ".om0rx-xnexus-owner.ini"
Var XNPath
Var XNCanonical
Var XNRequireOwned
Var XNOwned
Var XNResult
Var XNReason
Var XNEntry
Var XNProcessName
Var XNSelected
Var XNRetained
Var XNDepth

!macro XNGuardFunctions PREFIX
Function ${PREFIX}XNCheckRegular
  Push $0
  Push $1
  Push $2
  Push $3
  System::Call 'kernel32::GetFileAttributesW(w "$XNEntry") i.r0'
  ${If} $0 = -1
    StrCpy $XNResult 1
    StrCpy $XNReason "Required installation file is missing or inaccessible."
    Goto regular_done
  ${EndIf}
  IntOp $0 $0 & 0x410
  ${If} $0 <> 0
    StrCpy $XNResult 1
    StrCpy $XNReason "Installation files must not be directories or reparse points."
    Goto regular_done
  ${EndIf}
  System::Call 'kernel32::CreateFileW(w "$XNEntry", i 0, i 7, p 0, i 3, i 0x200000, p 0) p.r0'
  ${If} $0 = -1
    StrCpy $XNResult 1
    StrCpy $XNReason "Cannot inspect installation file ownership."
    Goto regular_done
  ${EndIf}
  System::Alloc 52
  Pop $1
  System::Call 'kernel32::GetFileInformationByHandle(p r0, p r1) i.r2'
  System::Call 'kernel32::CloseHandle(p r0)'
  IntOp $3 $1 + 40
  System::Call '*$3(i.r3)'
  System::Free $1
  ${If} $2 = 0
  ${OrIf} $3 <> 1
    StrCpy $XNResult 1
    StrCpy $XNReason "Cannot use hard-linked or unverified installation files."
  ${EndIf}
  regular_done:
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

Function ${PREFIX}XNPathOnly
  Push $0
  Push $1
  Push $2
  StrCpy $XNResult 1
  StrCpy $XNReason "Choose a canonical absolute local directory, not a drive root or network path."
  StrCpy $0 $XNPath 2 1
  StrCmp $0 ":\" 0 path_done
  StrLen $0 $XNPath
  IntCmp $0 3 path_done path_done 0
  StrCpy $1 3
  path_char:
    StrCpy $2 $XNPath 1 $1
    StrCmp $2 "" path_full
    StrCmp $2 ":" path_done
    StrCmp $2 "/" path_done
    StrCmp $2 "*" path_done
    StrCmp $2 "?" path_done
    StrCmp $2 "\" path_component
    StrCmp $2 "" path_component
    Goto path_next
    path_component:
    IntOp $0 $1 - 1
    StrCpy $0 $XNPath 1 $0
    StrCmp $0 "." path_done
    StrCmp $0 " " path_done
    path_next:
    IntOp $1 $1 + 1
    Goto path_char
  path_full:
  IntOp $0 $1 - 1
  StrCpy $0 $XNPath 1 $0
  StrCmp $0 "." path_done
  StrCmp $0 " " path_done
  System::Call 'kernel32::GetFullPathNameW(w "$XNPath", i ${NSIS_MAX_STRLEN}, w.r0, p 0) i.r1'
  ${If} $1 = 0
  ${OrIf} $1 >= ${NSIS_MAX_STRLEN}
    Goto path_done
  ${EndIf}
  StrCmp $0 $XNPath 0 path_done
  StrCpy $XNCanonical $0
  StrCpy $1 $0
  path_ancestor:
    System::Call 'kernel32::GetFileAttributesW(w r1) i.r2 ?e'
    Pop $0
    ${If} $2 = -1
      ${If} $0 <> 2
      ${AndIf} $0 <> 3
        StrCpy $XNReason "An installation path cannot be inspected."
        Goto path_done
      ${EndIf}
    ${Else}
      IntOp $0 $2 & 0x400
      ${If} $0 <> 0
        StrCpy $XNReason "Junctions and other reparse points are not installation locations."
        Goto path_done
      ${EndIf}
      IntOp $0 $2 & 0x10
      ${If} $0 = 0
        StrCpy $XNReason "An installation path component is not a directory."
        Goto path_done
      ${EndIf}
    ${EndIf}
    StrLen $0 $1
    IntCmp $0 3 path_allowed path_allowed 0
    ${GetParent} "$1" $1
    StrLen $0 $1
    ${If} $0 = 2
      StrCpy $1 "$1\"
    ${EndIf}
    Goto path_ancestor
  path_allowed:
  StrCpy $XNResult 0
  StrCpy $XNReason ""
  path_done:
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

; Recursive inspection never follows a reparse point. Unknown ordinary files stay
; untouched; foreign executables reject a mixed installation even with a marker.
Function ${PREFIX}XNScanTree
  Exch $0
  Push $1
  Push $2
  Push $3
  Push $4
  Push $5
  IntOp $XNDepth $XNDepth + 1
  ${If} $XNDepth > 32
    StrCpy $XNResult 1
    StrCpy $XNReason "Installation directory nesting exceeds the reviewed limit."
    Goto scan_return
  ${EndIf}
  ClearErrors
  FindFirst $1 $2 "$0\*"
  IfErrors scan_failed
  scan_next:
    StrCmp $2 "" scan_done
    StrCmp $2 "." scan_skip
    StrCmp $2 ".." scan_skip
    StrCpy $4 "$0\$2"
    System::Call 'kernel32::GetFileAttributesW(w r4) i.r3'
    IntOp $5 $3 & 0x400
    ${If} $3 = -1
    ${OrIf} $5 <> 0
      StrCpy $XNResult 1
      StrCpy $XNReason "Installation contents include an inaccessible entry or reparse point."
      Goto scan_done
    ${EndIf}
    IntOp $5 $3 & 0x10
    ${If} $5 <> 0
      Push $4
      Call ${PREFIX}XNScanTree
    ${Else}
      StrCpy $XNEntry $4
      Call ${PREFIX}XNCheckRegular
      ${GetFileExt} "$4" $5
      ${If} $5 == "exe"
        StrLen $3 $XNCanonical
        IntOp $3 $3 + 1
        StrCpy $5 $4 "" $3
        StrCmp $5 "OM0RX-xNexus.exe" scan_checked
        StrCmp $5 "uninstall.exe" scan_checked
        StrCmp $5 "resources\hamlib\rigctl.exe" scan_checked
        StrCmp $5 "resources\hamlib\rigctld.exe" scan_checked
        StrCmp $5 "resources\hamlib\rotctl.exe" scan_checked
        StrCmp $5 "resources\hamlib\rotctld.exe" scan_checked
        StrCpy $XNResult 1
        StrCpy $XNReason "The folder contains another program; use a separate installation directory."
      ${EndIf}
    ${EndIf}
    scan_checked:
    ${If} $XNResult <> 0
      Goto scan_done
    ${EndIf}
    scan_skip:
    FindNext $1 $2
    Goto scan_next
  scan_done:
  FindClose $1
  Goto scan_return
  scan_failed:
  StrCpy $XNResult 1
  StrCpy $XNReason "Cannot enumerate an installation directory."
  scan_return:
  IntOp $XNDepth $XNDepth - 1
  Pop $5
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

Function ${PREFIX}XNValidateRoot
  Push $0
  Push $1
  Push $2
  StrCpy $XNOwned 0
  Call ${PREFIX}XNPathOnly
  ${If} $XNResult <> 0
    Goto root_done
  ${EndIf}
  IfFileExists "$XNCanonical\${XN_OWNER_FILE}" root_owned
  ${If} $XNRequireOwned = 1
    StrCpy $XNResult 1
    StrCpy $XNReason "A matching xNexus installation ownership marker is required."
    Goto root_done
  ${EndIf}
  System::Call 'kernel32::GetFileAttributesW(w "$XNCanonical") i.r0'
  ${If} $0 = -1
    Goto root_done
  ${EndIf}
  ClearErrors
  FindFirst $0 $1 "$XNCanonical\*"
  ${If} ${Errors}
    StrCpy $XNResult 1
    StrCpy $XNReason "Cannot enumerate the selected installation directory."
    Goto root_done
  ${EndIf}
  empty_next:
    StrCmp $1 "" empty_done
    StrCmp $1 "." empty_skip
    StrCmp $1 ".." empty_skip
    StrCpy $XNResult 1
    StrCpy $XNReason "The selected folder is nonempty and is not owned by xNexus."
    Goto empty_done
    empty_skip:
    FindNext $0 $1
    Goto empty_next
  empty_done:
  FindClose $0
  Goto root_done
  root_owned:
  StrCpy $XNEntry "$XNCanonical\${XN_OWNER_FILE}"
  Call ${PREFIX}XNCheckRegular
  ${If} $XNResult <> 0
    Goto root_done
  ${EndIf}
  ReadINIStr $0 "$XNEntry" "Ownership" "Product"
  ReadINIStr $1 "$XNEntry" "Ownership" "Format"
  ReadINIStr $2 "$XNEntry" "Ownership" "Root"
  ${If} $0 != "sk.om0rx.xnexus"
  ${OrIf} $1 != "1"
  ${OrIf} $2 != $XNCanonical
    StrCpy $XNResult 1
    StrCpy $XNReason "Installation ownership marker does not match this product and location."
    Goto root_done
  ${EndIf}
  StrCpy $XNEntry "$XNCanonical\OM0RX-xNexus.exe"
  Call ${PREFIX}XNCheckRegular
  StrCpy $XNEntry "$XNCanonical\uninstall.exe"
  Call ${PREFIX}XNCheckRegular
  ${If} $XNResult <> 0
    Goto root_done
  ${EndIf}
  Push $XNCanonical
  Call ${PREFIX}XNScanTree
  ${If} $XNResult = 0
    StrCpy $XNOwned 1
  ${EndIf}
  root_done:
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

; Called before maintenance and again before writes. No registry reads or writes
; here: the template supplies the exact retained value it will act upon.
Function ${PREFIX}XNValidateLocations
  StrCpy $XNPath $XNSelected
  StrCpy $XNRequireOwned 0
  Call ${PREFIX}XNValidateRoot
  ${If} $XNResult <> 0
    Return
  ${EndIf}
  ${If} $XNRetained != ""
    ${If} $XNSelected != $XNRetained
      StrCpy $XNResult 1
      StrCpy $XNReason "Upgrade in the existing owned directory, or uninstall it normally before choosing a new location."
      Return
    ${EndIf}
    StrCpy $XNPath $XNRetained
    StrCpy $XNRequireOwned 1
    Call ${PREFIX}XNValidateRoot
  ${EndIf}
FunctionEnd

Function ${PREFIX}XNCheckClosed
  Push $0
  nsis_tauri_utils::FindProcessCurrentUser "$XNProcessName"
  Pop $0
  StrCpy $XNResult 1
  StrCpy $XNReason "Close all xNexus windows normally before installing or uninstalling."
  ; The pinned plugin reports 0 when found and 1 when not found.
  ${If} $0 = 1
    StrCpy $XNResult 0
    StrCpy $XNReason ""
  ${EndIf}
  Pop $0
FunctionEnd
!macroend
!insertmacro XNGuardFunctions ""
!ifndef XN_TEST_NO_UNINSTALL
!insertmacro XNGuardFunctions "un."
!endif
!endif
