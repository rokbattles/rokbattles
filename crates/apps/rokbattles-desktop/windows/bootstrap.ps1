# Compiled into NSIS as a fixed EncodedCommand. Never read as a runtime .ps1 file.
# The generator appends exactly one fixed operation; no caller input is accepted.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$Trusted = @('S-1-5-18', 'S-1-5-32-544', 'S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464')
$ProgramFiles = [Environment]::GetFolderPath([Environment+SpecialFolder]::ProgramFiles)
$Root = Join-Path $ProgramFiles 'ROK Battles'
$Maintenance = Join-Path $Root '.maintenance'
$Image = Join-Path $Maintenance 'rokbattles-capture-maintenance.exe'
$CurrentPin = Join-Path $Maintenance 'current.sha256'
function Assert-Protected([string]$Path) {
    $Item = Get-Item -LiteralPath $Path -Force
    if (($Item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'Reparse point refused' }
    $Acl = Get-Acl -LiteralPath $Path
    if ($Trusted -notcontains $Acl.GetOwner([Security.Principal.SecurityIdentifier]).Value) { throw 'Untrusted owner' }
    $Raw = New-Object Security.AccessControl.RawSecurityDescriptor($Acl.GetSecurityDescriptorBinaryForm(), 0)
    if ($null -eq $Raw.DiscretionaryAcl) { throw 'Unrestricted DACL refused' }
    foreach ($Ace in $Raw.DiscretionaryAcl) {
        if ($Ace.AceFlags -band [Security.AccessControl.AceFlags]::InheritOnly) { continue }
        if ($Ace -isnot [Security.AccessControl.CommonAce] -or $Ace.IsCallback) { throw 'Unsupported access rule' }
        if ($Ace.AceQualifier -eq [Security.AccessControl.AceQualifier]::AccessDenied) { continue }
        if ($Ace.AceQualifier -ne [Security.AccessControl.AceQualifier]::AccessAllowed) { throw 'Unsupported grant' }
        if (([long]$Ace.AccessMask -band 0x500D0156) -ne 0 -and ($Trusted -notcontains $Ace.SecurityIdentifier.Value)) { throw 'Writable installation path' }
    }
}
function Create-Protected([string]$Path) {
    if (Test-Path -LiteralPath $Path) { Assert-Protected $Path; return }
    $Acl = New-Object Security.AccessControl.DirectorySecurity
    $Acl.SetSecurityDescriptorSddlForm('O:BAG:SYD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;0x1200a9;;;BU)')
    [IO.Directory]::CreateDirectory($Path, $Acl) | Out-Null
    Assert-Protected $Path
}
function Verify-Pin([string]$Path, [string]$Pin) {
    if ($Pin -notmatch '^[a-f0-9]{64}$') { throw 'Invalid expected hash' }
    Assert-Protected $Path
    $Stream = [IO.File]::Open($Path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
    try {
        $Hash = [Security.Cryptography.SHA256]::Create()
        try { $Actual = ([BitConverter]::ToString($Hash.ComputeHash($Stream))).Replace('-', '').ToLowerInvariant() }
        finally { $Hash.Dispose() }
        if ($Actual -cne $Pin) { throw 'Maintenance hash mismatch' }
    } finally { $Stream.Dispose() }
}
function Write-AtomicPin([string]$Pin) {
    $Pending = "$CurrentPin.next"
    if (Test-Path -LiteralPath $Pending) { Assert-Protected $Pending; [IO.File]::Delete($Pending) }
    if (Test-Path -LiteralPath $CurrentPin) { Assert-Protected $CurrentPin }
    $Bytes = [Text.Encoding]::ASCII.GetBytes($Pin)
    $File = New-Object IO.FileStream($Pending, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None, 4096, [IO.FileOptions]::WriteThrough)
    try { $File.Write($Bytes, 0, $Bytes.Length); $File.Flush($true) } finally { $File.Dispose() }
    if (Test-Path -LiteralPath $CurrentPin) { [IO.File]::Replace($Pending, $CurrentPin, $null) }
    else { [IO.File]::Move($Pending, $CurrentPin) }
    Assert-Protected $CurrentPin
    if ([IO.File]::ReadAllText($CurrentPin) -cne $Pin) { throw 'Pin persistence failed' }
}
Assert-Protected $ProgramFiles
