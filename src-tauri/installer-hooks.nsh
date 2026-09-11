; Only update shortcuts belonging to this installation. Preserve /NS and /UPDATE
; behavior by leaving missing shortcuts and other installation paths untouched.
!macro SHILU_REFRESH_SHORTCUT SHORTCUT
  Push $0
  Push $1
  Push $2
  Push $3
  !insertmacro IsShortcutTarget "${SHORTCUT}" "$INSTDIR\${MAINBINARYNAME}.exe"
  Pop $0
  ${If} $0 = 1
    !insertmacro ComHlpr_CreateInProcInstance ${CLSID_ShellLink} ${IID_IShellLink} r0 ""
    ${If} $0 P<> 0
      StrCpy $1 0
      ${IUnknown::QueryInterface} $0 '("${IID_IPersistFile}",.r1)'
      ${If} $1 P<> 0
        ${IPersistFile::Load} $1 '("${SHORTCUT}", ${STGM_READWRITE})i.r2'
        ${If} $2 = 0
          ${IShellLink::SetIconLocation} $0 '(w "$INSTDIR\shilu-icon-08.ico", i 0)i.r2'
          ${If} $2 = 0
            ${IPersistFile::Save} $1 '("${SHORTCUT}",1)i.r2'
            ${If} $2 = 0
              System::Call 'shell32::SHChangeNotify(i 0x00002000, i 0x0005, w "${SHORTCUT}", p 0)'
            ${EndIf}
          ${EndIf}
        ${EndIf}
        ${IUnknown::Release} $1 ""
      ${EndIf}
      ${IUnknown::Release} $0 ""
    ${EndIf}
  ${EndIf}
  Pop $3
  Pop $2
  Pop $1
  Pop $0
!macroend

!macro NSIS_HOOK_POSTINSTALL
  Push $0
  !insertmacro SHILU_REFRESH_SHORTCUT "$DESKTOP\${PRODUCTNAME}.lnk"
  !if "${STARTMENUFOLDER}" != ""
    !insertmacro SHILU_REFRESH_SHORTCUT "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk"
  !else
    !insertmacro SHILU_REFRESH_SHORTCUT "$SMPROGRAMS\${PRODUCTNAME}.lnk"
  !endif
  WriteRegStr SHCTX "${UNINSTKEY}" "DisplayIcon" "$\"$INSTDIR\shilu-icon-08.ico$\",0"
  Pop $0
!macroend
