; editlicious – "Open with" on Windows (design 012 E2).
; Tauri's fileAssociations would make editlicious the DEFAULT program for every listed extension
; (.txt, .json, .js …). A text editor should only offer itself, so on Windows the associations are
; switched off (tauri.windows.conf.json) and this hook adds the app to the "Open with" list of
; the user (HKCU, matches NSIS installMode currentUser). The MSI package does not do this.

!macro ED_OPEN_WITH EXT
  WriteRegStr HKCU "Software\Classes\Applications\${MAINBINARYNAME}.exe\SupportedTypes" "${EXT}" ""
  WriteRegStr HKCU "Software\Classes\${EXT}\OpenWithList\${MAINBINARYNAME}.exe" "" ""
!macroend

!macro ED_OPEN_WITH_REMOVE EXT
  DeleteRegKey HKCU "Software\Classes\${EXT}\OpenWithList\${MAINBINARYNAME}.exe"
!macroend

!macro ED_ALL_TYPES MACRO
  !insertmacro ${MACRO} ".txt"
  !insertmacro ${MACRO} ".text"
  !insertmacro ${MACRO} ".log"
  !insertmacro ${MACRO} ".md"
  !insertmacro ${MACRO} ".csv"
  !insertmacro ${MACRO} ".tsv"
  !insertmacro ${MACRO} ".json"
  !insertmacro ${MACRO} ".jsonl"
  !insertmacro ${MACRO} ".xml"
  !insertmacro ${MACRO} ".yaml"
  !insertmacro ${MACRO} ".yml"
  !insertmacro ${MACRO} ".toml"
  !insertmacro ${MACRO} ".ini"
  !insertmacro ${MACRO} ".cfg"
  !insertmacro ${MACRO} ".conf"
  !insertmacro ${MACRO} ".properties"
  !insertmacro ${MACRO} ".sql"
  !insertmacro ${MACRO} ".html"
  !insertmacro ${MACRO} ".css"
  !insertmacro ${MACRO} ".js"
  !insertmacro ${MACRO} ".ts"
  !insertmacro ${MACRO} ".py"
  !insertmacro ${MACRO} ".rs"
  !insertmacro ${MACRO} ".java"
  !insertmacro ${MACRO} ".c"
  !insertmacro ${MACRO} ".h"
  !insertmacro ${MACRO} ".cpp"
  !insertmacro ${MACRO} ".cs"
  !insertmacro ${MACRO} ".go"
  !insertmacro ${MACRO} ".sh"
  !insertmacro ${MACRO} ".ps1"
  !insertmacro ${MACRO} ".bat"
!macroend

!macro NSIS_HOOK_POSTINSTALL
  WriteRegStr HKCU "Software\Classes\Applications\${MAINBINARYNAME}.exe" "FriendlyAppName" "editlicious"
  WriteRegStr HKCU "Software\Classes\Applications\${MAINBINARYNAME}.exe\shell\open\command" "" '"$INSTDIR\${MAINBINARYNAME}.exe" "%1"'
  !insertmacro ED_ALL_TYPES ED_OPEN_WITH
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  !insertmacro ED_ALL_TYPES ED_OPEN_WITH_REMOVE
  DeleteRegKey HKCU "Software\Classes\Applications\${MAINBINARYNAME}.exe"
!macroend
