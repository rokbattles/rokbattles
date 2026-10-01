# Read-only signature verification. Never loads the DLL or installs/opens the driver.
[CmdletBinding()]
param(
    [ValidateSet('x86_64-pc-windows-msvc')]
    [string]$Target = 'x86_64-pc-windows-msvc'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if (-not $IsWindows) { throw 'Windows is required for kernel-policy verification.' }
$Desktop = Split-Path $PSScriptRoot -Parent
$Resources = Join-Path $Desktop "vendor/windivert/$Target"

# Pin checks come first, including the exact file set and all license/notices.
& python (Join-Path $PSScriptRoot 'windivert_vendor.py') verify --target $Target
if ($LASTEXITCODE -ne 0) { throw 'WinDivert resource verification failed.' }

$Dll = Join-Path $Resources 'WinDivert.dll'
$Driver = Join-Path $Resources 'WinDivert64.sys'
$DllSignature = Get-AuthenticodeSignature -LiteralPath $Dll
if ($DllSignature.Status -ne 'NotSigned') {
    throw "Unexpected official DLL signature state: $($DllSignature.Status)"
}
$DriverSignature = Get-AuthenticodeSignature -LiteralPath $Driver
if ($DriverSignature.Status -ne 'Valid' -or $null -eq $DriverSignature.TimeStamperCertificate) {
    throw "Driver requires a valid timestamped upstream signature: $($DriverSignature.Status)"
}

# Use the installed Windows SDK tool; never fetch or execute a vendor utility.
$SdkBin = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10/bin'
$SignTool = Get-ChildItem -Path "$SdkBin/*/x64/signtool.exe" -File |
    Sort-Object FullName -Descending | Select-Object -First 1
if ($null -eq $SignTool) { throw 'Windows SDK SignTool is required; verification cannot be skipped.' }
$OsVersion = [Environment]::OSVersion.Version
$PolicyVersion = "2:$($OsVersion.Major).$($OsVersion.Minor).$($OsVersion.Build)"
& $SignTool.FullName verify /kp /all /tw /v /o $PolicyVersion $Driver
if ($LASTEXITCODE -ne 0) { throw "Kernel-policy signature verification failed: $LASTEXITCODE" }

# Confirm verification did not change any bytes before handing files to the bundler.
& python (Join-Path $PSScriptRoot 'windivert_vendor.py') verify --target $Target
if ($LASTEXITCODE -ne 0) { throw 'WinDivert resource verification failed after signature check.' }
Write-Host 'Verified official hash-pinned DLL and upstream-signed kernel driver; nothing installed or activated.'
