# sign.ps1 — Create a self-signed code signing certificate and sign the exe
#
# Usage:
#   .\scripts\sign.ps1                    # Sign target\release\windowed-claude.exe
#   .\scripts\sign.ps1 -ExePath .\my.exe  # Sign a specific exe
#   .\scripts\sign.ps1 -CreateCert        # Force recreate the certificate
#
# The certificate is stored in CurrentUser\My and persists across builds.
# First run creates it; subsequent runs reuse it.
#
# NOTE: Self-signed certs won't eliminate SmartScreen entirely, but they:
#   - Give the exe a consistent publisher identity
#   - Make the "Unknown publisher" dialog less scary
#   - Build reputation faster with SmartScreen's heuristics
#   - Prove the exe hasn't been tampered with after signing

param(
    [string]$ExePath = "target\release\windowed-claude.exe",
    [switch]$CreateCert
)

$CertSubject = "CN=WindowedClaude, O=WindowedClaude, L=Open Source"
$CertFriendlyName = "WindowedClaude Code Signing"

# Check if certificate already exists
$cert = Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert |
    Where-Object { $_.Subject -eq $CertSubject } |
    Sort-Object NotAfter -Descending |
    Select-Object -First 1

if ($CreateCert -or -not $cert) {
    Write-Host "Creating self-signed code signing certificate..." -ForegroundColor Yellow

    $cert = New-SelfSignedCertificate `
        -Type CodeSigningCert `
        -Subject $CertSubject `
        -FriendlyName $CertFriendlyName `
        -CertStoreLocation Cert:\CurrentUser\My `
        -NotAfter (Get-Date).AddYears(5) `
        -KeyUsage DigitalSignature `
        -KeyAlgorithm RSA `
        -KeyLength 2048 `
        -HashAlgorithm SHA256

    Write-Host "Certificate created: $($cert.Thumbprint)" -ForegroundColor Green

    # Export the public cert so users can optionally trust it
    $exportPath = Join-Path $PSScriptRoot "..\assets\WindowedClaude.cer"
    Export-Certificate -Cert $cert -FilePath $exportPath | Out-Null
    Write-Host "Public certificate exported to: $exportPath"
    Write-Host ""
    Write-Host "To trust this cert on another machine (optional):" -ForegroundColor Cyan
    Write-Host "  Import-Certificate -FilePath WindowedClaude.cer -CertStoreLocation Cert:\CurrentUser\Root"
} else {
    Write-Host "Using existing certificate: $($cert.Thumbprint)" -ForegroundColor Green
    Write-Host "  Expires: $($cert.NotAfter)"
}

# Check exe exists
if (-not (Test-Path $ExePath)) {
    Write-Host ""
    Write-Host "ERROR: Exe not found at $ExePath" -ForegroundColor Red
    Write-Host "Build first with: cargo build --release"
    exit 1
}

# Sign the exe
Write-Host ""
Write-Host "Signing $ExePath ..." -ForegroundColor Yellow

try {
    Set-AuthenticodeSignature `
        -FilePath $ExePath `
        -Certificate $cert `
        -TimestampServer "http://timestamp.digicert.com" `
        -HashAlgorithm SHA256

    # Verify
    $sig = Get-AuthenticodeSignature -FilePath $ExePath
    if ($sig.Status -eq "Valid" -or $sig.Status -eq "UnknownError") {
        # UnknownError is expected for self-signed (not in trusted root)
        Write-Host "Signed successfully!" -ForegroundColor Green
        Write-Host "  Status: $($sig.Status)"
        Write-Host "  Signer: $($sig.SignerCertificate.Subject)"
    } else {
        Write-Host "Signing may have issues: $($sig.Status)" -ForegroundColor Yellow
    }
} catch {
    Write-Host "Signing failed: $_" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "Done! The exe now has:" -ForegroundColor Green
Write-Host "  - Authenticode signature (self-signed)"
Write-Host "  - SHA256 hash integrity"
Write-Host "  - Timestamp from DigiCert"
Write-Host ""
Write-Host "SmartScreen will still warn on first download, but:" -ForegroundColor Cyan
Write-Host "  - The publisher shows 'WindowedClaude' instead of 'Unknown'"
Write-Host "  - Reputation builds faster with consistent signing identity"
Write-Host "  - The exe is tamper-evident"
