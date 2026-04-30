#!/usr/bin/env pwsh
# WinSH Backend Installer
# Downloads and installs uutils/coreutils (winuxcmd) for WinSH.
#
# Usage:
#   pwsh -File install-backend.ps1
#   pwsh -File install-backend.ps1 -Force
#   pwsh -File install-backend.ps1 -Version 0.8.0

param(
    [switch]$Force,
    [string]$Version = ""
)

$ErrorActionPreference = "Stop"

# Configuration
$INSTALL_DIR = Join-Path $env:LOCALAPPDATA "WinSH\backends"
$REPO = "uutils/coreutils"
$BINARY_NAME = "winuxcmd"

Write-Host "WinSH Backend Installer" -ForegroundColor Cyan
Write-Host "========================" -ForegroundColor Cyan
Write-Host ""

# Create install directory
if (-not (Test-Path $INSTALL_DIR)) {
    New-Item -ItemType Directory -Force -Path $INSTALL_DIR | Out-Null
    Write-Host "Created directory: $INSTALL_DIR" -ForegroundColor Green
}

# Check if already installed
$exePath = Join-Path $INSTALL_DIR "$BINARY_NAME.exe"
if ((Test-Path $exePath) -and -not $Force) {
    $ver = & $exePath --version 2>&1
    Write-Host "winuxcmd already installed: $ver" -ForegroundColor Green
    Write-Host "Use -Force to reinstall"
    exit 0
}

# Determine version
if (-not $Version) {
    Write-Host "Fetching latest release info from $REPO..." -ForegroundColor Yellow
    try {
        $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases/latest" -Headers @{ "User-Agent" = "WinSH-Installer" }
        $Version = $release.tag_name
        Write-Host "Latest version: $Version" -ForegroundColor Green
    } catch {
        Write-Host "Failed to fetch release info. Using default version." -ForegroundColor Yellow
        $Version = "0.0.27"
    }
}

# Build download URL
$assets = @()
try {
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases/tags/$Version" -Headers @{ "User-Agent" = "WinSH-Installer" }
    $assets = $release.assets | Where-Object { $_.name -match "windows.*x64.*zip$" }
} catch {
    Write-Host "Could not fetch release assets" -ForegroundColor Yellow
}

if ($assets.Count -eq 0) {
    # Try common naming patterns
    $patterns = @(
        "*windows*x86_64*msvc*.zip",
        "*windows*amd64*msvc*.zip",
        "*windows*x64*.zip",
        "*windows*.zip"
    )
    foreach ($pattern in $patterns) {
        $found = $release.assets | Where-Object { $_.name -like $pattern }
        if ($found) { $assets = $found; break }
    }
}

if ($assets.Count -eq 0) {
    Write-Host "ERROR: No suitable Windows binary found in release $Version" -ForegroundColor Red
    Write-Host "Please install manually:" -ForegroundColor Yellow
    Write-Host "  scoop install uutils-coreutils"
    Write-Host "  or download from: https://github.com/$REPO/releases"
    exit 1
}

$asset = $assets[0]
$downloadUrl = $asset.browser_download_url
$fileName = $asset.name
$downloadPath = Join-Path $env:TEMP $fileName

Write-Host "Downloading $fileName..." -ForegroundColor Yellow
Invoke-WebRequest -Uri $downloadUrl -OutFile $downloadPath

Write-Host "Extracting to $INSTALL_DIR..." -ForegroundColor Yellow
Expand-Archive -Path $downloadPath -DestinationPath $INSTALL_DIR -Force

# Find the executable (might be in a subdirectory)
$exeFiles = Get-ChildItem -Path $INSTALL_DIR -Recurse -Filter "*.exe" | Where-Object { $_.Name -match "coreutils|uutils|winuxcmd" }

if ($exeFiles.Count -gt 0) {
    $srcExe = $exeFiles[0].FullName
    $dstExe = Join-Path $INSTALL_DIR "$BINARY_NAME.exe"
    
    if ($srcExe -ne $dstExe) {
        Copy-Item $srcExe $dstExe -Force
    }
} else {
    # Try any .exe file
    $exeFiles = Get-ChildItem -Path $INSTALL_DIR -Recurse -Filter "*.exe"
    if ($exeFiles.Count -gt 0) {
        $srcExe = $exeFiles[0].FullName
        $dstExe = Join-Path $INSTALL_DIR "$BINARY_NAME.exe"
        if ($srcExe -ne $dstExe) {
            Copy-Item $srcExe $dstExe -Force
        }
    } else {
        Write-Host "ERROR: Could not find executable in extracted files" -ForegroundColor Red
        exit 1
    }
}

# Add to PATH
$userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($userPath -notlike "*$INSTALL_DIR*") {
    Write-Host "Adding $INSTALL_DIR to PATH..." -ForegroundColor Yellow
    [Environment]::SetEnvironmentVariable("PATH", "$INSTALL_DIR;$userPath", "User")
    $env:PATH = "$INSTALL_DIR;$env:PATH"
}

# Clean up
Remove-Item $downloadPath -Force -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "Installation complete!" -ForegroundColor Green
Write-Host "  Version: $Version"
Write-Host "  Location: $INSTALL_DIR"
Write-Host "  Binary: $BINARY_NAME.exe"
Write-Host ""
Write-Host "Test: winuxcmd --version"
Write-Host "Configure: set backend = `"winuxcmd`" in ~/.winshrc"
