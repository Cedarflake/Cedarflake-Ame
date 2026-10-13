function Get-AmeWindowsUiaAssemblyProgressNames {
    param(
        [Parameter(Mandatory = $true)] [string]$ResultPath,
        [Parameter(Mandatory = $true)] [string]$Token,
        [Parameter(Mandatory = $true)] [string]$Phase,
        [Parameter(Mandatory = $true)] [int]$TargetProcessId
    )

    $identity = @(
        [System.IO.Path]::GetFullPath($ResultPath), $Token, $Phase, $TargetProcessId
    ) -join [char]0
    $hash = [System.Security.Cryptography.SHA256]::Create()
    try {
        $digest = [System.BitConverter]::ToString(
            $hash.ComputeHash([System.Text.Encoding]::UTF8.GetBytes($identity))
        ).Replace("-", "")
    } finally {
        $hash.Dispose()
    }
    $names = [ordered]@{}
    foreach ($assembly in @("UIAutomationTypes", "UIAutomationClient")) {
        foreach ($stage in @("entered", "loaded", "verified")) {
            $key = "$assembly-$stage"
            $names[$key] = "Local\AmeUiaAssembly-$digest-$key"
        }
    }
    return $names
}

function Close-AmeWindowsUiaAssemblyProgress {
    param(
        [AllowNull()]
        [System.Collections.Generic.Dictionary[string, System.Threading.EventWaitHandle]]$Progress
    )

    if ($null -eq $Progress) { return }
    $failures = [System.Collections.Generic.List[System.Exception]]::new()
    foreach ($handle in $Progress.Values) {
        try { $handle.Dispose() } catch { $failures.Add($_.Exception) }
    }
    $Progress.Clear()
    if ($failures.Count -ne 0) {
        throw [System.AggregateException]::new("Windows UIA assembly marker cleanup failed", $failures)
    }
}

function New-AmeWindowsUiaAssemblyProgress {
    param(
        [Parameter(Mandatory = $true)] [string]$ResultPath,
        [Parameter(Mandatory = $true)] [string]$Token,
        [Parameter(Mandatory = $true)] [string]$Phase,
        [Parameter(Mandatory = $true)] [int]$TargetProcessId,
        [Parameter(Mandatory = $true)] [ValidateSet("Create", "Open")] [string]$Mode
    )

    $names = Get-AmeWindowsUiaAssemblyProgressNames -ResultPath $ResultPath -Token $Token `
        -Phase $Phase -TargetProcessId $TargetProcessId
    $progress = [System.Collections.Generic.Dictionary[string, System.Threading.EventWaitHandle]]::new(
        [System.StringComparer]::Ordinal
    )
    try {
        foreach ($entry in $names.GetEnumerator()) {
            if ($Mode -ceq "Open") {
                $handle = [System.Threading.EventWaitHandle]::OpenExisting($entry.Value)
            } else {
                $created = $false
                $handle = [System.Threading.EventWaitHandle]::new(
                    $false, [System.Threading.EventResetMode]::ManualReset, $entry.Value, [ref]$created
                )
                if (-not $created) {
                    $handle.Dispose()
                    throw "Windows UIA assembly markers already have an owner"
                }
            }
            $progress.Add($entry.Key, $handle)
        }
        return $progress
    } catch {
        $failure = $_.Exception
        try { Close-AmeWindowsUiaAssemblyProgress $progress } catch {
            $failure.Data["assemblyMarkerCleanupFailure"] = $_.Exception.Message
        }
        throw $failure
    }
}

function Set-AmeWindowsUiaAssemblyProgress {
    param(
        [AllowNull()]
        [System.Collections.Generic.Dictionary[string, System.Threading.EventWaitHandle]]$Progress,
        [Parameter(Mandatory = $true)]
        [ValidateSet("UIAutomationTypes", "UIAutomationClient")] [string]$Name,
        [Parameter(Mandatory = $true)]
        [ValidateSet("entered", "loaded", "verified")] [string]$Stage
    )

    if ($null -ne $Progress) {
        $null = $Progress["$Name-$Stage"].Set()
    }
}

function Get-AmeWindowsUiaAssemblyProgress {
    param(
        [Parameter(Mandatory = $true)]
        [System.Collections.Generic.Dictionary[string, System.Threading.EventWaitHandle]]$Progress
    )

    $snapshot = [ordered]@{}
    foreach ($assembly in @("UIAutomationTypes", "UIAutomationClient")) {
        $snapshot[$assembly] = [ordered]@{}
        foreach ($stage in @("entered", "loaded", "verified")) {
            $snapshot[$assembly][$stage] = $Progress["$assembly-$stage"].WaitOne(0)
        }
    }
    return [pscustomobject]$snapshot
}

function Complete-AmeWindowsUiaAssemblyProgress {
    param(
        [AllowNull()]
        [System.Collections.Generic.Dictionary[string, System.Threading.EventWaitHandle]]$Progress,
        [Parameter(Mandatory = $true)] [object]$Cleanup
    )

    $snapshot = $null
    try {
        if ($null -ne $Progress -and $Cleanup.ProcessExited -eq $true -and $Cleanup.JobClosed -eq $true) {
            $snapshot = Get-AmeWindowsUiaAssemblyProgress $Progress
        }
    } catch {
        $Cleanup.Failures.Add([pscustomobject]@{
            stage = "read-assembly-markers"; exception = $_.Exception
        })
    } finally {
        try { Close-AmeWindowsUiaAssemblyProgress $Progress } catch {
            $Cleanup.Failures.Add([pscustomobject]@{
                stage = "close-assembly-markers"; exception = $_.Exception
            })
        }
    }
    return $snapshot
}
