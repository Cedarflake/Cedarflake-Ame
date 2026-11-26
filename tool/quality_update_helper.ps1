$ErrorActionPreference = 'Stop'

function Assert-AmeStandaloneUpdateHelper {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$CMakeCache
    )

    $cache = [IO.File]::ReadAllText($CMakeCache, [Text.Encoding]::UTF8)
    $match = [regex]::Match($cache, '(?m)^CMAKE_LINKER:FILEPATH=(.+)\r?$')
    if (-not $match.Success) { throw 'The Windows build did not record its native linker' }
    $linker = $match.Groups[1].Value.Trim()
    if (-not (Test-Path -LiteralPath $linker -PathType Leaf)) { throw 'The native linker is unavailable' }
    $output = & $linker /dump /dependents $Path
    if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect the shipped update helper dependencies' }
    $imports = @($output | ForEach-Object {
        if ($_ -match '^\s+([A-Za-z0-9_.-]+\.dll)\s*$') { $Matches[1].ToLowerInvariant() }
    } | Sort-Object -Unique)
    $systemLibraries = @('advapi32.dll', 'shell32.dll', 'ole32.dll', 'bcrypt.dll',
        'msi.dll', 'kernel32.dll', 'user32.dll')
    if ($imports -cnotcontains 'kernel32.dll' -or
        @($imports | Where-Object { $_ -cnotin $systemLibraries }).Count -ne 0) {
        throw "The update helper must run without adjacent runtime libraries: $($imports -join ', ')"
    }
    return $imports
}
