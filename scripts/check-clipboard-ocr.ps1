param()

$ErrorActionPreference = 'Stop'
$taskRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$taskImageDirectory = Join-Path $taskRoot 'work\clipboard-check'
New-Item -ItemType Directory -Path $taskImageDirectory -Force | Out-Null
Add-Type -AssemblyName System.Drawing
$taskBitmap = [System.Drawing.Bitmap]::new(900, 240)
$taskGraphics = [System.Drawing.Graphics]::FromImage($taskBitmap)
$taskFont = [System.Drawing.Font]::new('Arial', 36)
try {
    $taskGraphics.Clear([System.Drawing.Color]::White)
    $taskGraphics.DrawString('CLIPBOARD SCREENSHOT', $taskFont, [System.Drawing.Brushes]::Black, 30, 30)
    $taskGraphics.DrawString('SAVED IMAGE OCR TEST', $taskFont, [System.Drawing.Brushes]::Black, 30, 100)
    $taskBitmap.Save((Join-Path $taskImageDirectory 'ocr-fixture.png'), [System.Drawing.Imaging.ImageFormat]::Png)
} finally {
    $taskGraphics.Dispose()
    $taskFont.Dispose()
    $taskBitmap.Dispose()
}

# Uses a generated PNG and an isolated library; the system clipboard is untouched.
$taskPreviousFixture = $env:SHILU_OCR_TEST_IMAGE
try {
    $env:SHILU_OCR_TEST_IMAGE = Join-Path $taskImageDirectory 'ocr-fixture.png'
    & cargo test --manifest-path (Join-Path $taskRoot 'src-tauri\Cargo.toml') --lib clipboard_png_reaches_windows_ocr -- --ignored --nocapture
    $taskExitCode = $LASTEXITCODE
} finally {
    $env:SHILU_OCR_TEST_IMAGE = $taskPreviousFixture
}
exit $taskExitCode
