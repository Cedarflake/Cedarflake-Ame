[CmdletBinding()]
param(
    [string]$ResourceDirectory = (Join-Path (Split-Path -Parent $PSScriptRoot) "lib/app/presentation/l10n")
)

$ErrorActionPreference = "Stop"
$template = Get-Content -LiteralPath (Join-Path $resourceDirectory "app_zh.arb") -Encoding UTF8 -Raw |
    ConvertFrom-Json
$english = Get-Content -LiteralPath (Join-Path $resourceDirectory "app_en.arb") -Encoding UTF8 -Raw |
    ConvertFrom-Json
$keys = @($template.PSObject.Properties.Name | Where-Object { -not $_.StartsWith("@") })
$translated = @($english.PSObject.Properties.Name | Where-Object { -not $_.StartsWith("@") })
$differences = @(Compare-Object $keys $translated -CaseSensitive)
if ($differences.Count -gt 0) {
    throw "Chinese and English message keys differ: $($differences.InputObject -join ', ')"
}
foreach ($key in $keys) {
    $message = $english.PSObject.Properties[$key].Value
    $source = $template.PSObject.Properties[$key].Value
    if ([string]::IsNullOrWhiteSpace($source)) {
        throw "Simplified Chinese message '$key' is empty."
    }
    if ([string]::IsNullOrWhiteSpace($message)) {
        throw "English message '$key' is empty."
    }
    $metadata = $template.PSObject.Properties["@$key"]
    if ($null -eq $metadata) { continue }
    $placeholders = $metadata.Value.PSObject.Properties["placeholders"]
    if ($null -eq $placeholders) { continue }
    foreach ($argument in $placeholders.Value.PSObject.Properties.Name) {
        $pattern = "\{\s*" + [regex]::Escape($argument) + "\s*[,}]"
        if ($message -cnotmatch $pattern) {
            throw "English message '$key' is missing placeholder '$argument'."
        }
        if ($source -cnotmatch $pattern) {
            throw "Simplified Chinese message '$key' is missing placeholder '$argument'."
        }
    }
}
Write-Host "Verified $($keys.Count) Simplified Chinese and English messages."
