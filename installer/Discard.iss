#ifndef AppVersion
  #error AppVersion must be supplied by scripts/build-installer.ps1
#endif

[Setup]
AppId={{D5328CCE-9114-46C0-BE78-6EB7613DB98E}
AppName=Discard
AppVersion={#AppVersion}
AppPublisher=Zvenytoh
AppPublisherURL=https://github.com/Zvenytoh/discard
AppSupportURL=https://github.com/Zvenytoh/discard/issues
AppUpdatesURL=https://github.com/Zvenytoh/discard/releases
DefaultDirName={localappdata}\Programs\Discard
DisableDirPage=yes
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir=..\target\distribution
OutputBaseFilename=Discard-Setup
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\discard.ico
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=no

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked
Name: "startup"; Description: "Launch Discard when I sign in to Windows"

[Files]
Source: "..\LICENSE"; DestDir: "{app}\licenses"; Flags: ignoreversion
Source: "..\vendor\vencord\README.md"; DestDir: "{app}\licenses"; DestName: "Vencord-source.md"; Flags: ignoreversion
Source: "..\vendor\vencord\browser.js.LEGAL.txt"; DestDir: "{app}\licenses"; Flags: ignoreversion
Source: "..\vendor\ARRPC-LICENSE"; DestDir: "{app}\licenses"; Flags: ignoreversion
Source: "..\vendor\GAMES-SOURCE.txt"; DestDir: "{app}\licenses"; Flags: ignoreversion
Source: "..\target\release\discard.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\icon.ico"; DestDir: "{app}"; DestName: "discard.ico"; Flags: ignoreversion
Source: "..\target\release\WebView2Loader.dll"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\target\dependencies\MicrosoftEdgeWebview2Setup.exe"; Flags: dontcopy

[Icons]
Name: "{autoprograms}\Discard"; Filename: "{app}\discard.exe"; IconFilename: "{app}\discard.ico"; AppUserModelID: "Zvenytoh.Discard"
Name: "{autodesktop}\Discard"; Filename: "{app}\discard.exe"; IconFilename: "{app}\discard.ico"; AppUserModelID: "Zvenytoh.Discard"; Tasks: desktopicon
Name: "{userstartup}\Discard"; Filename: "{app}\discard.exe"; IconFilename: "{app}\discard.ico"; AppUserModelID: "Zvenytoh.Discard"; Tasks: startup

[InstallDelete]
Type: files; Name: "{userstartup}\Discard.lnk"; Tasks: not startup

[Run]
Filename: "{app}\discard.exe"; Description: "Launch Discard"; Flags: nowait postinstall skipifsilent
Filename: "{app}\discard.exe"; Flags: nowait skipifnotsilent; Check: IsUpdaterInstall

[Code]
{ Inno Setup 6 runs a 32-bit installer, including in 64-bit install mode. }
procedure NotifyShellPath(EventId: Integer; Flags: Cardinal; Item1: string; Item2: LongWord);
  external 'SHChangeNotify@shell32.dll stdcall';
procedure NotifyShellIcons(EventId: Integer; Flags: Cardinal; Item1, Item2: LongWord);
  external 'SHChangeNotify@shell32.dll stdcall';

procedure CurStepChanged(CurStep: TSetupStep);
var
  PinnedDirectory, ShortcutPath, TargetPath: string;
  Entry: TFindRec;
  Shell, Shortcut: Variant;
begin
  if CurStep <> ssPostInstall then Exit;
  { Refresh Explorer's icon cache after replacing the installed artwork. }
  NotifyShellIcons($08000000, $2000, 0, 0);
  NotifyShellPath($2000, $2005, ExpandConstant('{app}\discard.ico'), 0);
  NotifyShellPath($2000, $2005, ExpandConstant('{app}\discard.exe'), 0);
  { Windows 11 also needs a notification for the existing pinned shortcut. }
  try
    Shell := CreateOleObject('WScript.Shell');
  except
    Log('Pinned icon refresh unavailable: ' + GetExceptionMessage);
    Exit;
  end;
  PinnedDirectory := ExpandConstant('{userappdata}\Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar\');
  if not FindFirst(PinnedDirectory + '*.lnk', Entry) then Exit;
  try
    repeat
      ShortcutPath := PinnedDirectory + Entry.Name;
      try
        Shortcut := Shell.CreateShortcut(ShortcutPath);
        TargetPath := Shortcut.TargetPath;
        if CompareText(ExpandFileName(TargetPath),
          ExpandConstant('{app}\discard.exe')) = 0 then
          NotifyShellPath($2000, $2005, ShortcutPath, 0);
      except
        Log('Could not inspect pinned shortcut: ' + ShortcutPath);
      end;
    until not FindNext(Entry);
  finally
    FindClose(Entry);
  end;
end;

function IsUpdaterInstall: Boolean;
begin
  Result := WizardSilent and (ExpandConstant('{param:DISCARDUPDATE|0}') = '1');
end;

function HasRuntimeAt(Root: Integer; Key: String): Boolean;
var
  Version: String;
begin
  Result := RegQueryStringValue(Root, Key, 'pv', Version) and
    (Version <> '') and (Version <> '0.0.0.0');
end;

function HasWebView2: Boolean;
var
  Key: String;
begin
  Key := 'Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}';
  Result := HasRuntimeAt(HKCU32, Key) or HasRuntimeAt(HKLM32, Key);
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ExitCode: Integer;
begin
  Result := '';
  if not HasWebView2 then
  begin
    ExtractTemporaryFile('MicrosoftEdgeWebview2Setup.exe');
    if not Exec(ExpandConstant('{tmp}\MicrosoftEdgeWebview2Setup.exe'),
      '/silent /install', '', SW_HIDE, ewWaitUntilTerminated, ExitCode) then
      Result := 'Microsoft Edge WebView2 could not start installing. Check your Internet connection and try again.'
    else if not HasWebView2 then
      Result := 'Microsoft Edge WebView2 is required. Install the Evergreen Runtime, then run Discard Setup again.';
  end;
end;
