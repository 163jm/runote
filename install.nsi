Unicode true
!include "MUI2.nsh"

Name "RuNote 便签"
OutFile "RuNote-Setup.exe"
InstallDir "$LOCALAPPDATA\RuNote"
InstallDirRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuNote" "InstallLocation"
RequestExecutionLevel user
SetCompressor /SOLID lzma

Icon "assets\app.ico"
!define MUI_ICON "assets\app.ico"
!define MUI_UNICON "assets\app.ico"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\RuNote.exe"
!define MUI_FINISHPAGE_RUN_TEXT "立即运行 RuNote"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "SimpChinese"

Section "RuNote 便签" SEC_MAIN
  SetOutPath "$INSTDIR"
  File "RuNote.exe"

  ; 桌面快捷方式
  CreateShortcut "$DESKTOP\RuNote 便签.lnk" "$INSTDIR\RuNote.exe" "" "$INSTDIR\RuNote.exe"

  ; 开始菜单
  CreateDirectory "$SMPROGRAMS\RuNote"
  CreateShortcut "$SMPROGRAMS\RuNote\RuNote 便签.lnk" "$INSTDIR\RuNote.exe" "" "$INSTDIR\RuNote.exe"
  CreateShortcut "$SMPROGRAMS\RuNote\卸载 RuNote.lnk" "$INSTDIR\Uninstall.exe"

  ; 卸载信息（控制面板-应用）
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuNote" "DisplayName" "RuNote 便签"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuNote" "DisplayIcon" "$INSTDIR\RuNote.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuNote" "UninstallString" "$\"$INSTDIR\Uninstall.exe$\""
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuNote" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuNote" "DisplayVersion" "0.2.0"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuNote" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuNote" "NoRepair" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuNote" "EstimatedSize" 9000
SectionEnd

Section "Uninstall"
  ; 注意：%APPDATA%\RuNote\notes.json（便签数据）保留，不删除
  Delete "$DESKTOP\RuNote 便签.lnk"
  Delete "$SMPROGRAMS\RuNote\RuNote 便签.lnk"
  Delete "$SMPROGRAMS\RuNote\卸载 RuNote.lnk"
  RMDir "$SMPROGRAMS\RuNote"
  Delete "$INSTDIR\RuNote.exe"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuNote"
SectionEnd
