$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'quality_unsigned_windows_payload.ps1')

$script:AmeUpdateEngine = [ordered]@{
    Version = '1.2.161+ame.rollback.1'
    SourceCommit = '92d6a1c91716729d449034df5c50307dcce39493'
    SourceArchiveSha256 = '3e35756fddcc9f966792411268e3c085a44b482a02942005e4a3f6827c4a50a7'
    SourceArchiveLength = 228408436L
    SourceUrl = 'https://codeload.github.com/velopack/velopack/zip/92d6a1c91716729d449034df5c50307dcce39493'
    Patch = (Join-Path $PSScriptRoot 'patches/velopack-1.2.161-directory-rollback.patch')
}

function Assert-AmeUpdateEngine {
    param([Parameter(Mandatory)][string]$Path, [Parameter(Mandatory)][string]$ReceiptPath,
        [ValidateSet('x64', 'arm64')][string]$Architecture)
    Assert-AmeUnsignedBuildPath $Path
    Assert-AmeUnsignedBuildPath $ReceiptPath
    $receipt = Get-Content -LiteralPath $ReceiptPath -Raw -Encoding utf8 | ConvertFrom-Json
    if ($receipt.format -cne 'cedarflake-ame-update-engine-v1' -or
        $receipt.version -cne $script:AmeUpdateEngine.Version -or $receipt.architecture -cne $Architecture -or
        $receipt.sourceCommit -cne $script:AmeUpdateEngine.SourceCommit -or
        $receipt.sourceArchiveSha256 -cne $script:AmeUpdateEngine.SourceArchiveSha256 -or
        $receipt.patchSha256 -ine (Get-FileHash -LiteralPath $script:AmeUpdateEngine.Patch -Algorithm SHA256).Hash -or
        $receipt.engineSha256 -ine (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash -or
        $receipt.staticCrt -ne $true -or $receipt.transactionTests -cne 'passed') {
        throw 'Portable update engine does not match its pinned source, patch and verified build'
    }
    Assert-AmeWindowsPeImage -Path $Path -Architecture $Architecture
    return $receipt
}

function Expand-AmeUpdateEngineSource {
    param([Parameter(Mandatory)][string]$ArchivePath, [Parameter(Mandatory)][string]$Directory)
    $archive = [IO.Compression.ZipFile]::OpenRead($ArchivePath)
    try {
        $prefix = 'velopack-' + $script:AmeUpdateEngine.SourceCommit + '/'
        $count = 0
        $total = 0L
        foreach ($entry in $archive.Entries) {
            if (-not $entry.FullName.StartsWith($prefix, [StringComparison]::Ordinal)) { throw 'Unexpected dependency source root' }
            $relative = $entry.FullName.Substring($prefix.Length)
            if ($relative -notmatch '^(src/|locales/|\.cargo/|Cargo\.(toml|lock)$|LICENSE$)' -or $relative.EndsWith('/')) { continue }
            $count++
            $total += $entry.Length
            if ($count -gt 2048 -or $total -gt 16MB -or $relative -match '(^|/)\.\.(/|$)|:|\\' -or
                (($entry.ExternalAttributes -shr 16) -band 0xF000) -eq 0xA000) {
                throw 'Dependency source selection exceeds its path or size bound'
            }
            $target = Join-Path $Directory $relative
            [void][IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($target))
            [IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $target)
        }
    } finally { $archive.Dispose() }
}
