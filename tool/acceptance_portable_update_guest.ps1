[CmdletBinding()]
param(
    [switch]$Worker,
    [string]$InputRoot = 'C:\AmeInput',
    [string]$OutputRoot = 'C:\AmeResults'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$workerResultPath = Join-Path $outputRoot 'worker-result.json'
$resultPath = if ($Worker) { $workerResultPath } else { Join-Path $outputRoot 'result.json' }
$utf8 = [Text.UTF8Encoding]::new($false)
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$currentAccount = $identity.Name.Split('\')[-1]
if (-not $Worker -and ($currentAccount -cne 'WDAGUtilityAccount' -or
    $inputRoot -cne 'C:\AmeInput' -or $outputRoot -cne 'C:\AmeResults')) {
    throw 'This controller requires its disposable Sandbox account and mapped fixture directories'
}
if ($Worker -and ($currentAccount -cnotmatch '^AmeFixture[0-9a-f]{8}$' -or
    $inputRoot -cne 'C:\AmeFixtureInput' -or $outputRoot -cne 'C:\AmeFixtureResults')) {
    throw 'This worker requires its disposable standard account and guest-local fixture directories'
}

function Publish-Result($Value) {
    $temporary = Join-Path $outputRoot 'result.tmp'
    [IO.File]::WriteAllText($temporary, ($Value | ConvertTo-Json -Depth 8), $utf8)
    Move-Item -LiteralPath $temporary -Destination $resultPath
}

try {
    if (-not $Worker) {
        . (Join-Path $inputRoot 'user.ps1')
        $guest = New-AmeGuestUser -InputRoot $inputRoot -OutputRoot $outputRoot
        try {
            Start-AmeGuestWorker -Guest $guest
            if (-not $guest.Process.WaitForExit(270000)) { throw 'Standard-user Sandbox worker did not retire' }
            $result = Read-AmeGuestWorkerResult -Guest $guest
        } finally { Remove-AmeGuestUser -Guest $guest }
        Publish-Result $result
        exit 0
    }
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    if ($principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'The Sandbox fixture still has an elevated token; the shipping helper must run unelevated'
    }
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $plan = Get-Content -LiteralPath (Join-Path $inputRoot 'plan.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    foreach ($file in $plan.files) {
        $path = Join-Path $inputRoot $file.name
        if ((Get-Item -LiteralPath $path).Length -ne $file.length -or
            (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ine $file.sha256) {
            throw 'Sandbox fixture input differs from its host manifest'
        }
    }
    $localAppData = [Environment]::GetFolderPath('LocalApplicationData')
    $work = Join-Path $localAppData 'AmePortableFixture'
    if (Test-Path -LiteralPath $work) { throw 'Sandbox work directory must be fresh' }
    [void][IO.Directory]::CreateDirectory($work)
    $profileMarker = Join-Path $localAppData 'Cedarflake\Ame\fixture-user-decision.txt'
    [void][IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($profileMarker))
    [IO.File]::WriteAllText($profileMarker, 'preserved user decision', $utf8)
    $cache = Join-Path $localAppData 'Cedarflake\Ame\cache\application-updates'
    $cases = @()
    foreach ($mode in @('commit', 'no-commit', 'dead-helper')) {
        $root = Join-Path $work $mode
        [IO.Compression.ZipFile]::ExtractToDirectory((Join-Path $inputRoot 'A.zip'), $root)
        [void][IO.Directory]::CreateDirectory((Join-Path $root 'packages'))
        [IO.File]::WriteAllText((Join-Path $root 'user-marker.txt'), 'preserved portable data', $utf8)
        $package = Join-Path $root 'packages\CedarflakeAme-1.2.4-full.nupkg'
        Copy-Item -LiteralPath (Join-Path $inputRoot 'B.nupkg') -Destination $package
        $archive = [IO.Compression.ZipFile]::OpenRead($package)
        try {
            $entry = $archive.GetEntry('lib/app/Squirrel.exe')
            if ($null -eq $entry -or $entry.Length -gt 64MB) { throw 'Target updater is missing or too large' }
            [IO.Compression.ZipFileExtensions]::ExtractToFile($entry, (Join-Path $root 'target-engine.exe'))
        } finally { $archive.Dispose() }
        $engine = Join-Path $root 'target-engine.exe'
        if ((Get-FileHash -LiteralPath $engine -Algorithm SHA256).Hash -ine $plan.engineSha256) {
            throw 'Target package did not carry the pinned patched updater'
        }
        $receipt = Join-Path $cache 'result.json'
        if (Test-Path -LiteralPath $receipt) { throw 'Previous case did not consume its result' }
        $arguments = @('--begin', $mode, 'CedarflakeAme-1.2.4-full.nupkg', $plan.packageSha256,
            $plan.packageLength, '1.2.4', $plan.engineSha256, (Get-Item -LiteralPath $engine).Length)
        $parent = Start-Process -FilePath (Join-Path $root 'current\cedarflake_ame.exe') `
            -ArgumentList $arguments -WorkingDirectory $root -WindowStyle Hidden -PassThru
        try {
            if (-not $parent.WaitForExit(45000)) { throw "Fixture parent did not retire: $mode" }
            if ($parent.ExitCode -ne 0) {
                $errorPath = Join-Path $root 'parent-error.txt'
                $detail = if (Test-Path -LiteralPath $errorPath) { [IO.File]::ReadAllText($errorPath) } else { 'No parent error detail' }
                throw "Fixture parent failed: $mode, exit $($parent.ExitCode): $detail"
            }
        } finally { $parent.Dispose() }
        $restarted = Join-Path $root 'restarted.txt'
        $deadline = [DateTime]::UtcNow.AddSeconds(90)
        while (-not (Test-Path -LiteralPath $restarted)) {
            if ([DateTime]::UtcNow -ge $deadline) { throw "Application did not reopen: $mode" }
            Start-Sleep -Milliseconds 100
        }
        $expected = if ($mode -ceq 'commit') { '1.2.4' } else { '1.2.3' }
        $record = Get-Content -LiteralPath $receipt -Raw -Encoding UTF8 | ConvertFrom-Json
        if ([IO.File]::ReadAllText($restarted) -cne $expected -or
            [IO.File]::ReadAllText((Join-Path $root 'current\fixture-version.txt')) -cne $expected -or
            [IO.File]::ReadAllText((Join-Path $root 'user-marker.txt')) -cne 'preserved portable data' -or
            [IO.File]::ReadAllText($profileMarker) -cne 'preserved user decision' -or
            -not (Test-Path -LiteralPath (Join-Path $root 'parent-exit.txt')) -or
            -not (Test-Path -LiteralPath (Join-Path $root 'startup-guard.txt')) -or
            $record.version -cne '1.2.4' -or $record.code -ne $(if ($mode -ceq 'commit') { 0 } else { 1603 })) {
            throw "Portable replacement or recovery evidence differs: $mode"
        }
        $engineHash = (Get-FileHash -LiteralPath (Join-Path $root 'Update.exe') -Algorithm SHA256).Hash.ToLowerInvariant()
        $expectedEngine = if ($mode -ceq 'commit') { $plan.engineSha256 } else { $plan.oldEngineSha256 }
        if ($engineHash -cne $expectedEngine) { throw "Root updater adoption differs: $mode" }
        $cases += @{ mode = $mode; version = $expected; code = $record.code; userDataPreserved = $true
            parentExitObserved = $true; competingLaunchRejected = $true }
        Remove-Item -LiteralPath $receipt
    }
    Publish-Result @{ format = 'ame-portable-update-acceptance-v1'; status = 'passed'; elevated = $false; cases = $cases }
} catch {
    if (-not (Test-Path -LiteralPath $resultPath)) {
        Publish-Result @{ format = 'ame-portable-update-acceptance-v1'; status = 'failed'; error = $_.Exception.Message }
    }
    exit 1
}
