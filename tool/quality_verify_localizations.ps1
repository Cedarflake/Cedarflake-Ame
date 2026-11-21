[CmdletBinding()]
param(
    [string]$ResourceDirectory = (Join-Path (Split-Path -Parent $PSScriptRoot) "lib/app/presentation/l10n")
)

$ErrorActionPreference = "Stop"

function Read-AmeCatalog([string]$Locale) {
    $file = Join-Path $ResourceDirectory "app_$Locale.arb"
    $catalog = Get-Content -LiteralPath $file -Encoding UTF8 -Raw | ConvertFrom-Json
    if ($catalog.'@@locale' -cne $Locale) {
        throw "Expected locale '$Locale' in $file."
    }
    return $catalog
}

$template = Read-AmeCatalog "zh"
$keys = @($template.PSObject.Properties.Name | Where-Object { -not $_.StartsWith("@") })
foreach ($locale in @("zh", "zh_Hans", "en", "ja")) {
    $catalog = if ($locale -eq "zh") { $template } else { Read-AmeCatalog $locale }
    foreach ($name in $catalog.PSObject.Properties.Name) {
        if (-not $name.StartsWith("@") -and $keys -cnotcontains $name) {
            throw "Unexpected $locale message '$name'."
        }
    }
    foreach ($key in $keys) {
        $translation = $catalog.PSObject.Properties[$key]
        # Flutter inherits absent script-specific messages from the complete Chinese base catalog.
        if ($locale -eq "zh_Hans" -and $null -eq $translation) { continue }
        if ($null -eq $translation -or $translation.Value -isnot [string] -or
            [string]::IsNullOrWhiteSpace($translation.Value)) {
            throw "Missing or empty $locale message '$key'."
        }
        $metadata = $template.PSObject.Properties["@$key"]
        if ($null -eq $metadata) { continue }
        $placeholders = $metadata.Value.PSObject.Properties["placeholders"]
        if ($null -eq $placeholders) { continue }
        foreach ($argument in $placeholders.Value.PSObject.Properties.Name) {
            $pattern = "\{\s*" + [regex]::Escape($argument) + "\s*[,}]"
            if ($translation.Value -cnotmatch $pattern) {
                throw "$locale message '$key' is missing placeholder '$argument'."
            }
        }
    }
}
Write-Host "Verified $($keys.Count) messages for zh-Hans, en and ja."
