function Invoke-AmeR2cROwnedProcess {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [ValidateSet("CargoExactTest", "CargoTestBuild", "PowerShell")]
        [string]$ExecutableKind,
        [Parameter(Mandatory = $true)][string]$FileName,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [Parameter(Mandatory = $true)][string]$WorkingDirectory,
        [AllowNull()][string]$OutputPath,
        [ValidateRange(1, 3600000)][int]$TimeoutMilliseconds = 300000,
        [switch]$Detached
    )
    $auditState = $null
    $boundaryState = $null
    $job = $null
    $process = $null
    $timedOut = $false
    $processId = 0
    $exitCode = -1
    $lines = @()
    try {
        $expectedWorkingDirectory = [IO.Path]::GetFullPath(
            (Join-Path $PSScriptRoot "..")
        ).TrimEnd('\')
        $actualWorkingDirectory = [IO.Path]::GetFullPath($WorkingDirectory).TrimEnd('\')
        if (-not $actualWorkingDirectory.Equals(
                $expectedWorkingDirectory,
                [StringComparison]::OrdinalIgnoreCase
            )) {
            throw "R2c-R verified process boundary rejected its working directory"
        }
        $boundaryPath = Join-Path `
            $PSScriptRoot `
            "acceptance_r2c_change_driven_reliability_common.ps1"
        $boundaryState = Assert-AmeR2cRDeletionSafetySource `
            -SourceText $null `
            -SourcePath $boundaryPath `
            -Label "verified process boundary" `
            -RetainSnapshots
        if ($ExecutableKind -cin @("CargoExactTest", "CargoTestBuild")) {
            Assert-AmeR2cRCargoInvocation -Kind $ExecutableKind -FileName $FileName `
                -Arguments $Arguments -Detached:$Detached
        } else {
            $currentProcess = Microsoft.PowerShell.Management\Get-Process -Id $PID
            try { $expectedExecutable = [IO.Path]::GetFullPath($currentProcess.Path) }
            finally { $currentProcess.Dispose() }
            $actualExecutable = [IO.Path]::GetFullPath($FileName)
            if (-not $actualExecutable.Equals(
                    $expectedExecutable,
                    [StringComparison]::OrdinalIgnoreCase
                )) {
                throw "R2c-R verified process boundary rejected the PowerShell executable"
            }
            if ($Arguments.Count -lt 4 -or
                $Arguments[0] -cne "-NoProfile" -or
                $Arguments[1] -cne "-NonInteractive") {
                throw "R2c-R verified process boundary rejected PowerShell host arguments"
            }
            $payloadLabel = "PowerShell $($Arguments[2]) payload"
            if ($Arguments[2] -ceq "-Command") {
                if ($Arguments.Count -ne 4) {
                    throw "R2c-R verified process boundary rejected ambiguous Command arguments"
                }
                $auditState = Assert-AmeR2cRDeletionSafetySource `
                    -SourceText ([string]$Arguments[3]) `
                    -Label $payloadLabel `
                    -RetainSnapshots
            } elseif ($Arguments[2] -ceq "-EncodedCommand") {
                if ($Arguments.Count -ne 4 -or
                    [string]$Arguments[3] -notmatch '^[A-Za-z0-9+/]+={0,2}$') {
                    throw "R2c-R verified process boundary rejected malformed EncodedCommand arguments"
                }
                try {
                    $payloadBytes = [Convert]::FromBase64String([string]$Arguments[3])
                } catch {
                    throw "R2c-R verified process boundary rejected malformed EncodedCommand payload"
                }
                if (($payloadBytes.Length % 2) -ne 0 -or
                    [Convert]::ToBase64String($payloadBytes) -cne [string]$Arguments[3]) {
                    throw "R2c-R verified process boundary rejected non-canonical EncodedCommand payload"
                }
                $payloadText = [Text.Encoding]::Unicode.GetString($payloadBytes)
                $auditState = Assert-AmeR2cRDeletionSafetySource `
                    -SourceText $payloadText `
                    -Label $payloadLabel `
                    -RetainSnapshots
            } elseif ($Arguments[2] -ceq "-File") {
                if ($Detached -or $Arguments.Count -lt 4) {
                    throw "R2c-R verified process boundary rejected detached or missing File payload"
                }
                $payloadPath = [IO.Path]::GetFullPath([string]$Arguments[3])
                $auditState = Assert-AmeR2cRDeletionSafetySource `
                    -SourceText $null `
                    -SourcePath $payloadPath `
                    -Label $payloadLabel `
                    -RetainSnapshots
            } else {
                throw "R2c-R verified process boundary rejected an unknown PowerShell selector"
            }
        }
        if ($Detached) {
            if ($ExecutableKind -cne "PowerShell" -or $Arguments[2] -ceq "-File") {
                throw "R2c-R verified process boundary rejected an unsafe detached payload"
            }
            if ($null -ne $auditState) {
                Assert-AmeR2cRDeletionAuditSnapshotsHeld -State $auditState
            }
            Assert-AmeR2cRDeletionAuditSnapshotsHeld -State $boundaryState
            $process = Microsoft.PowerShell.Management\Start-Process `
                -FilePath $FileName `
                -ArgumentList $Arguments `
                -WorkingDirectory $actualWorkingDirectory `
                -WindowStyle Hidden `
                -PassThru
            $processId = $process.Id
            return [pscustomobject]@{
                ExitCode = -1
                Lines = @()
                Process = $process
                ProcessId = $processId
                TimedOut = $false
            }
        }
        if ([string]::IsNullOrEmpty($OutputPath)) {
            throw "R2c-R verified process boundary requires an owned output path"
        }
        $job = [AmeR2cRProcessJob]::new()
        if ($null -ne $auditState) {
            Assert-AmeR2cRDeletionAuditSnapshotsHeld -State $auditState
        }
        Assert-AmeR2cRDeletionAuditSnapshotsHeld -State $boundaryState
        $process = $job.Start($FileName, $Arguments, $WorkingDirectory, $OutputPath)
        $processId = $process.Id
        if (-not $process.WaitForExit($TimeoutMilliseconds)) {
            $timedOut = $true
            $job.TerminateOwnedTree()
            if (-not $process.WaitForExit(5000)) { throw "R2c-R timed-out process tree did not terminate after job disposal" }
        } else { $exitCode = [int]$job.PrimaryExitCode }
        $lines = @($job.ReadOutputLines())
    } finally {
        if ($null -ne $job) { $job.Dispose() }
        if ($null -ne $process -and -not $Detached) { $process.Dispose() }
        if ($null -ne $auditState) {
            Close-AmeR2cRDeletionAuditState -State $auditState
        }
        if ($null -ne $boundaryState) {
            Close-AmeR2cRDeletionAuditState -State $boundaryState
        }
    }
    return [pscustomobject]@{ ExitCode = $exitCode; Lines = $lines; ProcessId = $processId; TimedOut = $timedOut }
}

