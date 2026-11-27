$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'quality_common.ps1')
. (Join-Path $PSScriptRoot 'acceptance_scratch_storage.ps1')
. (Join-Path $PSScriptRoot 'release_msix_manifest.ps1')
. (Join-Path $PSScriptRoot 'release_msix_payload.ps1')

function Assert-MsixRejected {
    param([Parameter(Mandatory)][scriptblock]$Action)
    $rejected = $false
    try { & $Action } catch { $rejected = $true }
    if (-not $rejected) { throw 'MSIX admitted an invalid package fixture' }
}

foreach ($case in @(
    @('v0.1.0-alpha.3', '0.1.0-alpha.3+3', '1.1.3.0'),
    @('v0.1.0-beta.1', '0.1.0-beta.1+4', '1.1.4.0'),
    @('v1.2.3', '1.2.3+9', '2.2.9.0')
)) {
    if ((Get-AmeMsixVersion -Tag $case[0] -ApplicationVersion $case[1]) -cne $case[2]) {
        throw 'MSIX version does not retain monotonic build identity'
    }
}
Assert-MsixRejected { Get-AmeMsixVersion -Tag v0.1.0 -ApplicationVersion '0.1.0+0' }
foreach ($helper in @('cedarflake_ame_update.exe', 'cedarflake_ame_journal_broker.exe')) {
    if (Test-AmeMsixPayloadFile $helper) { throw 'MSIX included an external deployment helper' }
}
if (-not (Test-AmeMsixPayloadFile 'cedarflake_ame_classification_worker.exe')) {
    throw 'MSIX excluded the required analysis worker'
}
foreach ($managed in @('AppxManifest.xml', 'AppxSignature.p7x', 'resources.pri', 'Update.exe', 'Assets/logo.png')) {
    Assert-MsixRejected { Test-AmeMsixPayloadFile $managed }
}

function New-MsixArchiveFixture {
    param([string]$Path, [string[]]$Names, [string]$Content = 'fixture')
    $archive = [IO.Compression.ZipFile]::Open($Path, [IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($entryName in $Names) {
            $writer = [IO.StreamWriter]::new($archive.CreateEntry($entryName).Open())
            try { $writer.Write($Content) } finally { $writer.Dispose() }
        }
    } finally { $archive.Dispose() }
}

Invoke-AmeAcceptanceScratch -Name 'msix-payload-contract' -Action {
    param($scratch)
    $manifestPath = Join-Path $scratch 'AppxManifest.xml'
    New-AmeMsixManifest -Path $manifestPath -Version '1.1.4.0'
    $manifest = [xml](Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8)
    if ($manifest.Package.Identity.Name -cne 'Cedarflake.Cedarflake-Ame' -or
        $manifest.Package.Identity.Publisher -cne 'CN=4756B427-4333-4200-A66E-9407CC4BB239') {
        throw 'MSIX identity differs from the reserved Store family'
    }
    if (@($manifest.Package.Capabilities.Capability).Count -ne 1 -or
        $manifest.Package.Capabilities.Capability.Name -cne 'runFullTrust') {
        throw 'MSIX declared capabilities outside its deployment contract'
    }
    foreach ($version in @('0.1.4.0', '1.1.4.1', '1.1.65536.0')) {
        Assert-MsixRejected { New-AmeMsixManifest -Path $manifestPath -Version $version }
    }
    $fixtureFile = Join-Path $scratch 'source.bin'
    [IO.File]::WriteAllText($fixtureFile, 'fixture', [Text.UTF8Encoding]::new($false))
    $expected = @([pscustomobject]@{
        relativePath = 'cedarflake_ame.exe'; length = 7L
        sha256 = (Get-FileHash -LiteralPath $fixtureFile -Algorithm SHA256).Hash
    })
    $metadata = @('AppxBlockMap.xml', '[Content_Types].xml')
    $valid = Join-Path $scratch 'valid.msix'
    New-MsixArchiveFixture $valid (@('cedarflake_ame.exe') + $metadata)
    Assert-AmeMsixArchive -Path $valid -Files $expected
    $index = 0
    foreach ($entries in @(
        @('other.exe', 'AppxBlockMap.xml', '[Content_Types].xml'),
        @('cedarflake_ame.exe', 'CEDARFLAKE_AME.exe', 'AppxBlockMap.xml'),
        @('cedarflake_ame.exe', '../outside', 'AppxBlockMap.xml'),
        @('cedarflake_ame.exe', 'C:/outside', 'AppxBlockMap.xml'),
        @('cedarflake_ame.exe', 'data:stream', 'AppxBlockMap.xml'),
        @('cedarflake_ame.exe', 'AppxSignature.p7x', 'AppxBlockMap.xml')
    )) {
        $invalid = Join-Path $scratch "invalid-$index.msix"
        $index++
        New-MsixArchiveFixture $invalid $entries
        Assert-MsixRejected { Assert-AmeMsixArchive -Path $invalid -Files $expected }
    }
    $changed = Join-Path $scratch 'changed.msix'
    New-MsixArchiveFixture $changed (@('cedarflake_ame.exe') + $metadata) 'changed'
    Assert-MsixRejected { Assert-AmeMsixArchive -Path $changed -Files $expected }
}
Write-Host 'MSIX version, capability and payload admission contracts passed'
