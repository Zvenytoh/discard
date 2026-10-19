param([ValidateSet('check', 'build', 'fmt', 'clippy', 'run')][string]$Task = 'check')
$ErrorActionPreference = 'Stop'
# Install the reviewed CLI with: cargo install tauri-cli --version 2.12.3 --locked
function Copy-WebViewLoader([string]$Profile) {
    $hostInfo = & rustc -vV
    if ($LASTEXITCODE -ne 0) { throw 'Could not determine the Rust host.' }
    if ($hostInfo -match '^host: x86_64-pc-windows-gnu$') {
        $loader = Get-ChildItem "target/$Profile/build/webview2-com-sys-*/out/x64/WebView2Loader.dll" |
            Select-Object -First 1
        if (-not $loader) { throw 'WebView2Loader.dll is missing.' }
        Copy-Item -LiteralPath $loader.FullName -Destination "target/$Profile/WebView2Loader.dll" -Force
    }
}

Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    if ($Task -in @('build', 'run')) {
        $cli = & cargo tauri --version
        if ($LASTEXITCODE -ne 0 -or $cli.Trim() -ne 'tauri-cli 2.12.3') {
            throw 'Install the pinned CLI: cargo install tauri-cli --version 2.12.3 --locked'
        }
    }
    switch ($Task) {
        check {
            & cargo fmt --all -- --check
            if ($LASTEXITCODE -ne 0) { throw 'Formatting failed.' }
            & cargo clippy --locked --all-targets -- -D warnings
            if ($LASTEXITCODE -ne 0) { throw 'Clippy failed.' }
        }
        fmt { & cargo fmt --all -- --check }
        clippy { & cargo clippy --locked --all-targets -- -D warnings }
        build {
            & cargo tauri build --no-bundle -- --locked
            if ($LASTEXITCODE -ne 0) { throw 'Build failed.' }
            Copy-WebViewLoader release
        }
        run {
            # GNU needs its loader beside the executable before Tauri launches it.
            & cargo build --locked
            if ($LASTEXITCODE -ne 0) { throw 'Build failed.' }
            Copy-WebViewLoader debug
            & cargo tauri dev -- --locked
        }
    }
    if ($LASTEXITCODE -ne 0) { throw "Cargo failed ($LASTEXITCODE)." }
} finally {
    Pop-Location
}
