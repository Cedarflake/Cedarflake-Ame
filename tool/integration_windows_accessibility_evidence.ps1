function Write-AmeWindowsUiaProbeRecord {
    param(
        [Parameter(Mandatory = $true)] [string]$ResultPath,
        [Parameter(Mandatory = $true)] [string]$Token,
        [Parameter(Mandatory = $true)] [string]$Phase,
        [Parameter(Mandatory = $true)] [int]$TargetProcessId,
        [Parameter(Mandatory = $true)] [int]$ProbeProcessId,
        [ValidateSet("progress", "complete")] [string]$Status = "progress",
        [ValidateSet("loading-assemblies", "locating-window", "finding-elements", "reading-properties", "asserting-contract", "complete")]
        [string]$Stage,
        [ValidateRange(0, 2147483647)] [int]$Attempt = 0,
        [ValidateRange(0, 2147483647)] [int]$WindowCount = 0,
        [ValidateRange(0, 2147483647)] [int]$ElementCount = 0,
        [ValidateRange(0, 2147483647)] [int]$ElapsedMilliseconds = 0,
        [AllowNull()] [string]$LastMismatch,
        [AllowNull()] [string]$Failure
    )

    $record = [ordered]@{
        format = "ame-windows-uia-probe-v2"
        token = $Token
        phase = $Phase
        targetProcessId = $TargetProcessId
        probeProcessId = $ProbeProcessId
        status = $Status
        stage = $Stage
        attempt = $Attempt
        windowCount = $WindowCount
        elementCount = $ElementCount
        elapsedMilliseconds = $ElapsedMilliseconds
        lastMismatch = $(if ($LastMismatch) { $LastMismatch } else { $null })
        failure = $(if ($Failure) { $Failure } else { $null })
    }
    $utf8 = [System.Text.UTF8Encoding]::new($false)
    $payload = $record | ConvertTo-Json -Compress
    if ($utf8.GetByteCount($payload) -gt 65536) {
        throw "Windows UIA probe evidence exceeds its bounded record size"
    }
    $draftPath = [System.IO.Path]::ChangeExtension($ResultPath, "tmp")
    [System.IO.File]::WriteAllText($draftPath, $payload, $utf8)
    if ([System.IO.File]::Exists($ResultPath)) {
        [System.IO.File]::Replace(
            $draftPath,
            $ResultPath,
            [System.Management.Automation.Language.NullString]::Value
        )
    } else {
        [System.IO.File]::Move($draftPath, $ResultPath)
    }
}

function Read-AmeWindowsUiaProbeRecord {
    param(
        [Parameter(Mandatory = $true)] [string]$ResultPath,
        [Parameter(Mandatory = $true)] [string]$Token,
        [Parameter(Mandatory = $true)] [string]$Phase,
        [Parameter(Mandatory = $true)] [int]$TargetProcessId,
        [Parameter(Mandatory = $true)] [int]$ProbeProcessId
    )

    if ((Get-Item -LiteralPath $ResultPath).Length -gt 65536) {
        throw "Windows UIA probe evidence exceeds its bounded record size"
    }
    $record = [System.IO.File]::ReadAllText($ResultPath) | ConvertFrom-Json
    if (
        $record.format -cne "ame-windows-uia-probe-v2" -or
        $record.token -cne $Token -or $record.phase -cne $Phase -or
        $record.targetProcessId -ne $TargetProcessId -or
        $record.probeProcessId -ne $ProbeProcessId
    ) {
        throw "Windows UIA probe evidence does not match its owned request"
    }
    if (
        $record.status -cnotin @("progress", "complete") -or
        $record.stage -cnotin @(
            "loading-assemblies", "locating-window", "finding-elements",
            "reading-properties", "asserting-contract", "complete"
        ) -or
        (($record.status -ceq "complete") -ne ($record.stage -ceq "complete"))
    ) {
        throw "Windows UIA probe evidence has an invalid progress state"
    }
    foreach ($field in @("targetProcessId", "probeProcessId", "attempt", "windowCount", "elementCount", "elapsedMilliseconds")) {
        $value = $record.$field
        if (($value -isnot [int] -and $value -isnot [long]) -or $value -lt 0 -or $value -gt [int]::MaxValue) {
            throw "Windows UIA probe evidence has an invalid numeric field"
        }
    }
    foreach ($field in @("lastMismatch", "failure")) {
        if ($null -ne $record.$field -and $record.$field -isnot [string]) {
            throw "Windows UIA probe evidence has an invalid diagnostic field"
        }
    }
    return $record
}

function Complete-AmeWindowsAccessibilityRun {
    param(
        [Parameter(Mandatory = $true)]
        [string]$OutputPath,
        [AllowEmptyString()]
        [string]$CapturedOutput = "",
        [AllowEmptyString()]
        [string]$ProbeTranscript = "",
        [int]$ExitCode = 0,
        [AllowNull()]
        [System.Exception]$RunFailure,
        [AllowNull()]
        [System.Exception]$CleanupFailure
    )

    $completion = [ordered]@{
        exitCode = $ExitCode
        runFailure = $(if ($null -ne $RunFailure) { $RunFailure.Message } else { $null })
        cleanupFailure = $(if ($null -ne $CleanupFailure) { $CleanupFailure.Message } else { $null })
        probeEvidenceStatus = $(
            if ($null -ne $RunFailure) { $RunFailure.Data["ameWindowsUiaProbeEvidenceStatus"] } else { $null }
        )
        probeProgress = $(
            if ($null -ne $RunFailure) { $RunFailure.Data["ameWindowsUiaProbeProgress"] } else { $null }
        )
        probeCleanupFailure = $(
            if ($null -ne $RunFailure) { $RunFailure.Data["ameWindowsUiaProbeCleanupFailure"] } else { $null }
        )
    } | ConvertTo-Json -Compress -Depth 4
    $combinedOutput = @(
        $CapturedOutput.TrimEnd()
        $ProbeTranscript.TrimEnd()
        "AME_WINDOWS_ACCESSIBILITY_COMPLETION $completion"
    ) -join [Environment]::NewLine
    $resolvedOutputPath = [System.IO.Path]::GetFullPath($OutputPath)
    $outputDirectory = Split-Path -Parent $resolvedOutputPath
    New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null
    [System.IO.File]::WriteAllText(
        $resolvedOutputPath,
        "$combinedOutput$([Environment]::NewLine)",
        [System.Text.UTF8Encoding]::new($false)
    )
    Write-Host $combinedOutput

    if ($null -ne $RunFailure) {
        throw $RunFailure
    }
    if ($null -ne $CleanupFailure) {
        throw $CleanupFailure
    }
    if ($ExitCode -ne 0) {
        throw "Windows accessibility integration failed with exit code $ExitCode"
    }
}
