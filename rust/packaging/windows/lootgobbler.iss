; The Windows installer (Inno Setup 6): LootGobbler.exe of the release, installing Loot Gobbler for the current user
; (no admin prompt) into %APPDATA%\LootGobbler-Rust, beside the Java app's %APPDATA%\LootGobbler, which it never
; touches (its first start offers to take over the Java app's records from there). Another folder can be picked.
; The records are kept next to the program; uninstalling removes the program only, never the records.
; Updates come from inside the app (it swaps its own program file), so the installer is only needed once.
;
; Built by .github/workflows/release.yml:  iscc /DAppVersion=<x.y.z> /DSourceDir=<folder with LootGobbler.exe> ...
; Keep AppId fixed: a newer installer then updates the same installation.

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef SourceDir
  #define SourceDir "..\..\dist\LootGobbler"
#endif
#ifndef OutputDir
  #define OutputDir "..\..\dist\installer"
#endif

[Setup]
AppId={{6F0D3C52-8B1E-4E8A-9C59-2B7D1A4E7F31}
AppName=Loot Gobbler
AppVersion={#AppVersion}
AppVerName=Loot Gobbler {#AppVersion}
AppPublisher=Pluribus Guild
AppPublisherURL=https://deliberated.github.io/Loot-Gobbler-Client/
AppSupportURL=https://github.com/Deliberated/Loot-Gobbler-Client/issues
AppUpdatesURL=https://github.com/Deliberated/Loot-Gobbler-Client/releases/latest
DefaultDirName={userappdata}\LootGobbler-Rust
DefaultGroupName=Loot Gobbler
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
UsePreviousAppDir=yes
OutputDir={#OutputDir}
OutputBaseFilename=LootGobbler-setup
SetupIconFile=..\..\assets\icon\lootgobbler.ico
UninstallDisplayIcon={app}\LootGobbler.exe
UninstallDisplayName=Loot Gobbler
LicenseFile={#SourceDir}\LICENSE
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
CloseApplications=yes

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "{#SourceDir}\LootGobbler.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\THIRD_PARTY_LICENSE-RealmShark.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Loot Gobbler"; Filename: "{app}\LootGobbler.exe"
Name: "{group}\Uninstall Loot Gobbler"; Filename: "{uninstallexe}"
Name: "{userdesktop}\Loot Gobbler"; Filename: "{app}\LootGobbler.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\LootGobbler.exe"; Description: "Start Loot Gobbler now"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
; what the in-app updater leaves behind (the replaced program, its download folder); the records stay
Type: files; Name: "{app}\LootGobbler.exe.old"
Type: filesandordirs; Name: "{app}\update"
