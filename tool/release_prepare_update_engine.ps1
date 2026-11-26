[CmdletBinding()]
param([Parameter(Mandatory)][string]$OutputDirectory,
    [ValidateSet('x64', 'arm64')][string]$Architecture = 'x64')

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'quality_common.ps1')
. (Join-Path $PSScriptRoot 'acceptance_scratch_storage.ps1')
. (Join-Path $PSScriptRoot 'release_update_engine.ps1')
Add-Type -AssemblyName System.IO.Compression.FileSystem

$repository = Get-AmeRepositoryRoot
$output = [IO.Path]::GetFullPath($OutputDirectory).TrimEnd('\', '/')
if (-not $output.StartsWith((Join-Path $repository 'build') + '\', [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Update engine output must remain beneath repository build'
}
if ([Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture.ToString() -ine $Architecture) {
    throw "Build and run updater transaction tests on a native Windows $Architecture host"
}
Assert-AmeUnsignedBuildPath $output
$engine = Join-Path $output 'Update.exe'
$receiptPath = Join-Path $output 'engine.json'
if ((Test-Path -LiteralPath $engine) -or (Test-Path -LiteralPath $receiptPath)) {
    Assert-AmeUpdateEngine -Path $engine -ReceiptPath $receiptPath -Architecture $Architecture | Out-Null
    return @{ UpdateEnginePath = $engine; UpdateEngineReceiptPath = $receiptPath }
}
$cache = Join-Path $repository 'build/dependency/velopack-1.2.161'
Assert-AmeUnsignedBuildPath $cache
[void][IO.Directory]::CreateDirectory($cache)
$archive = Join-Path $cache ($script:AmeUpdateEngine.SourceCommit + '.zip')
Assert-AmeUnsignedBuildPath $archive
if (-not (Test-Path -LiteralPath $archive)) {
    $partial = $archive + '.partial'
    Assert-AmeUnsignedBuildPath $partial
    Invoke-WebRequest -Uri $script:AmeUpdateEngine.SourceUrl -OutFile $partial -Resume -TimeoutSec 600
    if ((Get-Item -LiteralPath $partial).Length -ne $script:AmeUpdateEngine.SourceArchiveLength -or
        (Get-FileHash -LiteralPath $partial -Algorithm SHA256).Hash -ine $script:AmeUpdateEngine.SourceArchiveSha256) {
        throw 'Velopack source archive differs from its immutable source pin'
    }
    Move-Item -LiteralPath $partial -Destination $archive
}
if ((Get-Item -LiteralPath $archive).Length -ne $script:AmeUpdateEngine.SourceArchiveLength -or
    (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ine $script:AmeUpdateEngine.SourceArchiveSha256) {
    throw 'Velopack source archive differs from its immutable source pin'
}
$patchHash = (Get-FileHash -LiteralPath $script:AmeUpdateEngine.Patch -Algorithm SHA256).Hash.ToLowerInvariant()
$cargo = Resolve-AmeExecutable -Name 'cargo' -FallbackPath (Join-Path $env:USERPROFILE '.cargo/bin/cargo.exe')
$git = Resolve-AmeExecutable -Name 'git' -FallbackPath (Join-Path $env:ProgramFiles 'Git/cmd/git.exe')
$rustConfiguration = [IO.File]::ReadAllText((Join-Path $repository 'rust-toolchain.toml'), [Text.Encoding]::UTF8)
$rustChannel = [regex]::Match($rustConfiguration, '(?m)^channel\s*=\s*"(\d+\.\d+\.\d+)"\s*$')
if (-not $rustChannel.Success) { throw 'The repository must declare an exact Rust release for the updater build' }
$cargoToolchain = '+' + $rustChannel.Groups[1].Value
$target = if ($Architecture -ceq 'arm64') { 'aarch64-pc-windows-msvc' } else { 'x86_64-pc-windows-msvc' }
$targetDirectory = Join-Path $cache "target-$Architecture"
Assert-AmeUnsignedBuildPath $targetDirectory
$lock = Enter-AmeRepositoryToolLock
$previousFlags = $env:RUSTFLAGS
$previousTarget = $env:CARGO_TARGET_DIR
$previousIncremental = $env:CARGO_INCREMENTAL
$previousCFlags = $env:CFLAGS
$previousTemp = $env:TEMP
$previousTmp = $env:TMP
try {
    $env:RUSTFLAGS = '-C target-feature=+crt-static'
    $env:CARGO_TARGET_DIR = $targetDirectory
    $env:CARGO_INCREMENTAL = '0'
    $env:CFLAGS = '/utf-8'
    Invoke-AmeAcceptanceScratch -Name "update-engine-$Architecture" -Action {
        param($scratch)
        $temporary = Join-Path $scratch 'temporary'
        [void][IO.Directory]::CreateDirectory($temporary)
        $env:TEMP = $temporary
        $env:TMP = $temporary
        $source = Join-Path $scratch 'source'
        Expand-AmeUpdateEngineSource -ArchivePath $archive -Directory $source
        $relative = [IO.Path]::GetRelativePath($repository, $source).Replace('\', '/')
        Invoke-AmeChecked $git @('-C', $repository, 'apply', "--directory=$relative", '--check', $script:AmeUpdateEngine.Patch) | Out-Host
        Invoke-AmeChecked $git @('-C', $repository, 'apply', "--directory=$relative", $script:AmeUpdateEngine.Patch) | Out-Host
        # Upstream release builds stamp the shared workspace version. Keep every locked
        # third-party dependency unchanged while assigning this reproducible patch identity.
        foreach ($name in @('Cargo.toml', 'Cargo.lock')) {
            $path = Join-Path $source $name
            $content = [IO.File]::ReadAllText($path, [Text.Encoding]::UTF8)
            $content = $content.Replace('version = "0.0.0-local"', ('version = "' + $script:AmeUpdateEngine.Version + '"'))
            [IO.File]::WriteAllText($path, $content, [Text.UTF8Encoding]::new($false))
        }
        Push-Location $source
        try {
            $testArguments = @($cargoToolchain, 'test', '--locked', '--release', '--jobs', '1', '--target', $target, '--color', 'never',
                '-p', 'velopack_bins', '--lib', '--features', 'windows', 'apply_windows_impl::transaction::tests', '--', '--test-threads=1')
            & $cargo @testArguments 2>&1 | Tee-Object -Variable testOutput | Out-Host
            if ($LASTEXITCODE -ne 0 -or
                @($testOutput | Where-Object { "$_" -match '^test result: ok\. 3 passed; 0 failed;' }).Count -ne 1) {
                throw 'All three updater directory-transaction regression tests must pass'
            }
            Invoke-AmeChecked $cargo @($cargoToolchain, 'build', '--locked', '--release', '--jobs', '1', '--target', $target,
                '-p', 'velopack_bins', '--bin', 'update', '--features', 'windows') | Out-Host
        } finally { Pop-Location }
        $built = Join-Path $targetDirectory "$target/release/update.exe"
        Assert-AmeWindowsPeImage $built $Architecture
        [void][IO.Directory]::CreateDirectory($output)
        Copy-Item -LiteralPath $built -Destination $engine
        $receipt = [ordered]@{
            format = 'cedarflake-ame-update-engine-v1'; version = $script:AmeUpdateEngine.Version
            architecture = $Architecture; sourceCommit = $script:AmeUpdateEngine.SourceCommit
            sourceArchiveSha256 = $script:AmeUpdateEngine.SourceArchiveSha256; patchSha256 = $patchHash
            engineSha256 = (Get-FileHash -LiteralPath $engine -Algorithm SHA256).Hash.ToLowerInvariant()
            staticCrt = $true; transactionTests = 'passed'
        }
        [IO.File]::WriteAllText($receiptPath, ($receipt | ConvertTo-Json), [Text.UTF8Encoding]::new($false))
    }
} finally {
    $env:RUSTFLAGS = $previousFlags
    $env:CARGO_TARGET_DIR = $previousTarget
    $env:CARGO_INCREMENTAL = $previousIncremental
    $env:CFLAGS = $previousCFlags
    $env:TEMP = $previousTemp
    $env:TMP = $previousTmp
    Exit-AmeRepositoryToolLock $lock
}
Assert-AmeUpdateEngine -Path $engine -ReceiptPath $receiptPath -Architecture $Architecture | Out-Null
return @{ UpdateEnginePath = $engine; UpdateEngineReceiptPath = $receiptPath }
