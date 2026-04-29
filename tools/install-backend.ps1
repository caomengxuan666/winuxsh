# WinSH Backend Installer
# Downloads and installs WinuxCmd (uutils/coreutils) for WinSH.
# Run: pwsh -File install-backend.ps1

param(
    [switch]$Force,
    [string]$InstallDir = ""
)

$ErrorActionPreference = "Stop"

# Determine install directory
if (-not $InstallDir) {
    $InstallDir = Join-Path $env:LOCALAPPDATA "WinSH"
}

Write-Host "WinSH Backend Installer" -ForegroundColor Cyan
Write-Host "=========================" -ForegroundColor Cyan
Write-Host ""

# Create install directory
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
}

# Check if already installed
$backendDir = Join-Path $InstallDir "winuxcmd"
$backendExe = Join-Path $backendDir "winuxcmd.exe"

if (Test-Path $backendExe -and -not $Force) {
    $ver = & $backendExe --version 2>&1
    Write-Host "WinuxCmd already installed: $ver" -ForegroundColor Green
    Write-Host "Use -Force to reinstall."
    exit 0
}

# Download latest WinuxCmd release from GitHub
$repo = "WinuxCmd/WinuxCmd"
$apiUrl = "https://api.github.com/repos/$repo/releases/latest"

Write-Host "Fetching latest release info from $repo..." -ForegroundColor Yellow

try {
    $release = Invoke-RestMethod -Uri $apiUrl -Headers @{ "User-Agent" = "WinSH-Installer" }
} catch {
    Write-Host "Failed to fetch release info. Trying uutils/coreutils..." -ForegroundColor Yellow
    
    # Try uutils/coreutils as fallback
    $repo = "uutils/coreutils"
    $apiUrl = "https://api.github.com/repos/$repo/releases/latest"
    
    try {
        $release = Invoke-RestMethod -Uri $apiUrl -Headers @{ "User-Agent" = "WinSH-Installer" }
    } catch {
        Write-Host "ERROR: Cannot fetch release info. Please install manually:" -ForegroundColor Red
        Write-Host "  scoop install uutils-coreutils"
        Write-Host "  or download from: https://github.com/$repo/releases"
        exit 1
    }
}

$version = $release.tag_name
Write-Host "Latest version: $version" -ForegroundColor Green

# Find Windows x64 asset
$asset = $release.assets | Where-Object { 
    $_.name -match "x86_64.*windows|win.*x64|win.*64" -and $_.name -match "\.zip$" 
} | Select-Object -First 1

if (-not $asset) {
    # Try any zip
    $asset = $release.assets | Where-Object { $_.name -match "\.zip$" } | Select-Object -First 1
}

if (-not $asset) {
    Write-Host "ERROR: No suitable Windows binary found in release." -ForegroundColor Red
    Write-Host "Please install manually from: $($release.html_url)"
    exit 1
}

$downloadUrl = $asset.browser_download_url
$fileName = $asset.name
$downloadPath = Join-Path $env:TEMP $fileName

Write-Host "Downloading $fileName ..." -ForegroundColor Yellow
Invoke-WebRequest -Uri $downloadUrl -OutFile $downloadPath

Write-Host "Extracting to $backendDir ..." -ForegroundColor Yellow

# Clean old installation
if (Test-Path $backendDir) {
    Remove-Item -Recurse -Force $backendDir
}
New-Item -ItemType Directory -Force -Path $backendDir | Out-Null

# Extract
Expand-Archive -Path $downloadPath -DestinationPath $backendDir -Force

# Find the actual exe (might be in a subdirectory)
$exePath = Get-ChildItem -Path $backendDir -Recurse -Filter "*.exe" -Name "coreutils.exe","winuxcmd.exe","uutils.exe" | Select-Object -First 1

if (-not $exePath) {
    # Any exe
    $exePath = Get-ChildItem -Path $backendDir -Recurse -Filter "*.exe" | Select-Object -First 1
}

if ($exePath) {
    $srcExe = Join-Path $backendDir $exePath
    $dstExe = Join-Path $backendDir "winuxcmd.exe"
    
    if ($srcExe -ne $dstExe) {
        Copy-Item $srcExe $dstExe -Force
    }
    
    # Add to PATH
    $userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
    if ($userPath -notlike "*$backendDir*") {
        Write-Host "Adding $backendDir to PATH..." -ForegroundColor Yellow
        [Environment]::SetEnvironmentVariable("PATH", "$backendDir;$userPath", "User")
        $env:PATH = "$backendDir;$env:PATH"
    }
    
    Write-Host ""
    Write-Host "Installation complete!" -ForegroundColor Green
    Write-Host "  Version: $version"
    Write-Host "  Location: $backendDir"
    Write-Host "  Added to PATH: $backendDir"
    Write-Host ""
    Write-Host "Test: winuxcmd --version"
} else {
    Write-Host "ERROR: Could not find executable in extracted files." -ForegroundColor Red
    exit 1
}

# Cleanup
Remove-Item $downloadPath -Force -ErrorAction SilentlyContinue
