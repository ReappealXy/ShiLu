param(
    [Parameter(Mandatory = $true)]
    [string]$InstallDirectory,
    [string[]]$PreviousExecutable = @()
)

$ErrorActionPreference = 'Stop'
$projectDirectory = Split-Path -Parent $PSScriptRoot
$installPath = (Resolve-Path -LiteralPath $InstallDirectory).Path
$exePath = Join-Path $installPath 'shilu.exe'
$allowedTargets = @($exePath) + @($PreviousExecutable | ForEach-Object { [IO.Path]::GetFullPath($_) })
$iconSource = Join-Path $projectDirectory 'src-tauri\icons\icon.ico'
$iconPath = Join-Path $installPath 'shilu-icon-08.ico'
if (-not (Test-Path -LiteralPath $exePath -PathType Leaf)) {
    throw "ShiLu executable not found: $exePath"
}
if (-not (Test-Path -LiteralPath $iconSource -PathType Leaf)) {
    throw "Selected icon not found: $iconSource"
}

$shortcutRoots = @(
    [Environment]::GetFolderPath('Desktop'),
    [Environment]::GetFolderPath('CommonDesktopDirectory'),
    [Environment]::GetFolderPath('Programs'),
    (Join-Path $env:APPDATA 'Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar')
)
$shell = New-Object -ComObject WScript.Shell
$shellApp = New-Object -ComObject Shell.Application
$shortcuts = @(
    foreach ($shortcutRoot in $shortcutRoots) {
        if (-not $shortcutRoot -or -not (Test-Path -LiteralPath $shortcutRoot)) { continue }
        foreach ($file in (Get-ChildItem -LiteralPath $shortcutRoot -Filter '*.lnk' -File -Recurse)) {
            $shortcut = $shell.CreateShortcut($file.FullName)
            if ([IO.Path]::GetFileName($shortcut.TargetPath) -ine 'shilu.exe') { continue }
            if ($shortcut.TargetPath -notin $allowedTargets) { continue }
            $folder = $shellApp.NameSpace($file.DirectoryName)
            $identity = $folder.ParseName($file.Name).ExtendedProperty('System.AppUserModel.ID')
            # Require the installer identity to avoid modifying unrelated shortcuts.
            if ($identity -ne 'com.shilu.desktop') { continue }
            [pscustomobject]@{
                Path = $file.FullName
                Target = $shortcut.TargetPath
                Icon = $shortcut.IconLocation
                AppUserModelId = $identity
            }
        }
    }
)
if ($shortcuts.Count -eq 0) { throw 'No existing ShiLu installer shortcuts found.' }

$backupDirectory = Join-Path $projectDirectory ('work\shortcut-backup-' + (Get-Date -Format 'yyyyMMdd-HHmmss-fff'))
New-Item -ItemType Directory -Path $backupDirectory | Out-Null
$shortcuts | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $backupDirectory 'before.json') -Encoding utf8
for ($index = 0; $index -lt $shortcuts.Count; $index++) {
    Copy-Item -LiteralPath $shortcuts[$index].Path -Destination (Join-Path $backupDirectory "$index.lnk")
}
if (Test-Path -LiteralPath $iconPath) {
    Copy-Item -LiteralPath $iconPath -Destination (Join-Path $backupDirectory 'previous-icon.ico')
}
Copy-Item -LiteralPath $iconSource -Destination $iconPath -Force

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class ShiLuShellNotification {
    [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
    public static extern void SHChangeNotify(uint eventId, uint flags, string item1, IntPtr item2);
}
'@

$results = @(
    foreach ($item in $shortcuts) {
        $shortcut = $shell.CreateShortcut($item.Path)
        $shortcut.TargetPath = $exePath
        $shortcut.WorkingDirectory = $installPath
        $shortcut.IconLocation = "$iconPath,0"
        $shortcut.Save()
        $saved = $shell.CreateShortcut($item.Path)
        $folder = $shellApp.NameSpace((Split-Path -Parent $item.Path))
        $identity = $folder.ParseName([IO.Path]::GetFileName($item.Path)).ExtendedProperty('System.AppUserModel.ID')
        if ($saved.TargetPath -ne $exePath -or $saved.IconLocation -ne "$iconPath,0" -or $identity -ne $item.AppUserModelId) {
            throw "Shortcut verification failed; originals are backed up in $backupDirectory"
        }
        # Refresh only the changed shell item. Do not clear caches or restart Explorer.
        [ShiLuShellNotification]::SHChangeNotify(0x00002000, 0x0005, $item.Path, [IntPtr]::Zero)
        [pscustomobject]@{ Path = $item.Path; Target = $saved.TargetPath; Icon = $saved.IconLocation; AppUserModelId = $identity }
    }
)
[ShiLuShellNotification]::SHChangeNotify(0x00002000, 0x0005, $iconPath, [IntPtr]::Zero)
[pscustomobject]@{
    BackupDirectory = $backupDirectory
    IconMatchesSource = ((Get-FileHash -LiteralPath $iconPath).Hash -eq (Get-FileHash -LiteralPath $iconSource).Hash)
    Shortcuts = $results
} | ConvertTo-Json -Depth 4
