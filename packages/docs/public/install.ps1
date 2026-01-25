# curl-runner installer for Windows
# Usage: irm https://www.curl-runner.com/install.ps1 | iex

$ErrorActionPreference = "Stop"

$Repo = "alexvcasillas/curl-runner"
$BinaryName = "curl-runner"
$InstallDir = if ($env:CURL_RUNNER_INSTALL_DIR) { $env:CURL_RUNNER_INSTALL_DIR } else { "$env:USERPROFILE\.local\bin" }

function Write-Info { param($Message) Write-Host "[INFO] $Message" -ForegroundColor Blue }
function Write-Success { param($Message) Write-Host "[OK] $Message" -ForegroundColor Green }
function Write-Warn { param($Message) Write-Host "[WARN] $Message" -ForegroundColor Yellow }
function Write-Error { param($Message) Write-Host "[ERROR] $Message" -ForegroundColor Red; exit 1 }

function Get-LatestVersion {
    Write-Info "Fetching latest version..."
    try {
        $Release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -UseBasicParsing
        return $Release.tag_name
    } catch {
        Write-Error "Failed to fetch latest version: $_"
    }
}

function Get-Architecture {
    $Arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
    switch ($Arch) {
        "X64" { return "x64" }
        "Arm64" { return "arm64" }
        default { Write-Error "Unsupported architecture: $Arch" }
    }
}

function Get-Checksum {
    param($FilePath)
    $Hash = Get-FileHash -Path $FilePath -Algorithm SHA256
    return $Hash.Hash.ToLower()
}

function Install-CurlRunner {
    Write-Host ""
    Write-Host "curl-runner installer" -ForegroundColor Blue
    Write-Host ""

    $Arch = Get-Architecture
    $Platform = "windows-$Arch"
    Write-Info "Detected platform: $Platform"

    $Version = Get-LatestVersion
    Write-Info "Latest version: $Version"

    $ZipName = "curl-runner-cli-$Version-$Platform.zip"
    $DownloadUrl = "https://github.com/$Repo/releases/download/$Version/$ZipName"
    $ChecksumUrl = "https://github.com/$Repo/releases/download/$Version/SHA256SUMS.txt"

    $TempDir = Join-Path $env:TEMP "curl-runner-install"
    if (Test-Path $TempDir) { Remove-Item -Recurse -Force $TempDir }
    New-Item -ItemType Directory -Path $TempDir | Out-Null

    $ZipPath = Join-Path $TempDir $ZipName
    $ChecksumPath = Join-Path $TempDir "SHA256SUMS.txt"

    # Download binary
    Write-Info "Downloading $ZipName..."
    try {
        Invoke-WebRequest -Uri $DownloadUrl -OutFile $ZipPath -UseBasicParsing
    } catch {
        Write-Error "Failed to download binary: $_"
    }

    # Download checksums
    Write-Info "Downloading checksums..."
    try {
        Invoke-WebRequest -Uri $ChecksumUrl -OutFile $ChecksumPath -UseBasicParsing
    } catch {
        Write-Error "Failed to download checksums: $_"
    }

    # Verify checksum
    Write-Info "Verifying checksum..."
    $Checksums = Get-Content $ChecksumPath
    $ExpectedLine = $Checksums | Where-Object { $_ -match $ZipName }
    if (-not $ExpectedLine) {
        Write-Error "Checksum not found for $ZipName"
    }
    $ExpectedChecksum = ($ExpectedLine -split '\s+')[0].ToLower()
    $ActualChecksum = Get-Checksum -FilePath $ZipPath

    if ($ExpectedChecksum -ne $ActualChecksum) {
        Write-Error "Checksum verification failed!`nExpected: $ExpectedChecksum`nActual: $ActualChecksum"
    }
    Write-Success "Checksum verified"

    # Extract
    Write-Info "Extracting binary..."
    Expand-Archive -Path $ZipPath -DestinationPath $TempDir -Force

    # Create install directory
    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }

    # Move binary
    $BinaryPath = Join-Path $TempDir "$BinaryName.exe"
    $DestPath = Join-Path $InstallDir "$BinaryName.exe"
    Move-Item -Path $BinaryPath -Destination $DestPath -Force

    Write-Success "Installed to $DestPath"

    # Cleanup
    Remove-Item -Recurse -Force $TempDir

    # Add to PATH
    Add-ToPath

    Write-Host ""
    Write-Success "Installation complete!"
    Write-Host ""
    Write-Host "Run 'curl-runner --version' to verify installation"
    Write-Host ""
}

function Add-ToPath {
    $CurrentPath = [Environment]::GetEnvironmentVariable("PATH", "User")

    if ($CurrentPath -split ';' -contains $InstallDir) {
        return
    }

    Write-Info "Adding $InstallDir to PATH..."

    $NewPath = "$InstallDir;$CurrentPath"
    [Environment]::SetEnvironmentVariable("PATH", $NewPath, "User")

    # Update current session
    $env:PATH = "$InstallDir;$env:PATH"

    Write-Success "Added to PATH"
    Write-Warn "Restart your terminal for PATH changes to take effect"
}

Install-CurlRunner
