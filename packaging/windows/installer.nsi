!include "MUI2.nsh"

!ifndef VERSION
  !define VERSION "0.1.0"
!endif

Name "Cover Wall Studio ${VERSION}"
OutFile "cover-wall-${VERSION}-windows-x86_64-setup.exe"
InstallDir "$PROGRAMFILES64\Cover Wall Studio"
InstallDirRegKey HKLM "Software\Cover Wall Studio" "Install_Dir"
RequestExecutionLevel admin
Unicode True

!define MUI_ABORTWARNING

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Section "Cover Wall Studio" SecMain
  SetOutPath "$INSTDIR"
  File "..\..\target\release\cover-wall.exe"
  File "..\..\LICENSE-MIT"
  File "..\..\LICENSE-APACHE"
  File "..\..\README.md"

  WriteRegStr HKLM "Software\Cover Wall Studio" "Install_Dir" "$INSTDIR"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Cover Wall Studio" "DisplayName" "Cover Wall Studio"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Cover Wall Studio" "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Cover Wall Studio" "Publisher" "SayMaven"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Cover Wall Studio" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Cover Wall Studio" "NoModify" 1
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Cover Wall Studio" "NoRepair" 1
  WriteUninstaller "$INSTDIR\uninstall.exe"

  CreateDirectory "$SMPROGRAMS\Cover Wall Studio"
  CreateShortcut "$SMPROGRAMS\Cover Wall Studio\Cover Wall Studio.lnk" "$INSTDIR\cover-wall.exe"
  CreateShortcut "$SMPROGRAMS\Cover Wall Studio\Uninstall.lnk" "$INSTDIR\uninstall.exe"
  CreateShortcut "$DESKTOP\Cover Wall Studio.lnk" "$INSTDIR\cover-wall.exe"
SectionEnd

Section "Uninstall"
  Delete "$DESKTOP\Cover Wall Studio.lnk"
  Delete "$SMPROGRAMS\Cover Wall Studio\Cover Wall Studio.lnk"
  Delete "$SMPROGRAMS\Cover Wall Studio\Uninstall.lnk"
  RMDir "$SMPROGRAMS\Cover Wall Studio"

  Delete "$INSTDIR\cover-wall.exe"
  Delete "$INSTDIR\LICENSE-MIT"
  Delete "$INSTDIR\LICENSE-APACHE"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Cover Wall Studio"
  DeleteRegKey HKLM "Software\Cover Wall Studio"
SectionEnd
