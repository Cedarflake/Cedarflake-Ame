. (Join-Path $PSScriptRoot 'acceptance_scratch_storage.ps1')

function Read-AmeUpdateSandboxOwner {
    param([Parameter(Mandatory)][string]$OutputDirectory)

    $directory = Assert-AmeAcceptanceScratchPath $OutputDirectory
    if (-not $directory.StartsWith((Join-Path (Get-AmeRepositoryRoot) 'build/test/derived') + '\', [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Sandbox ownership must remain beneath build/test/derived'
    }
    $owner = Get-Content -LiteralPath (Join-Path $directory 'owner.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    $id = [guid]::Empty
    if ($owner.format -cne 'ame-owned-sandbox-v1' -or -not [guid]::TryParse($owner.id, [ref]$id) -or
        $owner.state -cnotin @('running', 'awaiting-user', 'retired')) { throw 'Unexpected Sandbox ownership receipt' }
    return [pscustomobject]@{ Directory = $directory; Owner = $owner }
}

function Save-AmeUpdateSandboxOwner {
    param([Parameter(Mandatory)][string]$Directory, [Parameter(Mandatory)][object]$Owner)

    $path = Join-Path $Directory 'owner.json'
    [IO.File]::WriteAllText(($path + '.tmp'), ($Owner | ConvertTo-Json), [Text.UTF8Encoding]::new($false))
    Move-Item -LiteralPath ($path + '.tmp') -Destination $path -Force
}

function Stop-AmeUpdateSandbox {
    param([Parameter(Mandatory)][string]$OutputDirectory)

    $state = Read-AmeUpdateSandboxOwner $OutputDirectory
    $owner = $state.Owner
    $wsb = (Get-Command wsb.exe -CommandType Application -ErrorAction Stop).Source
    $remaining = & $wsb list --raw
    if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect owned Sandbox retirement' }
    if (($remaining -join "`n").Contains($owner.id)) {
        Invoke-AmeChecked $wsb @('stop', '--id', $owner.id, '--raw') | Out-Host
    }
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    do {
        $remaining = & $wsb list --raw
        if ($LASTEXITCODE -ne 0) { throw 'Cannot verify owned Sandbox retirement' }
        $active = ($remaining -join "`n").Contains($owner.id)
        if ($active) { Start-Sleep -Milliseconds 250 }
    } while ($active -and [DateTime]::UtcNow -lt $deadline)
    if ($active) { throw "Owned Sandbox remains active: $($owner.id)" }
    if ($owner.connectionId -gt 0) {
        $connection = Get-Process -Id $owner.connectionId -ErrorAction SilentlyContinue
        if ($null -ne $connection) {
            try {
                if ($connection.StartTime.ToUniversalTime().Ticks -eq $owner.connectionStarted -and
                    -not $connection.WaitForExit(10000)) { throw 'Owned Sandbox connection has not retired' }
            } finally { $connection.Dispose() }
        }
    }
    $credentials = Join-Path $state.Directory 'elevation-credential.json'
    if (Test-Path -LiteralPath $credentials) { Remove-Item -LiteralPath $credentials }
    $owner.state = 'retired'
    Save-AmeUpdateSandboxOwner -Directory $state.Directory -Owner $owner
}

function Resume-AmeInstalledUpdateSandbox {
    param([Parameter(Mandatory)][string]$OutputDirectory, [switch]$UiVerified, [switch]$Retire)

    $state = Read-AmeUpdateSandboxOwner $OutputDirectory
    if ($state.Owner.state -cne 'awaiting-user') { throw 'Only a retained user handoff can resume' }
    $lock = Enter-AmeRepositoryToolLock
    try {
        if ($Retire) { Stop-AmeUpdateSandbox $OutputDirectory; return }
        if ($UiVerified) {
            [IO.File]::WriteAllText((Join-Path $state.Directory 'ui-verified'), 'actual upgraded UI observed', [Text.UTF8Encoding]::new($false))
        }
        $report = Join-Path $state.Directory 'result.json'
        $existing = & wsb.exe list --raw
        if ($LASTEXITCODE -ne 0 -or (-not ($existing -join "`n").Contains($state.Owner.id) -and
            -not (Test-Path -LiteralPath $report))) { throw 'The retained Sandbox is unavailable and has no completed report' }
        $deadline = [DateTime]::UtcNow.AddMinutes(15)
        while (-not (Test-Path -LiteralPath $report)) {
            if ([DateTime]::UtcNow -ge $deadline) {
                throw "Retained Sandbox still awaits user action; ownership remains in $($state.Directory)"
            }
            Start-Sleep -Milliseconds 250
        }
        $result = Get-Content -LiteralPath $report -Raw -Encoding UTF8 | ConvertFrom-Json
        Stop-AmeUpdateSandbox $OutputDirectory
        if ($result.status -cne 'passed') { throw "Installed update fixture failed: $($result.error)" }
        if ($result.format -cne 'ame-installed-update-acceptance-v1' -or $result.elevated -ne $false -or
            @($result.cases).Count -ne 1 -or $result.uninstalled -ne $true) { throw 'Incomplete installed update evidence' }
        Write-Output $report
    } finally { Exit-AmeRepositoryToolLock $lock }
}
