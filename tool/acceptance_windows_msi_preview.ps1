[CmdletBinding()]
param([Parameter(Mandatory)][string]$PackageDirectory)

$ErrorActionPreference = 'Stop'
if ($env:RUNNER_ENVIRONMENT -ne 'github-hosted' -or
    (Get-CimInstance Win32_OperatingSystem).ProductType -ne 1) {
    throw 'MSI lifecycle verification requires a disposable hosted Windows client'
}
$output = (Resolve-Path -LiteralPath $PackageDirectory).Path
$receipt = Get-Content -LiteralPath (Join-Path $output 'package.json') -Raw -Encoding utf8 | ConvertFrom-Json
$arch = [Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString().ToLowerInvariant()
$msi = Join-Path $output "Cedarflake-Ame-$($receipt.releaseTag)-windows-$arch.msi"
if ($receipt.payload.machine -cne "windows-$arch" -or $receipt.databasePolicy -cne 'passed' -or
    $receipt.sourceCommit -cne $env:GITHUB_SHA -or $receipt.dirtyWorktree -ne $false -or
    (Get-FileHash -LiteralPath $msi).Hash -cne $receipt.packageSha256) {
    throw 'MSI differs from the verified native preview'
}
$root = Join-Path ([Environment]::GetFolderPath('ProgramFiles')) 'Cedarflake Ame'
$app = Join-Path $root 'Application'
$broker = Join-Path $root 'Journal Broker/cedarflake_ame_journal_broker.exe'
$serviceName = 'CedarflakeAmeJournalBroker'
if ((Test-Path -LiteralPath $root) -or (Get-Service -Name $serviceName -ErrorAction SilentlyContinue)) {
    throw 'Fresh install requires an unused product directory and service name'
}

function Invoke-PreviewMsi([string]$Operation, [string]$Option) {
    $log = Join-Path $output "$Operation.log"
    $process = Start-Process msiexec.exe -ArgumentList @($Option, "`"$msi`"", '/qn', '/norestart', '/L*v', "`"$log`"") -PassThru
    try {
        if (-not $process.WaitForExit(300000)) { throw "$Operation exceeded five minutes" }
        if ($process.ExitCode -ne 0) {
            Select-String -LiteralPath $log -Pattern 'Return value 3|installation cannot continue' -Context 12,0 | Out-Host
            throw "$Operation failed with Windows Installer exit code $($process.ExitCode)"
        }
    } finally { $process.Dispose() }
}

function Assert-InstalledPreview {
    $service = Get-CimInstance Win32_Service -Filter "Name='$serviceName'"
    if ($service.StartName -ne 'LocalSystem' -or $service.StartMode -ne 'Manual' -or
        $service.State -ne 'Running' -or $service.PathName.Trim('"') -ine $broker) {
        throw 'Installed broker did not start with its expected identity and protected path'
    }
    foreach ($file in $receipt.payload.files) {
        $path = Join-Path $app $file.relativePath
        if ($file.relativePath -ceq 'cedarflake_ame_journal_broker.exe') { $path = $broker }
        if ((Get-FileHash -LiteralPath $path).Hash -ine $file.sha256) {
            throw "Installed file differs from the verified payload: $($file.relativePath)"
        }
    }
}

Invoke-PreviewMsi 'install' '/i'
Assert-InstalledPreview
$repairFile = Join-Path $app 'data/icudtl.dat'
Remove-Item -LiteralPath $repairFile
Invoke-PreviewMsi 'repair' '/fa'
Assert-InstalledPreview
Invoke-PreviewMsi 'uninstall' '/x'
if ((Get-Service -Name $serviceName -ErrorAction SilentlyContinue) -or
    (Test-Path -LiteralPath $broker) -or (Test-Path -LiteralPath (Join-Path $app 'cedarflake_ame.exe'))) {
    throw 'Uninstall left the installed service or an executable behind'
}
Write-Host "$arch MSI installed, restored a missing file on repair, and removed its application and service."
