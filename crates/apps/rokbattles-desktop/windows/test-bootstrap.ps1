# Synthetic file-only recovery tests. No installer, service, driver or capture APIs.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$tokens = $null
$errors = $null
$source = Join-Path $PSScriptRoot 'bootstrap.ps1'
$ast = [Management.Automation.Language.Parser]::ParseFile($source, [ref]$tokens, [ref]$errors)
if ($errors.Count -ne 0) { throw ($errors | Out-String) }
# Load only the actual implementation functions. Do not evaluate its top-level
# Program Files lookup/check. Replace ACL validation with a temp-fixture boundary.
$functions = $ast.FindAll({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] }, $false)
foreach ($function in $functions) {
    if ($function.Name -ne 'Assert-Protected') { . ([ScriptBlock]::Create($function.Extent.Text)) }
}
$Fixture = Join-Path ([IO.Path]::GetTempPath()) ('rok-maintenance-test-' + [Guid]::NewGuid())
$Root = $Fixture
$Maintenance = Join-Path $Fixture '.maintenance'
$Image = Join-Path $Maintenance 'rokbattles-capture-maintenance.exe'
$CurrentPin = Join-Path $Maintenance 'current.sha256'
function Assert-Protected([string]$Path) {
    if (-not [IO.Path]::GetFullPath($Path).StartsWith($Fixture + [IO.Path]::DirectorySeparatorChar)) { throw 'Fixture escaped' }
    $item = Get-Item -LiteralPath $Path -Force
    if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Fixture link' }
}
function Assert([bool]$Value, [string]$Message) { if (-not $Value) { throw $Message } }
function Blocked { Assert (Test-Path -LiteralPath (Join-Path $Root '.capture-maintenance')) 'Capture block disappeared before FINAL' }
try {
    [IO.Directory]::CreateDirectory($Maintenance) | Out-Null
    [IO.File]::WriteAllText($Image, 'old synthetic image')
    $Old = (Get-FileHash -LiteralPath $Image -Algorithm SHA256).Hash.ToLowerInvariant()
    $Next = Join-Path $Maintenance 'rokbattles-capture-maintenance.next.exe'
    [IO.File]::WriteAllText($Next, 'new synthetic image')
    $New = (Get-FileHash -LiteralPath $Next -Algorithm SHA256).Hash.ToLowerInvariant()
    [IO.File]::WriteAllText((Join-Path $Root '.capture-maintenance'), 'blocked')
    Write-AtomicPins @($Old)
    Assert ((Find-CurrentPin) -ceq $Old) 'Initial image rejected'; Blocked
    # Crash before rename: the old image remains runnable from its fixed path.
    Verify-Pin $Next $New
    Write-AtomicPins @($Old, $New)
    Assert ((Find-CurrentPin) -ceq $Old) 'Pre-swap recovery rejected'; Blocked
    # Crash after rename: the new fixed image is accepted by the same persisted pins.
    [IO.File]::Copy($Next, $Image, $true)
    Assert ((Find-CurrentPin) -ceq $New) 'Post-swap recovery rejected'; Blocked
    [IO.File]::WriteAllText((Join-Path $Root '.capture-ready-v1'), 'invalid')
    $Rejected = $false
    try { Complete-Promotion $New } catch { $Rejected = $true }
    Assert $Rejected 'FINAL accepted an invalid ready marker'; Blocked
    # Crash after pin shrink still preserves the block.
    Write-AtomicPins @($New)
    Assert ((Find-CurrentPin) -ceq $New) 'Final pin recovery rejected'; Blocked
    [IO.File]::WriteAllText((Join-Path $Root '.capture-ready-v1'), "ROKBattlesCaptureReady:1`n")
    Complete-Promotion $New
    Assert (-not (Test-Path -LiteralPath (Join-Path $Root '.capture-maintenance'))) 'FINAL did not clear block'
    Assert ((Get-Item -LiteralPath $CurrentPin).Length -eq 65) 'Final pin was not canonical'
    foreach ($Bad in @($New, "$New`r`n", "$New`n$New`n", ($New.ToUpperInvariant() + "`n"), "$New`n`n")) {
        [IO.File]::WriteAllText($CurrentPin, $Bad)
        $Rejected = $false
        try { $null = Read-Pins } catch { $Rejected = $true }
        Assert $Rejected 'Malformed pin state accepted'
    }
    Write-AtomicPins @($New, $New)
    Assert ((Get-Item -LiteralPath $CurrentPin).Length -eq 65) 'Equal pins were not canonicalized'
    Write-Output 'Synthetic bootstrap pin/crash/FINAL tests passed; no privileged operation ran.'
} finally {
    if (Test-Path -LiteralPath $Fixture) { Remove-Item -LiteralPath $Fixture -Recurse -Force }
}
