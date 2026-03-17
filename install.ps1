$ErrorActionPreference = "Stop"

$Repo = "ashavijit/pymgr"
$File = "pymgr-windows-x86_64.zip"
$Url = "https://github.com/$Repo/releases/latest/download/$File"

$InstallDir = "$env:USERPROFILE\.cargo\bin"
if (!(Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
}

$TempZip = Join-Path $env:TEMP $File
Write-Host "Downloading pymgr from $Url..."
Invoke-WebRequest -Uri $Url -OutFile $TempZip

Write-Host "Extracting archive..."
Expand-Archive -Path $TempZip -DestinationPath $InstallDir -Force

$ExePath = Join-Path $InstallDir "pymgr.exe"
Write-Host "pymgr installed to $ExePath"

# Ensure in PATH
$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notmatch [regex]::Escape($InstallDir)) {
    Write-Host "Adding $InstallDir to your User PATH..."
    [Environment]::SetEnvironmentVariable("PATH", "$InstallDir;$UserPath", "User")
    Write-Host "Please restart your terminal to use pymgr!"
}

Remove-Item $TempZip -Force
