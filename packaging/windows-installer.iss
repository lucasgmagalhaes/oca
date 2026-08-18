#ifndef SourceDir
  #error SourceDir must point at the validated Windows bundle
#endif
#ifndef OutputDir
  #error OutputDir must point at the installer output directory
#endif
#ifndef AppVersion
  #error AppVersion must match the Cargo workspace version
#endif

[Setup]
AppId={{18EA5C90-C00D-4ACD-BB47-3F396527E09D}
AppName=oca
AppVersion={#AppVersion}
AppPublisher=oca
AppPublisherURL=https://github.com/lucasgmagalhaes/oca
AppSupportURL=https://github.com/lucasgmagalhaes/oca/issues
AppUpdatesURL=https://github.com/lucasgmagalhaes/oca/releases
DefaultDirName={autopf}\oca
DefaultGroupName=oca
AllowNoIcons=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=admin
PrivilegesRequiredOverridesAllowed=dialog
CloseApplications=yes
RestartApplications=no
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
LicenseFile={#SourceDir}\resources\licenses\oca.txt
UninstallDisplayIcon={app}\ui.exe
OutputDir={#OutputDir}
OutputBaseFilename=oca-x86_64-pc-windows-msvc-setup
VersionInfoVersion={#AppVersion}
VersionInfoDescription=oca video editor installer
VersionInfoProductName=oca
VersionInfoProductVersion={#AppVersion}
ShowLanguageDialog=yes

[Languages]
Name: "en"; MessagesFile: "compiler:Default.isl"
Name: "ptbr"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"

[CustomMessages]
en.DesktopIcon=Create a desktop shortcut
en.AdditionalShortcuts=Additional shortcuts:
en.LaunchOca=Launch oca
ptbr.DesktopIcon=Criar um atalho na área de trabalho
ptbr.AdditionalShortcuts=Atalhos adicionais:
ptbr.LaunchOca=Iniciar o oca

[Tasks]
Name: "desktopicon"; Description: "{cm:DesktopIcon}"; GroupDescription: "{cm:AdditionalShortcuts}"; Flags: unchecked

[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\oca"; Filename: "{app}\ui.exe"; WorkingDir: "{app}"
Name: "{autodesktop}\oca"; Filename: "{app}\ui.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Run]
Filename: "{app}\ui.exe"; Description: "{cm:LaunchOca}"; Flags: nowait postinstall skipifsilent
