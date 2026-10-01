# Manual acceptance only. Never called by automated workflows. An administrator
# must have separately installed and accepted Npcap before this script is used.
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if (-not $IsWindows) { throw 'Windows is required.' }
$Architecture = [System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture.ToString()
$Target = switch ($Architecture) {
    'X64' { 'x86_64-pc-windows-msvc' }
    'Arm64' { 'aarch64-pc-windows-msvc' }
    default { throw 'Run native 64-bit PowerShell on Windows x64 or ARM64.' }
}
if ($Architecture -ne [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()) {
    throw 'An emulated PowerShell process is not native runtime acceptance.'
}
$Directory = Join-Path ([Environment]::SystemDirectory) 'Npcap'
foreach ($Name in @('wpcap.dll', 'Packet.dll')) {
    $Path = Join-Path $Directory $Name
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Npcap runtime NOT RUN: missing administrator-installed $Name. No installation attempted."
    }
    $Signature = Get-AuthenticodeSignature -LiteralPath $Path
    if ($Signature.Status -ne 'Valid' -or $null -eq $Signature.SignerCertificate -or
        $Signature.SignerCertificate.Subject -notmatch '(?:^|,\s*)O=Nmap Software LLC(?:,|$)') {
        throw "Npcap runtime NOT RUN: $Name does not have a valid Nmap Software LLC publisher signature."
    }
}
$Repository = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$Previous = $env:ROKBATTLES_VERIFIED_NPCAP
try {
    $env:ROKBATTLES_VERIFIED_NPCAP = Join-Path $Directory 'wpcap.dll'
    Push-Location $Repository
    try {
        & cargo test --locked -p rokbattles-capture-adapters --target $Target --test native_dependencies npcap_preinstalled_library_loads_without_capture -- --ignored --exact
        if ($LASTEXITCODE -ne 0) { throw 'Npcap load-only acceptance failed.' }
    } finally {
        Pop-Location
    }
} finally {
    $env:ROKBATTLES_VERIFIED_NPCAP = $Previous
}
Write-Host 'Npcap DLL loading and adapter symbols passed. No pcap handle or driver operation was attempted.'
