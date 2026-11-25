[CmdletBinding()]
param([switch]$Worker, [string]$InputRoot, [string]$OutputRoot)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
if (-not $Worker -or $identity.Name.Split('\')[-1] -cnotmatch '^AmeFixture[0-9a-f]{8}$' -or
    $InputRoot -cne 'C:\AmeFixtureInput' -or $OutputRoot -cne 'C:\AmeFixtureResults') {
    throw 'Installed update worker requires the disposable guest account'
}
$utf8 = [Text.UTF8Encoding]::new($false)
function Publish-WorkerJson([string]$Name, $Value) {
    $path = Join-Path $OutputRoot $Name
    [IO.File]::WriteAllText(($path + '.tmp'), ($Value | ConvertTo-Json -Depth 6), $utf8)
    Move-Item -LiteralPath ($path + '.tmp') -Destination $path -Force
}

$parent = $null
$helper = $null
$application = $null
try {
    if ([Security.Principal.WindowsPrincipal]::new($identity).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'Installed handoff worker must have an ordinary token'
    }
    $plan = Get-Content -LiteralPath (Join-Path $InputRoot 'plan.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    $local = [Environment]::GetFolderPath('LocalApplicationData')
    $evidence = Join-Path $local 'AmeInstalledFixture'
    [void][IO.Directory]::CreateDirectory($evidence)
    [IO.File]::WriteAllText((Join-Path $evidence 'fixture-version.txt'), '0.1.0-alpha.3', $utf8)
    $marker = Join-Path $local 'Cedarflake\Ame\fixture-user-decision.txt'
    [void][IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($marker))
    [IO.File]::WriteAllText($marker, 'preserved installed user decision', $utf8)
    $cache = Join-Path $local 'Cedarflake\Ame\cache\application-updates'
    $packageDirectory = Join-Path $cache $plan.packageSha256
    [void][IO.Directory]::CreateDirectory($packageDirectory)
    Copy-Item -LiteralPath (Join-Path $InputRoot 'target.msi') -Destination (Join-Path $packageDirectory 'target.msi')
    $installed = (Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Cedarflake\Ame\Installer').InstallFolder
    $applicationPath = Join-Path $installed 'cedarflake_ame.exe'
    Publish-WorkerJson 'progress.json' @{ phase = 'preparing-handoff' }
    $parent = Start-Process -FilePath $applicationPath -ArgumentList @('--begin-installed', 'commit',
        'target.msi', $plan.packageSha256, $plan.packageLength, $plan.targetVersion, '-', '0') `
        -WorkingDirectory $evidence -WindowStyle Hidden -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds(40)
    $helperIdPath = Join-Path $evidence 'helper-process.txt'
    while (-not (Test-Path -LiteralPath $helperIdPath)) {
        if ($parent.HasExited -or [DateTime]::UtcNow -ge $deadline) {
            $errorPath = Join-Path $evidence 'parent-error.txt'
            $detail = if (Test-Path -LiteralPath $errorPath) { [IO.File]::ReadAllText($errorPath) } else { 'No readiness evidence' }
            throw "Installed handoff parent failed: $detail"
        }
        Start-Sleep -Milliseconds 10
    }
    $helper = Get-Process -Id ([int][IO.File]::ReadAllText($helperIdPath))
    [void]$helper.Handle
    if (-not $parent.WaitForExit(10000) -or $parent.ExitCode -ne 0) { throw 'Installed fixture parent did not retire normally' }
    Publish-WorkerJson 'progress.json' @{ phase = 'waiting-for-installer'; helperId = $helper.Id }
    $deadline = [DateTime]::UtcNow.AddMinutes(10)
    while (-not $helper.WaitForExit(200)) {
        if (Test-Path -LiteralPath (Join-Path $OutputRoot 'retain-for-user')) { $deadline = [DateTime]::UtcNow.AddMinutes(10) }
        if ([DateTime]::UtcNow -ge $deadline) { throw 'Installed helper is still waiting for elevation or installation' }
    }
    if ($helper.ExitCode -ne 0) { throw "Installed helper returned $($helper.ExitCode)" }
    $receipt = Join-Path $cache 'result.json'
    $resultCode = $null
    if (Test-Path -LiteralPath $receipt) {
        $record = Get-Content -LiteralPath $receipt -Raw -Encoding UTF8 | ConvertFrom-Json
        $resultCode = $record.code
        if ($record.code -notin @(0, 3010)) { throw "Windows Installer returned $($record.code)" }
    }
    if ((Get-FileHash -LiteralPath $applicationPath -Algorithm SHA256).Hash -ine $plan.applicationSha256 -or
        (Get-Item -LiteralPath $applicationPath).VersionInfo.ProductVersion -cne $plan.applicationVersion) {
        throw 'The registered application was not replaced by the expected full product'
    }
    $deadline = [DateTime]::UtcNow.AddSeconds(60)
    do {
        $application = Get-Process -Name cedarflake_ame -ErrorAction SilentlyContinue |
            Where-Object { $_.Path -ieq $applicationPath -and $_.MainWindowHandle -ne 0 } | Select-Object -First 1
        if ($null -eq $application) { Start-Sleep -Milliseconds 100 }
    } while ($null -eq $application -and [DateTime]::UtcNow -lt $deadline)
    if ($null -eq $application) { throw 'The upgraded desktop application did not reopen' }
    $token = & (Join-Path $InputRoot 'parent.exe') --inspect-token $application.Id | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0 -or $token.elevated -ne $false) { throw 'The upgraded application has an elevated token' }
    if ([IO.File]::ReadAllText($marker) -cne 'preserved installed user decision' -or
        (Get-Service CedarflakeAmeJournalBroker).Status -ne 'Running' -or
        -not (Test-Path -LiteralPath (Join-Path $evidence 'startup-guard.txt'))) {
        throw 'Installed update changed the user marker, broker availability or startup guard'
    }
    Publish-WorkerJson 'progress.json' @{ phase = 'upgraded-awaiting-ui'; processId = $application.Id
        version = $plan.applicationVersion; helperExitCode = $helper.ExitCode; resultCode = $resultCode }
    $deadline = [DateTime]::UtcNow.AddMinutes(5)
    while (-not (Test-Path -LiteralPath (Join-Path $OutputRoot 'ui-verified'))) {
        if (Test-Path -LiteralPath (Join-Path $OutputRoot 'retain-for-user')) { $deadline = [DateTime]::UtcNow.AddMinutes(5) }
        if ([DateTime]::UtcNow -ge $deadline) { throw 'Actual upgraded UI inspection is still pending' }
        Start-Sleep -Milliseconds 200
    }
    if (-not $application.CloseMainWindow() -or -not $application.WaitForExit(30000)) {
        throw 'The upgraded app did not close normally'
    }
    Publish-WorkerJson 'worker-result.json' @{ format = 'ame-installed-update-acceptance-v1'; status = 'passed'
        elevated = $false; cases = @(@{ mode = 'upgrade'; version = $plan.applicationVersion
            helperExitCode = 0; resultCode = $resultCode; userDataPreserved = $true
            brokerRunning = $true; competingLaunchRejected = $true; normalClose = $true; uiVerified = $true }) }
} catch {
    Publish-WorkerJson 'worker-result.json' @{ format = 'ame-installed-update-acceptance-v1'; status = 'failed'; error = $_.Exception.Message }
    exit 1
} finally {
    foreach ($process in @($application, $helper, $parent)) { if ($null -ne $process) { $process.Dispose() } }
}
