<#
.SYNOPSIS
    Install portless on Windows.

.DESCRIPTION
    Downloads the latest (or specified) portless release, verifies the
    SHA256 checksum, copies the binary into
    %LOCALAPPDATA%\Programs\portless\, adds that directory to the
    persistent user PATH, and registers an App Paths entry so Windows
    can resolve `portless` by name.

.PARAMETER Version
    Specific release tag to install (e.g. v0.1.0). Defaults to the
    latest stable GitHub release.

.PARAMETER InstallDir
    Override the destination directory. Defaults to
    "%LOCALAPPDATA%\Programs\portless".

.EXAMPLE
    irm https://raw.githubusercontent.com/muhammad-fiaz/portless/main/scripts/install.ps1 | iex

.EXAMPLE
    .\install.ps1 -Version v0.1.0
#>

[CmdletBinding()]
param(
    [string]$Version,
    [string]$InstallDir
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$Repo = 'muhammad-fiaz/portless'

# --- 1. Determine target asset ------------------------------------------
$arch = $env:PROCESSOR_ARCHITECTURE
switch ($arch) {
    'AMD64' { $asset = 'portless-windows-amd64.zip'; $binary = 'portless-windows-amd64.exe' }
    'ARM64' { $asset = 'portless-windows-arm64.zip'; $binary = 'portless-windows-arm64.exe' }
    default {
        Write-Error "Unsupported Windows architecture: $arch. Supported: AMD64, ARM64."
        exit 1
    }
}

# --- 2. Pick install directory ------------------------------------------
if (-not $InstallDir) {
    $InstallDir = Join-Path $env:LOCALAPPDATA 'Programs\portless'
}
$InstallDir = [System.IO.Path]::GetFullPath($InstallDir)
Write-Host "Install directory: $InstallDir"

# --- 3. Resolve version --------------------------------------------------
if (-not $Version) {
    Write-Host "Resolving latest version for $Repo ..."
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
    $Version = $release.tag_name
}
if (-not $Version) {
    Write-Error "Could not determine latest version. Set -Version vX.Y.Z and re-run."
    exit 1
}
Write-Host "Installing portless $Version (Windows/$arch)"

# --- 4. Download ---------------------------------------------------------
$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("portless-install-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $tmp -Force | Out-Null
try {
    $url = "https://github.com/$Repo/releases/download/$Version/$asset"
    $zipPath = Join-Path $tmp $asset
    Write-Host "Downloading $url"
    Invoke-WebRequest -Uri $url -OutFile $zipPath -UseBasicParsing

    # --- 5. Verify checksum (optional) ----------------------------------
    $shaUrl = "https://github.com/$Repo/releases/download/$Version/$asset.sha256"
    $shaPath = Join-Path $tmp "$asset.sha256"
    try {
        Invoke-WebRequest -Uri $shaUrl -OutFile $shaPath -UseBasicParsing
        $expected = (Get-Content $shaPath).Split(' ')[0].ToLower()
        $actual = (Get-FileHash -Algorithm SHA256 $zipPath).Hash.ToLower()
        if ($expected -ne $actual) {
            Write-Error "Checksum mismatch!`n  expected: $expected`n  actual:   $actual"
            exit 1
        }
        Write-Host "Checksum OK"
    } catch {
        Write-Host "No checksum file available -- skipping verification."
    }

    # --- 6. Install ------------------------------------------------------
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    Expand-Archive -Path $zipPath -DestinationPath $tmp -Force
    $src = Join-Path $tmp $binary
    if (-not (Test-Path $src)) {
        # The zip may have a different layout; find the .exe.
        $src = Get-ChildItem -Path $tmp -Filter 'portless*.exe' -Recurse | Select-Object -First 1
        if (-not $src) { Write-Error "Could not find portless.exe in archive."; exit 1 }
        $src = $src.FullName
    }
    $dest = Join-Path $InstallDir 'portless.exe'
    Copy-Item -Path $src -Destination $dest -Force
    Write-Host "Installed: $dest"

    # --- 7. Add to user PATH --------------------------------------------
    $currentUserPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if ($currentUserPath -notlike "*$InstallDir*") {
        $newPath = if ([string]::IsNullOrEmpty($currentUserPath)) { $InstallDir } else { "$currentUserPath;$InstallDir" }
        [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
        Write-Host "Added $InstallDir to user PATH."
    } else {
        Write-Host "$InstallDir is already on user PATH."
    }
    # Also update the current process so subsequent commands find it.
    $env:Path = "$InstallDir;$env:Path"

    # --- 8. App Paths registry entry -----------------------------------
    $appPathsKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\App Paths\portless.exe'
    if (-not (Test-Path $appPathsKey)) {
        New-Item -Path $appPathsKey -Force | Out-Null
    }
    Set-ItemProperty -Path $appPathsKey -Name '(Default)' -Value $dest
    Set-ItemProperty -Path $appPathsKey -Name 'Path' -Value $InstallDir
    Write-Host "Registered App Paths\portless.exe"

    # --- 9. Verify -------------------------------------------------------
    & $dest --version
} finally {
    Remove-Item -Path $tmp -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Host "✅ portless is ready. Try:"
Write-Host "   portless trust"
Write-Host "   portless run npm run dev"
Write-Host "   # open https://<your-app>.localhost"
Write-Host ""
Write-Host "(Open a new PowerShell or Command Prompt window for the PATH change to take effect.)"
