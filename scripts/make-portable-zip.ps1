param(
    [string]$Version = "0.4.0",
    [string]$Output = "D:\Proyectos de Kiro\Maity-desktop\target\release\Maity-Portable-v$Version.zip"
)

$ErrorActionPreference = "Stop"

$root = "D:\Proyectos de Kiro\Maity-desktop"
$rel  = Join-Path $root "target\release"
$stage = Join-Path $env:TEMP "maity-portable-stage-$Version"

if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
$pkg = Join-Path $stage "Maity-Portable-v$Version"
New-Item -ItemType Directory -Force -Path $pkg | Out-Null

$mustExist = @(
    "maity-desktop.exe",
    "llama-helper.exe",
    "msvcp140.dll",
    "msvcp140_1.dll",
    "msvcp140_2.dll",
    "vcruntime140.dll",
    "vcruntime140_1.dll",
    "vcomp140.dll",
    "concrt140.dll",
    "DirectML.dll"
)

foreach ($f in $mustExist) {
    $src = Join-Path $rel $f
    if (-not (Test-Path $src)) {
        throw "Missing required artifact: $src"
    }
    Copy-Item $src $pkg -Force
}

# Copy bundled resource folders if present
foreach ($dir in @("resources", "templates")) {
    $src = Join-Path $rel $dir
    if (Test-Path $src) {
        Copy-Item -Recurse -Force $src (Join-Path $pkg $dir)
    }
}

# README for end user
$readme = @"
Maity Desktop v$Version - Edicion Portable
==========================================

Requisitos minimos:
- Windows 10 1809 o superior / Windows 11
- 4 GB RAM (8 GB recomendado)
- 4 GB de espacio libre (modelo IA + base de datos)
- Microsoft Edge WebView2 Runtime (preinstalado en Windows 11; Windows 10 lo recibe via Update)

Como ejecutar:
1. Doble clic en maity-desktop.exe
2. La primera vez descargara el modelo de IA (Gemma 3n E2B, 2.8 GB) y el de transcripcion (Parakeet, 670 MB)
3. Las descargas se guardan en %APPDATA%\com.maity.ai\

Si el WebView2 Runtime falta, descargalo aqui:
https://developer.microsoft.com/microsoft-edge/webview2/

100% local - no se envia audio ni transcripciones a servidores externos.
"@

Set-Content -Path (Join-Path $pkg "LEEME.txt") -Value $readme -Encoding UTF8

# Compress to ZIP
if (Test-Path $Output) { Remove-Item $Output -Force }
Compress-Archive -Path $pkg -DestinationPath $Output -CompressionLevel Optimal

$size = (Get-Item $Output).Length / 1MB
Write-Host "Portable ZIP created: $Output ($([math]::Round($size, 1)) MB)"

# Cleanup stage
Remove-Item -Recurse -Force $stage
