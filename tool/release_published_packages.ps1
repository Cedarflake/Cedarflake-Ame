$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-AmePublishedPackages {
    param(
        [Parameter(Mandatory)][object]$Release,
        [Parameter(Mandatory)][string]$Tag,
        [Parameter(Mandatory)][string]$Directory,
        [string]$ExpectedArchiveSha256 = '',
        [string]$ExpectedInstallerSha256 = ''
    )

    if ($Release.tag_name -cne $Tag -or $Release.draft -ne $false) {
        throw 'Published release tag or visibility differs'
    }
    $expected = [ordered]@{
        "Cedarflake-Ame-$Tag-windows-x64-portable.zip" = $ExpectedArchiveSha256
        "Cedarflake-Ame-$Tag-windows-x64.msi" = $ExpectedInstallerSha256
    }
    if ([string]::IsNullOrEmpty($ExpectedArchiveSha256) -ne [string]::IsNullOrEmpty($ExpectedInstallerSha256)) {
        throw 'Expected publication hashes must cover both packages'
    }
    $items = @(Get-ChildItem -LiteralPath $Directory -Force)
    if ($items.Count -ne 2) { throw 'Published download requires exactly one ZIP and one MSI' }
    foreach ($item in $items) {
        if ($item.PSIsContainer -or -not $expected.Contains($item.Name) -or
            ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw 'Published download contains an unexpected entry'
        }
    }
    foreach ($name in $expected.Keys) {
        $assets = @($Release.assets | Where-Object { $_.name -ceq $name })
        if ($assets.Count -ne 1 -or $assets[0].state -cne 'uploaded' -or
            $assets[0].digest -cnotmatch '^sha256:[0-9a-f]{64}$' -or
            $assets[0].size -le 0 -or $assets[0].size -gt 2GB) {
            throw 'Published release is missing a verified ZIP or MSI'
        }
        $path = Join-Path $Directory $name
        $hash = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
        if ((Get-Item -LiteralPath $path).Length -ne $assets[0].size -or
            "sha256:$hash" -cne $assets[0].digest -or
            ($expected[$name] -ne '' -and $hash -cne $expected[$name])) {
            throw 'Published package differs from its verified SHA-256 identity'
        }
    }
}
