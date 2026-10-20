param([Parameter(Mandatory)][string]$Target)
$ErrorActionPreference = 'Stop'
$version = '2.12.3'
$extension = if ($Target -like '*windows*' -or $Target -like '*apple*') { 'zip' } else { 'tgz' }
$name = "cargo-tauri-$Target.$extension"
$headers = @{ Accept = 'application/vnd.github+json' }
if ($env:GH_TOKEN) { $headers.Authorization = "Bearer $env:GH_TOKEN" }
$release = Invoke-RestMethod -Headers $headers "https://api.github.com/repos/tauri-apps/tauri/releases/tags/tauri-cli-v$version"
$asset = @($release.assets | Where-Object name -CEQ $name)
if ($asset.Count -ne 1 -or $asset[0].digest -notmatch '^sha256:[a-f0-9]{64}$') {
    throw "Official Tauri CLI asset/digest unavailable: $name"
}
$directory = Join-Path $env:RUNNER_TEMP "discard-tauri-cli-$version"
New-Item -ItemType Directory -Force $directory | Out-Null
$archive = Join-Path $directory $name
Invoke-WebRequest $asset[0].browser_download_url -OutFile $archive
$digest = 'sha256:' + (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
if ($digest -cne $asset[0].digest) { throw 'Official Tauri CLI archive SHA256 mismatch.' }
if ($extension -eq 'zip') {
    Expand-Archive -LiteralPath $archive -DestinationPath $directory -Force
} else {
    & tar -xzf $archive -C $directory
    if ($LASTEXITCODE -ne 0) { throw 'Could not extract Tauri CLI.' }
}
$executable = if ($Target -like '*windows*') { 'cargo-tauri.exe' } else { 'cargo-tauri' }
$binary = @(Get-ChildItem -LiteralPath $directory -Filter $executable -Recurse -File)
if ($binary.Count -ne 1) { throw 'Expected one Tauri CLI executable.' }
$env:PATH = $binary[0].DirectoryName + [IO.Path]::PathSeparator + $env:PATH
$actual = & cargo tauri --version
if ($LASTEXITCODE -ne 0 -or $actual.Trim() -cne "tauri-cli $version") {
    throw 'Downloaded CLI version does not match the reviewed version.'
}
$binary[0].DirectoryName | Add-Content -LiteralPath $env:GITHUB_PATH
Write-Output "Installed verified $actual for $Target"
