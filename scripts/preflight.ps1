[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$probePath = $null

try {
    if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
        throw 'rustup is missing. Install Rust with rustup before building.'
    }
    $toolchainFile = Join-Path $repoRoot 'rust-toolchain.toml'
    $match = [regex]::Match((Get-Content -LiteralPath $toolchainFile -Raw),
        '(?m)^channel\s*=\s*"([^"]+)"')
    if (-not $match.Success) {
        throw 'Cannot read the pinned channel from rust-toolchain.toml.'
    }
    $pinnedVersion = $match.Groups[1].Value
    $toolchain = $pinnedVersion
    if ($env:RUSTUP_TOOLCHAIN) {
        $toolchain = $env:RUSTUP_TOOLCHAIN
        Write-Host "[INFO] RUSTUP_TOOLCHAIN override: $toolchain"
    }
    # rustup run does not install a missing toolchain unless --install is requested.
    $compiler = & rustup run $toolchain rustc --version
    if ($LASTEXITCODE -ne 0) {
        throw "Install the pinned compiler with: rustup toolchain install $pinnedVersion"
    }
    if ($compiler -notmatch ('^rustc ' + [regex]::Escape($pinnedVersion) + '\s')) {
        throw "Expected rustc $pinnedVersion, got: $compiler"
    }
    Write-Host "[OK] $compiler"
    $cargoVersion = & rustup run $toolchain cargo --version
    if ($LASTEXITCODE -ne 0) {
        throw 'Cargo is not available in the selected Rust toolchain.'
    }
    Write-Host "[OK] $cargoVersion"

    if ([Environment]::OSVersion.Platform -eq [PlatformID]::Win32NT) {
        $programFiles32 = [Environment]::GetEnvironmentVariable('ProgramFiles(x86)')
        $vswhere = Join-Path $programFiles32 'Microsoft Visual Studio/Installer/vswhere.exe'
        if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) {
            throw 'Visual Studio Build Tools locator is missing. Install the MSVC C++ workload.'
        }
        $installation = & $vswhere -latest -products '*' `
            -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
            -property installationPath
        if (-not $installation) {
            throw 'MSVC x64 build tools are missing. Install the C++ desktop workload.'
        }
        Write-Host "[OK] MSVC toolset: $installation"
        $sdkLib = Join-Path $programFiles32 'Windows Kits/10/Lib'
        $sdkVersions = @(Get-ChildItem -LiteralPath $sdkLib -Directory -ErrorAction SilentlyContinue |
            Where-Object { Test-Path -LiteralPath (Join-Path $_.FullName 'um/x64/kernel32.lib') })
        if ($sdkVersions.Count -eq 0) {
            throw 'The Windows SDK x64 libraries are missing. Install a Windows SDK.'
        }
        Write-Host "[OK] Windows SDK x64 libraries: $($sdkVersions[-1].Name)"
    }
    elseif (-not (Get-Command cc -ErrorAction SilentlyContinue)) {
        throw 'A native C linker/compiler is required (for example, cc from build-essential).'
    }

    $outputPath = Join-Path $repoRoot 'target'
    if ($env:CARGO_TARGET_DIR) {
        $outputPath = $env:CARGO_TARGET_DIR
        if (-not [System.IO.Path]::IsPathRooted($outputPath)) {
            $outputPath = Join-Path $repoRoot $outputPath
        }
    }
    [void][System.IO.Directory]::CreateDirectory($outputPath)
    $probePath = Join-Path $outputPath ('.preflight-' + [guid]::NewGuid().ToString('N'))
    [System.IO.File]::WriteAllText($probePath, 'preflight')
    Write-Host "[OK] Writable build output: $outputPath"
    Write-Host '[OK] No .NET SDK, game, or interop assemblies are needed for Data Viewer.'
    Write-Host '[INFO] Cargo downloads can require separate network/cache access in a sandbox.'
    Write-Host '[OK] Preflight completed. No build or data conversion was run.'
}
catch {
    Write-Host "[FAIL] $($_.Exception.Message)"
    exit 1
}
finally {
    if ($probePath -and [System.IO.File]::Exists($probePath)) {
        [System.IO.File]::Delete($probePath)
    }
}
