param(
    [switch]$SkipBuild,
    [string]$ManifestTool
)

$ErrorActionPreference = 'Stop'
$taskRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$taskManifest = Join-Path $taskRoot 'src-tauri\Cargo.toml'
$taskExecutable = Join-Path $taskRoot 'src-tauri\target\debug\examples\check_single_instance.exe'

if (-not $SkipBuild) {
    & cargo build --manifest-path $taskManifest --example check_single_instance
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

# Activate common-controls v6 only in this isolated Cargo example executable.
$taskExampleManifest = Join-Path $taskRoot 'src-tauri\examples\check_single_instance.manifest'
& $ManifestTool '-nologo' '-manifest' $taskExampleManifest "-outputresource:$taskExecutable;#1"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host 'Running isolated native single-instance checks; real ShiLu data and processes are untouched.'
& $taskExecutable
exit $LASTEXITCODE
