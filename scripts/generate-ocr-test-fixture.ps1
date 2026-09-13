# Generates a fixed OCR capability test image. The expected text is never included in the model prompt.
Add-Type -AssemblyName System.Drawing
$fixtureDirectory = Join-Path $PSScriptRoot '..\src-tauri\fixtures'
[System.IO.Directory]::CreateDirectory($fixtureDirectory) | Out-Null
$bitmap = New-Object System.Drawing.Bitmap(640, 160)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$font = New-Object System.Drawing.Font('Arial', 48, [System.Drawing.FontStyle]::Bold)
try {
    $graphics.Clear([System.Drawing.Color]::White)
    $graphics.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
    $graphics.DrawString('SHILU 8246', $font, [System.Drawing.Brushes]::Black, 35, 42)
    $bitmap.Save((Join-Path $fixtureDirectory 'ocr-test.png'), [System.Drawing.Imaging.ImageFormat]::Png)
} finally {
    $font.Dispose()
    $graphics.Dispose()
    $bitmap.Dispose()
}
