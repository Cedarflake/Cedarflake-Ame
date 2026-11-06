$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Get-AmeInstallerVersion {
    param(
        [Parameter(Mandatory = $true)][string]$Tag,
        [Parameter(Mandatory = $true)][string]$ApplicationVersion
    )

    $identifier = '(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)'
    $pattern = '^v(?<major>0|[1-9][0-9]*)\.(?<minor>0|[1-9][0-9]*)\.' +
        '(?:0|[1-9][0-9]*)(?:-' + $identifier + '(?:\.' + $identifier + ')*)?$'
    $tagMatch = [regex]::Match($Tag, $pattern)
    $applicationMatch = [regex]::Match($ApplicationVersion,
        '^(?<release>[^+]+)\+(?<build>[1-9][0-9]*)$')
    if (-not $tagMatch.Success -or -not $applicationMatch.Success -or
        ('v' + $applicationMatch.Groups['release'].Value) -cne $Tag) {
        throw 'MSI requires a matching semantic release tag and a positive Flutter build number'
    }

    $values = @($tagMatch.Groups['major'].Value, $tagMatch.Groups['minor'].Value,
        $applicationMatch.Groups['build'].Value)
    $bounds = @(255, 255, 65535)
    for ($index = 0; $index -lt $values.Count; $index++) {
        $number = 0L
        if (-not [long]::TryParse($values[$index], [ref]$number) -or $number -gt $bounds[$index]) {
            throw 'Application version exceeds Windows Installer version limits'
        }
    }
    return $values -join '.'
}
