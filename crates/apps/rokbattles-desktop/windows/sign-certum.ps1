param(
  [Parameter(Mandatory)]
  [string]$Path
)

$ErrorActionPreference = 'Stop'

if ([string]::IsNullOrWhiteSpace($env:CERTUM_EMAIL) -or
    [string]::IsNullOrWhiteSpace($env:CERTUM_OTP)) {
  throw 'Set the CERTUM_EMAIL and CERTUM_OTP repository secrets in GitHub Actions.'
}

$thumbprint = $env:CERTUM_CERT_THUMBPRINT -replace '[\s\u200E]', ''
if ($thumbprint -notmatch '^[a-fA-F0-9]{40}$') {
  throw 'Set the CERTUM_CERT_THUMBPRINT repository secret to the 40-digit SHA-1 certificate thumbprint.'
}

& ssign --timestamp-url 'http://time.certum.pl/' -- $Path
if ($LASTEXITCODE -ne 0) {
  throw "Certum signing failed for '$Path' (ssign exit code $LASTEXITCODE)."
}

$signature = Get-AuthenticodeSignature -LiteralPath $Path
if ($signature.Status -ne 'Valid') {
  throw "Authenticode verification failed for '$Path' ($($signature.StatusMessage))."
}
if ($signature.SignerCertificate.Thumbprint -ne $thumbprint) {
  throw "Unexpected signing certificate for '$Path' ($($signature.SignerCertificate.Thumbprint))."
}
if ($null -eq $signature.TimeStamperCertificate) {
  throw "Missing Authenticode timestamp for '$Path'."
}

Write-Output "Verified timestamped Authenticode signature for '$Path' ($thumbprint)."
