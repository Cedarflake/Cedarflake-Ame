function Invoke-AmeUpdateSandbox {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string]$FixtureDirectory,
        [Parameter(Mandatory)][string]$OutputDirectory,
        [Parameter(Mandatory)][string]$FixtureFormat,
        [Parameter(Mandatory)][string]$ResultFormat,
        [Parameter(Mandatory)][string[]]$FileNames,
        [Parameter(Mandatory)][int]$CaseCount,
        [switch]$AllowUserHandoff,
        [ValidateRange(1, 15)][int]$TimeoutMinutes = 6
    )
    $ErrorActionPreference = 'Stop'
    . (Join-Path $PSScriptRoot 'quality_common.ps1')
    . (Join-Path $PSScriptRoot 'acceptance_scratch_storage.ps1')
    . (Join-Path $PSScriptRoot 'acceptance_update_sandbox_owner.ps1')
    $fixture = Assert-AmeAcceptanceScratchPath $FixtureDirectory
    $output = Assert-AmeAcceptanceScratchPath $OutputDirectory
    $repositoryBuild = Join-Path (Get-AmeRepositoryRoot) 'build/test'
    foreach ($path in @($fixture, $output)) {
        if (-not $path.StartsWith($repositoryBuild + '\', [StringComparison]::OrdinalIgnoreCase)) {
            throw 'Sandbox fixture inputs and reports must remain beneath build/test'
        }
    }
    if (-not $output.StartsWith((Join-Path $repositoryBuild 'derived') + '\', [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Sandbox reports must remain beneath build/test/derived'
    }
    if (Test-Path -LiteralPath $output) { throw 'Sandbox report directory must be unused' }
    $plan = Get-Content -LiteralPath (Join-Path $fixture 'plan.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($plan.format -cne $FixtureFormat -or (Compare-Object @($plan.files.name) $FileNames -CaseSensitive)) { throw 'Unexpected fixture manifest' }
    foreach ($file in $plan.files) {
        if ($file.name -cnotin $FileNames) { throw 'Unexpected fixture file' }
        $path = Assert-AmeAcceptanceScratchPath (Join-Path $fixture $file.name)
        if ((Get-Item -LiteralPath $path).Length -ne $file.length -or
            (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ine $file.sha256) {
            throw 'Fixture files differ from their prepared manifest'
        }
    }
    $wsb = (Get-Command wsb.exe -CommandType Application -ErrorAction Stop).Source
    $id = [guid]::NewGuid().ToString()
    $lock = Enter-AmeRepositoryToolLock
    $started = $false
    $connection = $null
    $operationFailure = $null
    $cleanupFailure = $null
    $retained = $false
    try {
        [void][IO.Directory]::CreateDirectory($output)
        $owner = [pscustomobject]@{ format = 'ame-owned-sandbox-v1'; id = $id; state = 'running'
            connectionId = 0; connectionStarted = 0L }
        Save-AmeUpdateSandboxOwner -Directory $output -Owner $owner
        $inputXml = [Security.SecurityElement]::Escape($fixture)
        $outputXml = [Security.SecurityElement]::Escape($output)
        $configuration = @"
    <Configuration><Networking>Disable</Networking><VGpu>Disable</VGpu><MemoryInMB>3072</MemoryInMB>
    <ClipboardRedirection>Disable</ClipboardRedirection><AudioInput>Disable</AudioInput><VideoInput>Disable</VideoInput>
    <MappedFolders><MappedFolder><HostFolder>$inputXml</HostFolder><SandboxFolder>C:\AmeInput</SandboxFolder><ReadOnly>true</ReadOnly></MappedFolder>
    <MappedFolder><HostFolder>$outputXml</HostFolder><SandboxFolder>C:\AmeResults</SandboxFolder><ReadOnly>false</ReadOnly></MappedFolder></MappedFolders>
    <LogonCommand><Command>powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:\AmeInput\guest.ps1</Command></LogonCommand></Configuration>
"@
        $existing = & $wsb list --raw
        if ($LASTEXITCODE -ne 0 -or ($existing -join "`n").Contains($id)) { throw 'Cannot establish fresh Sandbox ownership' }
        $started = $true
        Invoke-AmeChecked $wsb @('start', '--id', $id, '--config', $configuration, '--raw') | Out-Host
        $connection = Start-Process -FilePath $wsb -ArgumentList @('connect', '--id', $id) -WindowStyle Hidden -PassThru
        $owner.connectionId = $connection.Id
        $owner.connectionStarted = $connection.StartTime.ToUniversalTime().Ticks
        Save-AmeUpdateSandboxOwner -Directory $output -Owner $owner
        $report = Join-Path $output 'result.json'
        $deadline = [DateTime]::UtcNow.AddMinutes($TimeoutMinutes)
        while (-not (Test-Path -LiteralPath $report)) {
            if ($AllowUserHandoff -and (Test-Path -LiteralPath (Join-Path $output 'retain-for-user'))) {
                $owner.state = 'awaiting-user'
                Save-AmeUpdateSandboxOwner -Directory $output -Owner $owner
                $retained = $true
                break
            }
            if ([DateTime]::UtcNow -ge $deadline) { throw "Owned Sandbox fixture timed out: $id" }
            Start-Sleep -Milliseconds 250
        }
        if (-not $retained) {
            $result = Get-Content -LiteralPath $report -Raw -Encoding UTF8 | ConvertFrom-Json
            if ($result.status -cne 'passed') { throw "Application update fixture failed: $($result.error)" }
            if ($result.format -cne $ResultFormat -or $result.elevated -ne $false -or @($result.cases).Count -ne $CaseCount) { throw 'Incomplete application update evidence' }
            Write-Host 'Actual helper cases passed in Windows Sandbox'
        }
    } catch {
        $operationFailure = $_
    } finally {
        try {
            if ($started -and -not $retained) { Stop-AmeUpdateSandbox $output }
        } catch {
            $cleanupFailure = $_
        } finally {
            if ($null -ne $connection) { $connection.Dispose() }
            Exit-AmeRepositoryToolLock $lock
        }
    }
    if ($null -ne $operationFailure -and $null -ne $cleanupFailure) {
        throw [AggregateException]::new('Sandbox acceptance and retirement failed',
            [Exception[]]@($operationFailure.Exception, $cleanupFailure.Exception))
    }
    if ($null -ne $operationFailure) { throw $operationFailure }
    if ($null -ne $cleanupFailure) { throw $cleanupFailure }
    Write-Output (Join-Path $output $(if ($retained) { 'owner.json' } else { 'result.json' }))

}
