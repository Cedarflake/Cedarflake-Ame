function Get-AmeWorkflowJobBlock {
    param(
        [Parameter(Mandatory = $true)]
        [string]$WorkflowText,
        [Parameter(Mandatory = $true)]
        [string]$JobName
    )

    $match = [regex]::Match(
        $WorkflowText,
        ('(?ms)^  {0}:\r?\n.*?(?=^  [a-zA-Z0-9_]+:\r?\n|\z)' -f
            [regex]::Escape($JobName))
    )
    if (-not $match.Success) {
        throw "Hosted release workflow is missing the $JobName job"
    }
    return $match.Value
}
