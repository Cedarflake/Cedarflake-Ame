[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")
. (Join-Path $PSScriptRoot "quality_unsigned_windows_payload.ps1")
. (Join-Path $PSScriptRoot "integration_windows_runner_lifecycle.ps1")

if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT -or
    [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -ne 'X64' -or
    -not [Environment]::Is64BitProcess) {
    throw "Unsigned Windows verification requires a native Windows x64 process"
}
$repositoryRoot = Get-AmeRepositoryRoot
$toolchain = Get-AmeToolchain
$releaseRoot = Join-Path $repositoryRoot 'build\windows\x64\runner\Release'
$evidenceRoot = Join-Path $repositoryRoot 'build\quality-unsigned-windows'
$pluginRoot = Join-Path $repositoryRoot (
    'build\windows\x64\plugins\rust_lib_cedarflake_ame\cargokit_build\x86_64-pc-windows-msvc\release'
)
$brokerTarget = Join-Path $evidenceRoot 'broker-target'
$builtBroker = Join-Path $brokerTarget 'x86_64-pc-windows-msvc\release\cedarflake_ame_journal_broker.exe'
$evidencePath = Join-Path $evidenceRoot 'evidence.json'
$previousLibrary = $env:CEDARFLAKE_AME_TEST_LIBRARY_PATH
$aliasRoot = $null
$aliasPath = $null
$evidence = $null
$toolLock = Enter-AmeRepositoryToolLock
Push-Location $repositoryRoot
try {
    foreach ($path in @($releaseRoot, $evidencePath, $pluginRoot, $builtBroker)) {
        Assert-AmeUnsignedBuildPath $path
    }
    New-Item -ItemType Directory -Path $evidenceRoot -Force | Out-Null
    $sourceCommit = (& $toolchain.Git rev-parse --verify HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or $sourceCommit -cnotmatch '^[0-9a-f]{40}$') {
        throw "Unsigned build evidence requires an exact source commit"
    }
    $changes = @(& $toolchain.Git status --porcelain --untracked-files=normal)
    if ($LASTEXITCODE -ne 0) {
        throw "Unsigned build could not inspect source worktree state"
    }
    $evidence = [ordered]@{
        format = 'cedarflake-ame-unsigned-quality-v1'
        sourceCommit = $sourceCommit
        dirtyWorktree = $changes.Count -ne 0
        startedUtc = [DateTime]::UtcNow.ToString('o')
        status = 'running'
    }
    [System.IO.File]::WriteAllText($evidencePath, ($evidence | ConvertTo-Json -Depth 8))
    Invoke-AmeChecked $toolchain.Flutter @('pub', 'get', '--enforce-lockfile')
    Invoke-AmeChecked $toolchain.Flutter @('build', 'windows', '--release', '--no-pub')
    $evidence['nativeRunnerLifecycle'] = Invoke-AmeWindowsRunnerLifecycle -RepositoryRoot $repositoryRoot
    Invoke-AmeChecked $toolchain.Cargo @(
        'build', '--locked', '--manifest-path', 'rust\Cargo.toml', '--release',
        '--target', 'x86_64-pc-windows-msvc', '--target-dir', $brokerTarget,
        '--bin', 'cedarflake_ame_journal_broker'
    )
    $packagedBroker = Join-Path $releaseRoot 'cedarflake_ame_journal_broker.exe'
    Assert-AmeUnsignedBuildPath $packagedBroker
    Assert-AmeUnsignedBuildPath $builtBroker
    Copy-Item -LiteralPath $builtBroker -Destination $packagedBroker -Force
    $payloadArguments = @{
        BundlePath = $releaseRoot
        BuiltLibrary = Join-Path $pluginRoot 'rust_lib_cedarflake_ame.dll'
        DependencyFile = Join-Path $pluginRoot 'rust_lib_cedarflake_ame.d'
        RequiredRustSource = Join-Path $repositoryRoot 'rust\src\lib.rs'
        BuiltBroker = $builtBroker
    }
    $payload = Get-AmeUnsignedWindowsPayloadFacts @payloadArguments
    $aliasRoot = Join-Path $evidenceRoot ('bridge-' + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $aliasRoot | Out-Null
    $aliasPath = Join-Path $aliasRoot 'rust_lib_cedarflake_ame_unsigned_smoke.dll'
    Copy-Item -LiteralPath (Join-Path $releaseRoot 'rust_lib_cedarflake_ame.dll') -Destination $aliasPath
    $env:CEDARFLAKE_AME_TEST_LIBRARY_PATH = $aliasPath
    Invoke-AmeChecked $toolchain.Flutter @(
        'test', 'integration_test\unsigned_release_bridge_smoke_test.dart',
        '-d', 'windows', '--no-pub'
    )
    $afterSmoke = Get-AmeUnsignedWindowsPayloadFacts @payloadArguments
    if (($payload | ConvertTo-Json -Depth 8 -Compress) -cne
        ($afterSmoke | ConvertTo-Json -Depth 8 -Compress)) {
        throw "Unsigned Release payload changed while validating its bridge"
    }
    Assert-AmeUnsignedBuildPath $aliasPath
    Remove-Item -LiteralPath $aliasPath -Force
    [System.IO.Directory]::Delete($aliasRoot, $false)
    $aliasPath = $null
    $aliasRoot = $null
    $evidence['payload'] = $payload
    $evidence['bridgeSmoke'] = 'passed_without_catalog_access'
    $evidence['status'] = 'passed'
    $evidence['completedUtc'] = [DateTime]::UtcNow.ToString('o')
    Assert-AmeUnsignedBuildPath $evidencePath
    [System.IO.File]::WriteAllText($evidencePath, ($evidence | ConvertTo-Json -Depth 8))
    Write-Host "Unsigned Windows x64 Release build and catalog-free bridge smoke passed"
} catch {
    if ($null -ne $evidence) {
        $evidence['status'] = 'failed'
        $evidence['completedUtc'] = [DateTime]::UtcNow.ToString('o')
        try {
            Assert-AmeUnsignedBuildPath $evidencePath
            [System.IO.File]::WriteAllText($evidencePath, ($evidence | ConvertTo-Json -Depth 8))
        } catch {
            Write-Warning 'Unsigned Windows failure evidence could not be persisted'
        }
    }
    throw
} finally {
    try {
        if ($null -ne $aliasPath -and (Test-Path -LiteralPath $aliasPath)) {
            Assert-AmeUnsignedBuildPath $aliasPath
            Remove-Item -LiteralPath $aliasPath -Force
        }
        if ($null -ne $aliasRoot -and (Test-Path -LiteralPath $aliasRoot)) {
            Assert-AmeUnsignedBuildPath $aliasRoot
            [System.IO.Directory]::Delete($aliasRoot, $false)
        }
    } catch {
        Write-Warning 'Unsigned bridge scratch was retained after cleanup failed'
    } finally {
        if ($null -eq $previousLibrary) {
            [Environment]::SetEnvironmentVariable(
                'CEDARFLAKE_AME_TEST_LIBRARY_PATH', [NullString]::Value
            )
        } else {
            [Environment]::SetEnvironmentVariable(
                'CEDARFLAKE_AME_TEST_LIBRARY_PATH', $previousLibrary
            )
        }
        Pop-Location
        Exit-AmeRepositoryToolLock $toolLock
    }
}
