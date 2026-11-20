$script:AmeInstallerFamilies = Get-Content -LiteralPath (
    Join-Path $PSScriptRoot '../windows/installer/product_families.json'
) -Encoding utf8 -Raw | ConvertFrom-Json

function Get-AmeInstallerGuid {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [ValidateSet('x64', 'arm64')][string]$Architecture = 'x64'
    )

    $namespaceBytes = ([guid]$script:AmeInstallerFamilies.$Architecture).ToByteArray()
    [Array]::Reverse($namespaceBytes, 0, 4)
    [Array]::Reverse($namespaceBytes, 4, 2)
    [Array]::Reverse($namespaceBytes, 6, 2)
    $nameBytes = [Text.Encoding]::UTF8.GetBytes($Name)
    $sha1 = [Security.Cryptography.SHA1]::Create()
    try { $digest = $sha1.ComputeHash($namespaceBytes + $nameBytes) } finally { $sha1.Dispose() }
    $digest[6] = ($digest[6] -band 0x0F) -bor 0x50
    $digest[8] = ($digest[8] -band 0x3F) -bor 0x80
    $hex = -join @($digest[0..15] | ForEach-Object { $_.ToString("X2") })
    return "{0}-{1}-{2}-{3}-{4}" -f $hex.Substring(0, 8), $hex.Substring(8, 4),
        $hex.Substring(12, 4), $hex.Substring(16, 4), $hex.Substring(20, 12)
}
