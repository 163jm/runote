Unicode true
!include "MUI2.nsh"

Name "Memo"
OutFile "RuNote-Setup.exe"
InstallDir "$LOCALAPPDATA\Memo"
InstallDirRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Memo" "InstallLocation"
RequestExecutionLevel user
SetCompressor /SOLID lzma

Icon "assets\app.ico"
!define MUI_ICON "assets\app.ico"
!define MUI_UNICON "assets\app.ico"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\Memo.exe"
!define MUI_FINISHPAGE_RUN_TEXT "立即运行 Memo"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "SimpChinese"

Section "Memo" SEC_MAIN
  SetOutPath "$INSTDIR"
  File "Memo.exe"

  ; 桌面快捷方式
  CreateShortcut "$DESKTOP\Memo.lnk" "$INSTDIR\Memo.exe" "" "$INSTDIR\Memo.exe"

  ; 开始菜单
  CreateDirectory "$SMPROGRAMS\Memo"
  CreateShortcut "$SMPROGRAMS\Memo\Memo.lnk" "$INSTDIR\Memo.exe" "" "$INSTDIR\Memo.exe"
  CreateShortcut "$SMPROGRAMS\Memo\卸载 Memo.lnk" "$INSTDIR\Uninstall.exe"

  ; 卸载信息（控制面板-应用）
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Memo" "DisplayName" "Memo"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Memo" "DisplayIcon" "$INSTDIR\Memo.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Memo" "UninstallString" "$\"$INSTDIR\Uninstall.exe$\""
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Memo" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Memo" "DisplayVersion" "0.2.0"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Memo" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Memo" "NoRepair" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Memo" "EstimatedSize" 9000
SectionEnd

Section "Uninstall"
  ; 注意：%APPDATA%\Memo\notes.json（便签数据）保留，不删除
  Delete "$DESKTOP\Memo.lnk"
  Delete "$SMPROGRAMS\Memo\Memo.lnk"
  Delete "$SMPROGRAMS\Memo\卸载 Memo.lnk"
  RMDir "$SMPROGRAMS\Memo"
  Delete "$INSTDIR\Memo.exe"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Memo"
SectionEnd
