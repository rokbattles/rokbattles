# Compile-only NSIS harness. Never execute the generated installer or native helpers.
[CmdletBinding()]
param([Parameter(Mandatory)][ValidateSet('x86_64-pc-windows-msvc', 'aarch64-pc-windows-msvc')][string]$Target)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$Desktop = Split-Path $PSScriptRoot -Parent
$Work = Join-Path $env:RUNNER_TEMP ('rok-nsis-compile-' + [Guid]::NewGuid())
[IO.Directory]::CreateDirectory($Work) | Out-Null
try {
    $Archive = Join-Path $Work 'nsis.zip'
    # Exact official toolchain/hash used by Tauri's own Windows bundler. This is a
    # compiler dependency fetched only by CI, never by the installed application.
    Invoke-WebRequest -Uri 'https://github.com/tauri-apps/binary-releases/releases/download/nsis-3.11/nsis-3.11.zip' -OutFile $Archive
    if ((Get-Item -LiteralPath $Archive).Length -ne 2361546 -or
        (Get-FileHash -LiteralPath $Archive -Algorithm SHA256).Hash -cne 'C7D27F780DDB6CFFB4730138CD1591E841F4B7EDB155856901CDF5F214394FA1') { throw 'NSIS toolchain SHA-256 mismatch' }
    if ((Get-FileHash -LiteralPath $Archive -Algorithm SHA1).Hash -cne 'EF7FF767E5CBD9EDD22ADD3A32C9B8F4500BB10D') { throw 'NSIS upstream compatibility hash mismatch' }
    Expand-Archive -LiteralPath $Archive -DestinationPath $Work
    $Compiler = Join-Path $Work 'nsis-3.11/makensis.exe'
    $Hook = (Join-Path $PSScriptRoot "capture-$Target.nsh").Replace('$', '$$')
    $Out = (Join-Path $Work 'compile-only-never-run.exe').Replace('$', '$$')
    # The real Tauri template owns this macro; the harness checks our expansion
    # without replacing its separately reviewed normal close-application behavior.
    $Source = @'
Unicode true
RequestExecutionLevel admin
Name "ROK maintenance compile-only"
OutFile "__OUT__"
InstallDir "$PROGRAMFILES64\ROK Battles"
!define MAINBINARYNAME "rokbattles-desktop"
!define PRODUCTNAME "ROK Battles"
!macro CheckIfAppIsRunning MAIN NAME
!macroend
!include "__HOOK__"
Page instfiles
UninstPage instfiles
Section "Install"
  !insertmacro NSIS_HOOK_PREINSTALL
  !insertmacro NSIS_HOOK_POSTINSTALL
  WriteUninstaller "$INSTDIR\uninstall.exe"
SectionEnd
Section "Uninstall"
  !insertmacro NSIS_HOOK_PREUNINSTALL
  !insertmacro NSIS_HOOK_POSTUNINSTALL
SectionEnd
'@
    $Source = $Source.Replace('__HOOK__', $Hook).Replace('__OUT__', $Out)
    $Script = Join-Path $Work 'compile-only.nsi'
    [IO.File]::WriteAllText($Script, $Source, [Text.UTF8Encoding]::new($true))
    & $Compiler /V3 /WX $Script
    if ($LASTEXITCODE -ne 0) { throw "NSIS hook compilation failed: $LASTEXITCODE" }
    if (-not (Test-Path -LiteralPath (Join-Path $Work 'compile-only-never-run.exe'))) { throw 'NSIS produced no fixture' }
    Write-Output 'NSIS install/uninstall hooks compiled; fixture installer was not executed.'
} finally {
    Remove-Item -LiteralPath $Work -Recurse -Force
}
