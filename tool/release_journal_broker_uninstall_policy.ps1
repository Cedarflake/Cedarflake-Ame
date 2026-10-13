function Test-AmeBrokerUninstallApplicationRemoval {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State
    )

    if ([int]$State.Schema -eq 2) {
        return [string]$State.Operation -ceq "uninstall"
    }
    if ([int]$State.Schema -ne 3) {
        throw "The broker transaction schema is unsupported"
    }
    $intent = $State.PSObject.Properties["RemoveApplicationBundle"]
    if ($null -eq $intent -or $intent.Value -isnot [bool]) {
        throw "The broker transaction lacks an explicit application removal policy"
    }
    if ($intent.Value -and [string]$State.Operation -cne "uninstall") {
        throw "Only an uninstall transaction may request application removal"
    }
    return [bool]$intent.Value
}
