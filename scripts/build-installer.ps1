param([string]$ReleaseTag = '')
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$manifest = Get-Content (Join-Path $root 'Cargo.toml') -Raw
$package = [regex]::Match($manifest, '(?ms)^\[package\]\s*(.*?)(?=^\[|\z)').Groups[1].Value
$version = [regex]::Match($package, '(?m)^version\s*=\s*"(\d+\.\d+\.\d+)"\s*$').Groups[1].Value
if (-not $version) { throw 'Cargo.toml must contain a numeric major.minor.patch package version.' }
if ($ReleaseTag -and $ReleaseTag -cne "v$version") {
    throw "Release tag '$ReleaseTag' must match Cargo.toml version 'v$version'."
}
if (-not (Test-Path (Join-Path $root 'target/release/discard.exe'))) {
    throw 'Build first: cargo tauri build --no-bundle -- --locked'
}

$compiler = Get-ChildItem -Path @(
    "${env:ProgramFiles(x86)}/Inno Setup */ISCC.exe",
    "$env:ProgramFiles/Inno Setup */ISCC.exe",
    "$env:LOCALAPPDATA/Programs/Inno Setup */ISCC.exe"
) -ErrorAction SilentlyContinue | Sort-Object FullName -Descending | Select-Object -First 1
if (-not $compiler) { throw 'Install Inno Setup 6.3 or later before packaging.' }

$dependencies = Join-Path $root 'target/dependencies'
$distribution = Join-Path $root 'target/distribution'
New-Item -ItemType Directory -Force $dependencies, $distribution | Out-Null
$bootstrapper = Join-Path $dependencies 'MicrosoftEdgeWebview2Setup.exe'
Invoke-WebRequest 'https://go.microsoft.com/fwlink/p/?LinkId=2124703' -OutFile $bootstrapper
$signature = Get-AuthenticodeSignature -LiteralPath $bootstrapper
if ($signature.Status -ne 'Valid' -or
    $signature.SignerCertificate.Subject -notmatch '(^|,\s*)O=Microsoft Corporation(,|$)') {
    throw 'The Microsoft WebView2 bootstrapper Authenticode signature is invalid.'
}

& $compiler.FullName "/DAppVersion=$version" (Join-Path $root 'installer/Discard.iss')
if ($LASTEXITCODE -ne 0) { throw 'Inno Setup compilation failed.' }
Copy-Item -LiteralPath (Join-Path $root 'target/release/discard.exe') -Destination (Join-Path $distribution 'Discard.exe') -Force
$brandKit = Join-Path $distribution 'Discard-Brand-Kit.zip'
Compress-Archive -Path (Join-Path $root 'assets/branding/blue-d/*') -DestinationPath $brandKit -Force
$assets = @('Discard-Setup.exe', 'Discard.exe', 'Discard-Brand-Kit.zip')
$loader = Join-Path $root 'target/release/WebView2Loader.dll'
if (Test-Path -LiteralPath $loader) {
    Copy-Item -LiteralPath $loader -Destination $distribution -Force
    $assets += 'WebView2Loader.dll'
}
$checksums = foreach ($asset in $assets) {
    $hash = (Get-FileHash -LiteralPath (Join-Path $distribution $asset) -Algorithm SHA256).Hash.ToLowerInvariant()
    "$hash  $asset"
}
$checksums | Set-Content -LiteralPath (Join-Path $distribution 'SHA256SUMS.txt') -Encoding ascii
Write-Output "Packaged Discard $version in $distribution"
