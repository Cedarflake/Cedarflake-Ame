$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'release_portable_payload.ps1')
. (Join-Path $PSScriptRoot 'release_update_engine.ps1')

function Invoke-AmePortablePack {
    param([Parameter(Mandatory)][object]$Tool, [Parameter(Mandatory)][string]$ToolDirectory,
        [Parameter(Mandatory)][string]$EnginePath, [Parameter(Mandatory)][string]$PackDirectory,
        [Parameter(Mandatory)][string]$OutputDirectory, [Parameter(Mandatory)][string]$Scratch,
        [Parameter(Mandatory)][string]$Version, [ValidateSet('x64', 'arm64')][string]$Architecture)

    $staging = Join-Path $Scratch 'packaging-tool'
    $runtime = Join-Path $staging 'tools/net8.0/any'
    [void][IO.Directory]::CreateDirectory((Split-Path -Parent $runtime))
    Copy-Item -LiteralPath (Join-Path $ToolDirectory 'tools/net8.0/any') -Destination $runtime -Recurse
    Copy-Item -LiteralPath (Join-Path $ToolDirectory 'vendor') -Destination (Join-Path $staging 'vendor') -Recurse
    Copy-Item -LiteralPath $EnginePath -Destination (Join-Path $staging "vendor/update_$Architecture.exe") -Force
    $environment = @{}
    $names = @('VELOPACK_TEMP', 'DOTNET_CLI_HOME', 'DOTNET_CLI_TELEMETRY_OPTOUT',
        'DOTNET_SKIP_FIRST_TIME_EXPERIENCE', 'TEMP', 'TMP') + @(
        Get-ChildItem Env: | Where-Object Name -Like 'VPK_*' | Select-Object -ExpandProperty Name)
    foreach ($name in $names) { $environment[$name] = [Environment]::GetEnvironmentVariable($name, 'Process') }
    try {
        foreach ($name in $names | Where-Object { $_ -like 'VPK_*' }) { [Environment]::SetEnvironmentVariable($name, $null, 'Process') }
        $temporary = Join-Path $Scratch 'tool-temporary'
        [void][IO.Directory]::CreateDirectory($temporary)
        foreach ($name in @('VELOPACK_TEMP', 'DOTNET_CLI_HOME', 'TEMP', 'TMP')) {
            [Environment]::SetEnvironmentVariable($name, $temporary, 'Process')
        }
        $env:DOTNET_CLI_TELEMETRY_OPTOUT = '1'
        $env:DOTNET_SKIP_FIRST_TIME_EXPERIENCE = '1'
        $arguments = @((Join-Path $runtime 'vpk.dll'), 'pack', '--packId', 'CedarflakeAme',
            '--packVersion', $Version, '--packDir', $PackDirectory, '--outputDir', $OutputDirectory,
            '--mainExe', 'cedarflake_ame.exe', '--packTitle', 'Cedarflake Ame', '--packAuthors', 'Cedarflake',
            '--runtime', "win-$Architecture", '--channel', "win-$Architecture", '--delta', 'None',
            '--exclude', '(?!)', '--noDefaultExclude', '--noInst', '--skipVeloAppCheck',
            '--shortcuts', 'None', '--skip-updates', '--legacyConsole', '--yes')
        & $Tool.Dotnet @arguments | Out-Host
        if ($LASTEXITCODE -ne 0) { throw 'Velopack portable packaging failed' }
    } finally {
        foreach ($name in $names) { [Environment]::SetEnvironmentVariable($name, $environment[$name], 'Process') }
    }
}

function Assert-AmePortableGeneratedPackages {
    param([Parameter(Mandatory)][string]$ArchivePath, [Parameter(Mandatory)][string]$PackagePath,
        [Parameter(Mandatory)][object[]]$Files, [Parameter(Mandatory)][string]$Version,
        [ValidateSet('x64', 'arm64')][string]$Architecture, [Parameter(Mandatory)][string]$EngineSha256)

    $zip = [IO.Compression.ZipFile]::OpenRead($ArchivePath)
    $package = [IO.Compression.ZipFile]::OpenRead($PackagePath)
    try {
        $portable = Get-AmePortableArchiveEntries $zip
        $full = Get-AmePortableArchiveEntries $package
        Assert-AmePortableLayout -Entries $portable -Version $Version -Architecture $Architecture -Prefix ''
        Assert-AmePortablePayloadEntries -Entries $portable -Files $Files -Prefix 'current/' -GeneratedNames @('sq.version')
        Assert-AmePortablePayloadEntries -Entries $full -Files $Files -Prefix 'lib/app/' `
            -GeneratedNames @('sq.version', 'Squirrel.exe', 'Cedarflake Ame_ExecutionStub.exe')
        foreach ($name in @('CedarflakeAme.nuspec', 'lib/app/sq.version', 'lib/app/Squirrel.exe', 'lib/app/Cedarflake Ame_ExecutionStub.exe')) {
            if (-not $full.ContainsKey($name)) { throw "Full update package is missing $name" }
        }
        Assert-AmePortableVersionEntry -Entry $full['CedarflakeAme.nuspec'] -Version $Version -Architecture $Architecture
        Assert-AmePortableVersionEntry -Entry $full['lib/app/sq.version'] -Version $Version -Architecture $Architecture
        if ((Get-AmePortableEntryHash $portable['Update.exe']) -ine $EngineSha256 -or
            (Get-AmePortableEntryHash $full['lib/app/Squirrel.exe']) -ine $EngineSha256 -or
            (Get-AmePortableEntryHash $full['lib/app/Cedarflake Ame_ExecutionStub.exe']) -cne
                (Get-AmePortableEntryHash $portable['Cedarflake Ame.exe']) -or
            (Get-AmePortableEntryHash $full['lib/app/sq.version']) -cne
                (Get-AmePortableEntryHash $portable['current/sq.version'])) {
            throw 'Portable ZIP and full package disagree on the verified engine, launcher or version'
        }
    } finally { $package.Dispose(); $zip.Dispose() }
}
