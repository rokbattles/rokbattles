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
function Read-Pins {
    Assert-Protected $CurrentPin
    if ((Get-Item -LiteralPath $CurrentPin).Length -notin @(65, 130)) { throw 'Invalid pin length' }
    $Bytes = [IO.File]::ReadAllBytes($CurrentPin)
    if ($Bytes.Length -ne 65 -and $Bytes.Length -ne 130) { throw 'Invalid pin length' }
    $Text = [Text.Encoding]::ASCII.GetString($Bytes)
    if ($Text -cnotmatch '\A[a-f0-9]{64}\n(?:[a-f0-9]{64}\n)?\z') { throw 'Invalid pin grammar' }
    $Pins = @($Text.Substring(0, $Text.Length - 1).Split("`n"))
    if ($Pins.Count -ne @($Pins | Select-Object -Unique).Count) { throw 'Duplicate pins' }
    return $Pins
}
function Find-CurrentPin {
    foreach ($Pin in @(Read-Pins)) {
        try { Verify-Pin $Image $Pin; return $Pin } catch { }
    }
    throw 'Installed maintenance hash mismatch'
}
function Write-AtomicPins([string[]]$Pins) {
    $Pins = @($Pins | Select-Object -Unique)
    if ($Pins.Count -lt 1 -or $Pins.Count -gt 2) { throw 'Invalid pin count' }
    foreach ($Pin in $Pins) { if ($Pin -cnotmatch '\A[a-f0-9]{64}\z') { throw 'Invalid pin' } }
    $Text = ($Pins -join "`n") + "`n"
    $Pending = "$CurrentPin.next"
    if (Test-Path -LiteralPath $Pending) { Assert-Protected $Pending; [IO.File]::Delete($Pending) }
    if (Test-Path -LiteralPath $CurrentPin) { Assert-Protected $CurrentPin }
    $Bytes = [Text.Encoding]::ASCII.GetBytes($Text)
    $File = New-Object IO.FileStream($Pending, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None, 4096, [IO.FileOptions]::WriteThrough)
    try { $File.Write($Bytes, 0, $Bytes.Length); $File.Flush($true) } finally { $File.Dispose() }
    if (Test-Path -LiteralPath $CurrentPin) { [IO.File]::Replace($Pending, $CurrentPin, $null) }
    else { [IO.File]::Move($Pending, $CurrentPin) }
    $null = @(Read-Pins)
    if ([IO.File]::ReadAllText($CurrentPin) -cne $Text) { throw 'Pin persistence failed' }
}
function Complete-Promotion([string]$Pin) {
    Verify-Pin $Image $Pin
    [void](Find-CurrentPin)
    $Ready = Join-Path $Root '.capture-ready-v1'
    Assert-Protected $Ready
    if ((Get-Item -LiteralPath $Ready).Length -ne 25 -or [IO.File]::ReadAllText($Ready) -cne "ROKBattlesCaptureReady:1`n") { throw 'Helper not ready' }
    $Block = Join-Path $Root '.capture-maintenance'
    Assert-Protected $Block
    Write-AtomicPins @($Pin)
    [IO.File]::Delete($Block)
}
Assert-Protected $ProgramFiles
