param(
    [string]$Configuration = "release",
    [string]$CertificateThumbprint = $env:LOCASTRA_CERT_THUMBPRINT,
    [string]$UpdateBaseUrl = $env:LOCASTRA_UPDATE_BASE_URL
)

$ErrorActionPreference = "Stop"
$workspace = Split-Path -Parent $PSScriptRoot
$package = Get-Content -Raw -LiteralPath (Join-Path $workspace "package.json") | ConvertFrom-Json
$version = [string]$package.version
$cargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"

Push-Location $workspace
try {
    & pnpm test:release
    if ($LASTEXITCODE -ne 0) { throw "Frontend verification failed" }
    & pnpm tauri build --bundles nsis --config src-tauri/tauri.release.conf.json
    if ($LASTEXITCODE -ne 0) { throw "Tauri NSIS build failed" }
} finally {
    Pop-Location
}

$installer = Get-ChildItem -LiteralPath (Join-Path $workspace "src-tauri\target\release\bundle\nsis") -Filter "*.exe" |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
if (-not $installer) { throw "NSIS installer was not produced" }

if ($CertificateThumbprint) {
    $signtool = (Get-Command signtool.exe -ErrorAction Stop).Source
    & $signtool sign /sha1 $CertificateThumbprint /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 $installer.FullName
    if ($LASTEXITCODE -ne 0) { throw "Authenticode signing failed" }
    $signature = Get-AuthenticodeSignature -LiteralPath $installer.FullName
    if ($signature.Status -ne "Valid") { throw "Installer signature is not valid: $($signature.Status)" }
} else {
    Write-Warning "LOCASTRA_CERT_THUMBPRINT is not set. The installer remains unsigned and must not be published as a formal release."
}

$hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $installer.FullName).Hash.ToLowerInvariant()
$releaseDirectory = Join-Path $workspace "artifacts\Locastra-$version-windows-x64"
New-Item -ItemType Directory -Force -Path $releaseDirectory | Out-Null
$targetInstaller = Join-Path $releaseDirectory $installer.Name
Copy-Item -LiteralPath $installer.FullName -Destination $targetInstaller -Force
$hash | Set-Content -Encoding ascii -LiteralPath "$targetInstaller.sha256"

if ($UpdateBaseUrl) {
    $manifest = [ordered]@{
        version = $version
        notes = "Locastra $version stability release"
        publishedAt = [DateTimeOffset]::UtcNow.ToString("o")
        url = "$($UpdateBaseUrl.TrimEnd('/'))/$($installer.Name)"
        sha256 = $hash
    }
    $manifest | ConvertTo-Json | Set-Content -Encoding utf8 -LiteralPath (Join-Path $releaseDirectory "latest.json")
}

Write-Host "Release ready: $releaseDirectory"

