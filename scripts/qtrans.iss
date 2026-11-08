#define AppName "QTrans"
#ifndef AppVersion
#define AppVersion "0.1.0"
#endif
#define AppPublisher "QTrans"
#define AppSource AddBackslash(SourcePath) + "..\target\installer-staging\QTrans"

[Setup]
AppId={{8E4C0B9A-4312-4A56-9E5E-5687B08E1A0D}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
DefaultDirName={localappdata}\Programs\QTrans
DefaultGroupName=QTrans
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputBaseFilename=QTrans-Setup-x64
UninstallDisplayIcon={app}\qtrans-gui.exe
WizardStyle=modern
Compression=lzma2
SolidCompression=yes
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "chinesetraditional"; MessagesFile: "compiler:Languages\ChineseTraditional.isl"

[Tasks]
Name: "desktopicon"; Description: "建立桌面捷徑"; GroupDescription: "其他選項："; Flags: unchecked

[Dirs]
Name: "{app}\modpacks"; Flags: uninsneveruninstall
Name: "{app}\zip"; Flags: uninsneveruninstall
Name: "{app}\work"; Flags: uninsneveruninstall
Name: "{app}\work\projects"; Flags: uninsneveruninstall
Name: "{app}\work\output"; Flags: uninsneveruninstall
Name: "{app}\work\cache"; Flags: uninsneveruninstall

[Files]
Source: "{#AppSource}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\QTrans"; Filename: "{app}\qtrans-gui.exe"; WorkingDir: "{app}"
Name: "{autodesktop}\QTrans"; Filename: "{app}\qtrans-gui.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Run]
Filename: "{app}\qtrans-gui.exe"; Description: "啟動 QTrans"; Flags: nowait postinstall skipifsilent
