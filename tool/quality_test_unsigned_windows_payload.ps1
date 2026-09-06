[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot 'quality_common.ps1')
. (Join-Path $PSScriptRoot 'quality_unsigned_windows_payload.ps1')

function Assert-AmeUnsignedFixtureRejected {
    param([scriptblock]$Action, [string]$Expected)
    $failure = $null
    try { & $Action | Out-Null } catch { $failure = $_ }
    if ($null -eq $failure -or $failure.Exception.Message -notmatch $Expected) {
        throw "Unsigned payload guard did not reject the expected violation: $Expected"
    }
}

$scratch = Join-Path (Get-AmeRepositoryRoot) ('build\unsigned-payload-test-' + [Guid]::NewGuid().ToString('N'))
Assert-AmeUnsignedBuildPath $scratch
$bundle = Join-Path $scratch 'bundle'
$assetRoot = Join-Path $bundle 'data\flutter_assets'
$sourceRoot = Join-Path $scratch 'source with space'
$source = Join-Path $sourceRoot 'lib.rs'
$dependencies = Join-Path $scratch 'library.d'
$builtLibrary = Join-Path $scratch 'library.dll'
$builtBroker = Join-Path $scratch 'broker.exe'
$nativeNames = @(
    'cedarflake_ame.exe', 'cedarflake_ame_journal_broker.exe',
    'rust_lib_cedarflake_ame.dll', 'flutter_windows.dll'
)
$ownedFiles = @(
    $source, $dependencies, $builtLibrary, $builtBroker,
    (Join-Path $bundle 'data\app.so'), (Join-Path $bundle 'data\icudtl.dat'),
    (Join-Path $assetRoot 'AssetManifest.bin')
) + @($nativeNames | ForEach-Object { Join-Path $bundle $_ })
$ownedDirectories = @($assetRoot, (Join-Path $bundle 'data'), $bundle, $sourceRoot, $scratch)
$pe = [byte[]]::new(128)
$pe[0] = 0x4D; $pe[1] = 0x5A; $pe[0x3C] = 64
$pe[64] = 0x50; $pe[65] = 0x45
$pe[68] = 0x64; $pe[69] = 0x86
$pe[84] = 2; $pe[88] = 0x0B; $pe[89] = 2
$arguments = @{
    BundlePath = $bundle
    BuiltLibrary = $builtLibrary
    DependencyFile = $dependencies
    RequiredRustSource = $source
    BuiltBroker = $builtBroker
}
$library = Join-Path $bundle 'rust_lib_cedarflake_ame.dll'
$broker = Join-Path $bundle 'cedarflake_ame_journal_broker.exe'
$asset = Join-Path $assetRoot 'AssetManifest.bin'
$dependencyRule = $builtLibrary.Replace(' ', '\ ') + ': ' + $source.Replace(' ', '\ ') + "`n"
try {
    New-Item -ItemType Directory -Path $assetRoot, $sourceRoot | Out-Null
    foreach ($file in $ownedFiles) { [System.IO.File]::WriteAllBytes($file, $pe) }
    [System.IO.File]::WriteAllText($dependencies, $dependencyRule)
    [System.IO.File]::SetLastWriteTimeUtc($source, [DateTime]::UtcNow.AddMinutes(-1))
    $facts = Get-AmeUnsignedWindowsPayloadFacts @arguments
    if ($facts.machine -cne 'windows-x64' -or $facts.fileCount -ne 7 -or $facts.rustDependencyCount -ne 1) {
        throw "Valid unsigned Release fixture was not completely described"
    }
    if (@($facts.files | Where-Object { [System.IO.Path]::IsPathRooted($_.relativePath) }).Count -ne 0) {
        throw "Unsigned evidence must not contain machine-specific payload paths"
    }

    Remove-Item -LiteralPath $broker
    Assert-AmeUnsignedFixtureRejected { Get-AmeUnsignedWindowsPayloadFacts @arguments } 'missing required'
    [System.IO.File]::WriteAllBytes($broker, $pe)
    Remove-Item -LiteralPath $asset
    Assert-AmeUnsignedFixtureRejected { Get-AmeUnsignedWindowsPayloadFacts @arguments } 'Flutter assets'
    [System.IO.File]::WriteAllBytes($asset, $pe)

    $wrongMachine = [byte[]]$pe.Clone()
    $wrongMachine[68] = 0x64; $wrongMachine[69] = 0xAA
    [System.IO.File]::WriteAllBytes($library, $wrongMachine)
    Assert-AmeUnsignedFixtureRejected { Get-AmeUnsignedWindowsPayloadFacts @arguments } 'Windows x64 PE'
    $badOffset = [byte[]]$pe.Clone()
    $badOffset[0x3C] = 127
    [System.IO.File]::WriteAllBytes($library, $badOffset)
    Assert-AmeUnsignedFixtureRejected { Get-AmeUnsignedWindowsPayloadFacts @arguments } 'header offset'
    $mismatch = [byte[]]$pe.Clone()
    $mismatch[127] = 1
    [System.IO.File]::WriteAllBytes($library, $mismatch)
    Assert-AmeUnsignedFixtureRejected { Get-AmeUnsignedWindowsPayloadFacts @arguments } 'differs from its current build'
    [System.IO.File]::WriteAllBytes($library, $pe)
    [System.IO.File]::WriteAllBytes($broker, $mismatch)
    Assert-AmeUnsignedFixtureRejected { Get-AmeUnsignedWindowsPayloadFacts @arguments } 'differs from its current build'
    [System.IO.File]::WriteAllBytes($broker, $pe)

    [System.IO.File]::SetLastWriteTimeUtc($source, [DateTime]::UtcNow.AddMinutes(1))
    Assert-AmeUnsignedFixtureRejected { Get-AmeUnsignedWindowsPayloadFacts @arguments } 'older than'
    [System.IO.File]::SetLastWriteTimeUtc($source, [DateTime]::UtcNow.AddMinutes(-1))
    [System.IO.File]::WriteAllText($dependencies, 'not a dependency rule')
    Assert-AmeUnsignedFixtureRejected { Get-AmeUnsignedWindowsPayloadFacts @arguments } 'malformed'
    [System.IO.File]::WriteAllText($dependencies, $builtLibrary + ': ')
    Assert-AmeUnsignedFixtureRejected { Get-AmeUnsignedWindowsPayloadFacts @arguments } 'dependency count'
    [System.IO.File]::WriteAllText($dependencies, $dependencyRule)
    $arguments.RequiredRustSource = $builtBroker
    Assert-AmeUnsignedFixtureRejected { Get-AmeUnsignedWindowsPayloadFacts @arguments } 'application crate'
    $arguments.RequiredRustSource = $source

    $junction = Join-Path $bundle 'unexpected-link'
    New-Item -ItemType Junction -Path $junction -Target $sourceRoot | Out-Null
    try {
        Assert-AmeUnsignedFixtureRejected { Get-AmeUnsignedWindowsPayloadFacts @arguments } 'reparse point'
    } finally {
        [System.IO.Directory]::Delete($junction, $false)
    }
    Get-AmeUnsignedWindowsPayloadFacts @arguments | Out-Null
    Write-Host 'Unsigned Windows payload guardrails passed: valid bundle and 11 distinct rejection cases'
} finally {
    foreach ($file in $ownedFiles) {
        if (Test-Path -LiteralPath $file) {
            Assert-AmeUnsignedBuildPath $file
            Remove-Item -LiteralPath $file -Force
        }
    }
    foreach ($directory in $ownedDirectories) {
        if (Test-Path -LiteralPath $directory) {
            Assert-AmeUnsignedBuildPath $directory
            [System.IO.Directory]::Delete($directory, $false)
        }
    }
}
