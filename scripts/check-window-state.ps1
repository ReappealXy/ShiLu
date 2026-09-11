param(
    [switch]$SkipBuild,
    [string]$ManifestTool
)

$ErrorActionPreference = 'Stop'
$taskRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$taskManifest = Join-Path $taskRoot 'src-tauri\Cargo.toml'
$taskExecutable = Join-Path $taskRoot 'src-tauri\target\debug\examples\check_window_state.exe'

if (-not $SkipBuild) {
    & cargo build --manifest-path $taskManifest --example check_window_state
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

if (-not (Test-Path -LiteralPath $taskExecutable)) {
    throw "Native test executable does not exist: $taskExecutable"
}

if (-not $ManifestTool) {
    $taskSdkBin = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    $taskSdk = Get-ChildItem -LiteralPath $taskSdkBin -Directory |
        Where-Object { $_.Name -match '^\d+\.\d+\.\d+\.\d+$' } |
        Sort-Object { [version]$_.Name } -Descending |
        Where-Object { Test-Path -LiteralPath (Join-Path $_.FullName 'x64\mt.exe') } |
        Select-Object -First 1
    if (-not $taskSdk) { throw 'Windows SDK mt.exe was not found. Pass -ManifestTool with its full path.' }
    $ManifestTool = Join-Path $taskSdk.FullName 'x64\mt.exe'
}

# Cargo examples do not receive the main Tauri executable's Windows resources.
# This generated manifest activates common-controls v6 before main() is entered.
# Only the dedicated test binary is modified; production binaries are untouched.
$taskWork = Join-Path $taskRoot 'work'
New-Item -ItemType Directory -Path $taskWork -Force | Out-Null
$taskGeneratedManifest = Join-Path $taskWork 'check-window-state.manifest'
@'
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency><dependentAssembly>
    <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls"
      version="6.0.0.0" processorArchitecture="*"
      publicKeyToken="6595b64144ccf1df" language="*" />
  </dependentAssembly></dependency>
</assembly>
'@ | Set-Content -LiteralPath $taskGeneratedManifest -Encoding utf8

& $ManifestTool '-nologo' '-manifest' $taskGeneratedManifest "-outputresource:$taskExecutable;#1"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host 'Running isolated Tauri API restart checks (not tray mouse acceptance).'
& $taskExecutable
exit $LASTEXITCODE
