; Windows installer for Git Simple GUI, built with Inno Setup 6 (https://jrsoftware.org/isinfo.php).
;
; Built by the GitHub workflow (.github/workflows/release.yml) after `cargo build --release`:
;   iscc /DAppVersion=1.2.0 installer\setup.iss
; The result is dist\GitSimpleGUI-Setup.exe.
;
; - Installs for the current user only (no administrator rights needed), so the app can
;   update itself: it downloads this installer and runs it with /SILENT.
; - Asks for a language (English, French, German, Spanish) and writes the choice to
;   {app}\language.ini; the app starts in that language until another one is picked in it.
;   A silent update reuses the language chosen the first time.

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

#define AppName "Git Simple GUI"
#define AppExe "git-simple-gui.exe"

[Setup]
; Never change AppId: Windows recognises updates of the same app with it.
AppId={{8F3C2A51-6E2B-4C1D-9A57-2B7E4D9C1F03}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher=Pierre Bocquillon
AppPublisherURL=https://github.com/pierreBocquillon/git-simple-gui
AppSupportURL=https://github.com/pierreBocquillon/git-simple-gui/issues
DefaultDirName={localappdata}\Programs\{#AppName}
DisableProgramGroupPage=yes
DisableDirPage=auto
PrivilegesRequired=lowest
OutputDir=..\dist
OutputBaseFilename=GitSimpleGUI-Setup
UninstallDisplayIcon={app}\{#AppExe}
SetupIconFile=..\assets\icon.ico
LicenseFile=..\LICENSE
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
; Always show the language list (not only when the system language is not one of ours).
ShowLanguageDialog=yes
; Close the running app before replacing it.
CloseApplications=yes

[Languages]
; The names (en, fr, de, es) are the app's language codes: {language} is written to language.ini.
Name: "en"; MessagesFile: "compiler:Default.isl"
Name: "fr"; MessagesFile: "compiler:Languages\French.isl"
Name: "de"; MessagesFile: "compiler:Languages\German.isl"
Name: "es"; MessagesFile: "compiler:Languages\Spanish.isl"

[CustomMessages]
en.DesktopIcon=Create a desktop shortcut
fr.DesktopIcon=Créer une icône sur le Bureau
de.DesktopIcon=Desktop-Verknüpfung erstellen
es.DesktopIcon=Crear un acceso directo en el escritorio
en.LaunchApp=Launch {#AppName}
fr.LaunchApp=Lancer {#AppName}
de.LaunchApp={#AppName} starten
es.LaunchApp=Iniciar {#AppName}

[Tasks]
Name: "desktopicon"; Description: "{cm:DesktopIcon}"; Flags: unchecked

[Files]
Source: "..\target\release\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion

[INI]
Filename: "{app}\language.ini"; Section: "app"; Key: "language"; String: "{language}"

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Run]
; Also runs after a silent install (an update), to reopen the app.
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchApp}"; Flags: nowait postinstall
