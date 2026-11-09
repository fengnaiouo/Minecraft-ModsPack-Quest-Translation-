#define AppName "QTrans"
#ifndef AppVersion
#define AppVersion "0.1.4"
#endif
#ifndef UseChinese
#define UseChinese "0"
#endif
#define AppPublisher "QTrans"
#define AppSource AddBackslash(SourcePath) + "..\target\installer-staging\QTrans"
#define AppIcon AddBackslash(AppSource) + "assets\brand\qtrans-icon.ico"

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
UninstallDisplayIcon={app}\assets\brand\qtrans-icon.ico
SetupIconFile={#AppIcon}
WizardStyle=modern
Compression=lzma2
SolidCompression=yes
CloseApplications=yes
RestartApplications=no

[Languages]
#if UseChinese == "1"
Name: "chinesetraditional"; MessagesFile: "compiler:Languages\ChineseTraditional.isl"
#else
Name: "english"; MessagesFile: "compiler:Default.isl"
#endif

[CustomMessages]
#if UseChinese == "1"
chinesetraditional.UninstallDeleteUserData=是否一併刪除 QTrans 的翻譯專案、模組包與輸出資料？\n\n選擇「否」會保留這些資料，之後可手動刪除。預設不會刪除。
chinesetraditional.UninstallKeptUserData=QTrans 已解除安裝，翻譯專案與模組包資料已保留於：
chinesetraditional.UninstallFolderNotRemoved=QTrans 已解除安裝，但此資料夾仍含有非 QTrans 管理的檔案：
#else
english.UninstallDeleteUserData=Also delete QTrans translation projects, modpack ZIPs, and output data?\n\nChoose No to keep this data for manual removal. The default is to keep it.
english.UninstallKeptUserData=QTrans was uninstalled. Translation projects and modpack data were kept in:
english.UninstallFolderNotRemoved=QTrans was uninstalled, but this folder still contains files not managed by QTrans:
#endif

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Dirs]
Name: "{app}\modpacks"
Name: "{app}\zip"
Name: "{app}\work"
Name: "{app}\work\projects"
Name: "{app}\work\output"
Name: "{app}\work\cache"

[Files]
Source: "{#AppSource}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\QTrans"; Filename: "{app}\qtrans-gui.exe"; WorkingDir: "{app}"; IconFilename: "{app}\assets\brand\qtrans-icon.ico"
Name: "{autodesktop}\QTrans"; Filename: "{app}\qtrans-gui.exe"; WorkingDir: "{app}"; IconFilename: "{app}\assets\brand\qtrans-icon.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\qtrans-gui.exe"; Description: "啟動 QTrans"; Flags: nowait postinstall skipifsilent

[Code]
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  AppDir: String;
  KeptUserData: Boolean;
begin
  if CurUninstallStep = usPostUninstall then
  begin
    AppDir := ExpandConstant('{app}');
    KeptUserData := False;
    if DirExists(ExpandConstant('{app}\work')) or
       DirExists(ExpandConstant('{app}\modpacks')) or
       DirExists(ExpandConstant('{app}\zip')) then
    begin
      if UninstallSilent then
        KeptUserData := True
      else if MsgBox(CustomMessage('UninstallDeleteUserData'), mbConfirmation,
                     MB_YESNO or MB_DEFBUTTON2) = IDYES then
      begin
        DelTree(ExpandConstant('{app}\work'), True, True, True);
        DelTree(ExpandConstant('{app}\modpacks'), True, True, True);
        DelTree(ExpandConstant('{app}\zip'), True, True, True);
      end
      else
        KeptUserData := True;
    end;
    if DirExists(AppDir) and not RemoveDir(AppDir) then
    begin
      if not UninstallSilent then
      begin
        if KeptUserData then
          MsgBox(CustomMessage('UninstallKeptUserData') + #13#10 + AppDir,
                 mbInformation, MB_OK)
        else
          MsgBox(CustomMessage('UninstallFolderNotRemoved') + #13#10 + AppDir,
                 mbInformation, MB_OK);
      end;
    end;
  end;
end;
