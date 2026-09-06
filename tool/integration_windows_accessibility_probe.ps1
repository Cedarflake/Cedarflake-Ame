[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [int]$TargetProcessId,
    [Parameter(Mandatory = $true)]
    [string]$Phase,
    [Parameter(Mandatory = $true)]
    [string]$ResultPath,
    [Parameter(Mandatory = $true)]
    [string]$Token
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "integration_windows_accessibility_evidence.ps1")
$probeElapsed = [System.Diagnostics.Stopwatch]::StartNew()

$script:probeRecord = @{
    ResultPath = $ResultPath
    Token = $Token
    Phase = $Phase
    TargetProcessId = $TargetProcessId
    ProbeProcessId = $PID
    Status = "progress"
    Stage = "loading-assemblies"
    Attempt = 0
    WindowCount = 0
    ElementCount = 0
    LastMismatch = $null
    Failure = $null
}

function Publish-AmeWindowsUiaProbeProgress {
    param([Parameter(Mandatory = $true)] [string]$Stage)

    $script:probeRecord.Stage = $Stage
    $script:probeRecord.ElapsedMilliseconds = $probeElapsed.ElapsedMilliseconds
    Write-AmeWindowsUiaProbeRecord @script:probeRecord
}

$updateLibraryName = -join [char[]](0x66F4, 0x65B0, 0x56FE, 0x5E93)
$removeFromAmeName = (
    (-join [char[]](0x4ECE)) + " Ame " +
    (-join [char[]](0x4E2D, 0x79FB, 0x9664))
)
$copyPathName = -join [char[]](0x590D, 0x5236, 0x8DEF, 0x5F84)
$openInExplorerName = -join [char[]](
    0x5728,
    0x6587,
    0x4EF6,
    0x8D44,
    0x6E90,
    0x7BA1,
    0x7406,
    0x5668,
    0x4E2D,
    0x6253,
    0x5F00
)
$lightName = -join [char[]](0x6D45, 0x8272)
$darkName = -join [char[]](0x6DF1, 0x8272)
$settingsName = -join [char[]](0x8BBE, 0x7F6E)
$themeName = -join [char[]](0x5E94, 0x7528, 0x4E3B, 0x9898)
$themeDescriptionName = (
    $themeName + "`n" +
    (-join [char[]](
        0x9009,
        0x62E9,
        0x660E,
        0x6697,
        0x5916,
        0x89C2,
        0xFF0C,
        0x4E3B,
        0x9898,
        0x8272,
        0x8DDF,
        0x968F
    )) +
    " Windows"
)
$synchronizingName = -join [char[]](
    0x6B63,
    0x5728,
    0x66F4,
    0x65B0,
    0x56FE,
    0x5E93
)
$expandFolderName = -join [char[]](
    0x5C55,
    0x5F00,
    0x6587,
    0x4EF6,
    0x5939
)
$sourceButtonSuffix = "$synchronizingName. $expandFolderName"

function ConvertTo-AmeWindowsUiaDiagnosticName {
    param(
        [AllowEmptyString()]
        [string]$Name
    )

    $value = $Name.Replace("`r", "<CR>")
    $value = $value.Replace("`n", "<LF>")
    $value = $value.Replace("`t", "<TAB>")
    $value = [regex]::Replace($value, "[ ]+", " ").Trim()
    if ([string]::IsNullOrEmpty($value)) {
        return "<empty>"
    }
    if ($value -match "^[0-9]+\.[a-zA-Z0-9]{1,8}$") {
        return "<numbered-media-name>"
    }
    if (
        $value -match "(?i)(^|[^a-z0-9])([a-z]:[\\/]|\\\\\?\\|\\\\[^\\])"
    ) {
        return "<absolute-path-redacted>"
    }
    if ($value.Length -gt 48) {
        $value = $value.Substring(0, 48) + "..."
    }
    return $value.Replace("'", "''")
}

function Format-AmeWindowsUiaDiagnosticSummary {
    param(
        [Parameter(Mandatory = $true)]
        [object]$Snapshot
    )

    $records = @($Snapshot.Records)
    $sample = @(
        $records |
            ForEach-Object {
                $name = ConvertTo-AmeWindowsUiaDiagnosticName -Name $_.Name
                "name='$name',type='$($_.ControlType)',offscreen=$($_.IsOffscreen)," +
                    "control=$($_.IsControlElement),content=$($_.IsContentElement)"
            } |
            Sort-Object -Unique |
            Select-Object -First 20
    )
    $renderedSample = if ($sample.Count -eq 0) {
        "<none>"
    } else {
        $sample -join "; "
    }
    return (
        "window_count=$($Snapshot.WindowCount)," +
        "reachable_count=$($records.Count),sample=[$renderedSample]"
    )
}

function Format-AmeWindowsUiaNearMatch {
    param(
        [Parameter(Mandatory = $true)]
        [object[]]$Records,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedName
    )

    $prefix = $ExpectedName.Split(" ")[0]
    $near = @(
        $Records |
            Where-Object {
                $_.Name -and $_.Name.StartsWith(
                    $prefix,
                    [System.StringComparison]::Ordinal
                )
            } |
            Select-Object -First 1
    )
    if ($near.Count -eq 0) {
        return "<none>"
    }
    $safeName = ConvertTo-AmeWindowsUiaDiagnosticName -Name $near[0].Name
    if ($safeName -eq "<absolute-path-redacted>") {
        return $safeName
    }
    $codeUnits = @(
        $near[0].Name.ToCharArray() |
            Select-Object -First 64 |
            ForEach-Object { "{0:X4}" -f [int]$_ }
    ) -join ","
    return (
        "name='$safeName',type='$($near[0].ControlType)'," +
        "utf16=[$codeUnits]"
    )
}

function Get-AmeWindowsUiaSnapshot {
    param(
        [Parameter(Mandatory = $true)]
        [int]$TargetProcessId
    )

    $processCondition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ProcessIdProperty,
        $TargetProcessId
    )
    Publish-AmeWindowsUiaProbeProgress -Stage "locating-window"
    $windows = [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
        [System.Windows.Automation.TreeScope]::Children,
        $processCondition
    )
    $script:probeRecord.WindowCount = $windows.Count
    $records = [System.Collections.Generic.List[object]]::new()
    $cacheRequest = [System.Windows.Automation.CacheRequest]::new()
    $cacheRequest.TreeScope = [System.Windows.Automation.TreeScope]::Element
    $cacheRequest.TreeFilter = [System.Windows.Automation.Automation]::RawViewCondition
    $cacheRequest.AutomationElementMode = [System.Windows.Automation.AutomationElementMode]::None
    foreach ($property in @(
        [System.Windows.Automation.AutomationElement]::NameProperty,
        [System.Windows.Automation.AutomationElement]::IsOffscreenProperty,
        [System.Windows.Automation.AutomationElement]::IsEnabledProperty,
        [System.Windows.Automation.AutomationElement]::IsControlElementProperty,
        [System.Windows.Automation.AutomationElement]::IsContentElementProperty,
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty
    )) {
        $cacheRequest.Add($property)
    }
    foreach ($window in $windows) {
        Publish-AmeWindowsUiaProbeProgress -Stage "finding-elements"
        $activeCache = $cacheRequest.Activate()
        try {
            $elements = $window.FindAll(
                [System.Windows.Automation.TreeScope]::Subtree,
                [System.Windows.Automation.Condition]::TrueCondition
            )
        } finally {
            $activeCache.Dispose()
        }
        $script:probeRecord.ElementCount += $elements.Count
        Publish-AmeWindowsUiaProbeProgress -Stage "reading-properties"
        foreach ($element in $elements) {
            try {
                $current = $element.Cached
                if (-not $current.IsControlElement -and -not $current.IsContentElement) {
                    continue
                }
                $records.Add([pscustomobject]@{
                    Name = $current.Name
                    IsOffscreen = $current.IsOffscreen
                    IsEnabled = $current.IsEnabled
                    IsControlElement = $current.IsControlElement
                    IsContentElement = $current.IsContentElement
                    ControlType = $current.ControlType.ProgrammaticName
                })
            } catch [System.Windows.Automation.ElementNotAvailableException] {
                continue
            }
        }
    }
    return [pscustomobject]@{
        WindowCount = $windows.Count
        Records = @($records.ToArray())
    }
}

function Get-AmeWindowsUiaPhaseFailure {
    param(
        [Parameter(Mandatory = $true)]
        [int]$TargetProcessId,
        [Parameter(Mandatory = $true)]
        [string]$Phase
    )

    $contract = switch ($Phase) {
        "application-ready" {
            @{
                Expected = @(
                    @{
                        Name = "Pictures`n$sourceButtonSuffix"
                        ControlType = "ControlType.Button"
                    },
                    @{
                        Name = "Documents`n$sourceButtonSuffix"
                        ControlType = "ControlType.Button"
                    },
                    @{
                        Name = "Archive`n$sourceButtonSuffix"
                        ControlType = "ControlType.Button"
                    }
                )
                Forbidden = @()
            }
            break
        }
        "source-menu-open" {
            @{
                Expected = @(
                    @{
                        Name = $updateLibraryName
                        ControlType = "ControlType.Button"
                    },
                    @{
                        Name = $openInExplorerName
                        ControlType = "ControlType.Button"
                    },
                    @{
                        Name = $removeFromAmeName
                        ControlType = "ControlType.Button"
                    }
                )
                Forbidden = @()
            }
            break
        }
        "source-menu-closed" {
            @{
                Expected = @(
                    @{
                        Name = "Pictures`n$sourceButtonSuffix"
                        ControlType = "ControlType.Button"
                    }
                )
                Forbidden = @(
                    $updateLibraryName,
                    $openInExplorerName,
                    $removeFromAmeName
                )
            }
            break
        }
        "square-range-loading" {
            @{
                Expected = @(
                    @{
                        Name = "Pictures`n$sourceButtonSuffix"
                        ControlType = "ControlType.Custom"
                    }
                )
                Forbidden = @(
                    $updateLibraryName,
                    $openInExplorerName,
                    $removeFromAmeName
                )
            }
            break
        }
        "viewer-open" {
            @{
                Expected = @(
                    @{
                        Name = "1:1"
                        ControlType = "ControlType.Button"
                    },
                    @{
                        Name = (-join [char[]](0x91CD, 0x8BD5))
                        ControlType = "ControlType.Button"
                    }
                )
                Forbidden = @()
            }
            break
        }
        "viewer-menu-open" {
            @{
                Expected = @(
                    @{
                        Name = $copyPathName
                        ControlType = "ControlType.Button"
                    },
                    @{
                        Name = $openInExplorerName
                        ControlType = "ControlType.Button"
                    }
                )
                Forbidden = @()
            }
            break
        }
        "viewer-menu-closed" {
            @{
                Expected = @(
                    @{
                        Name = "1:1"
                        ControlType = "ControlType.Button"
                    },
                    @{
                        Name = (-join [char[]](0x91CD, 0x8BD5))
                        ControlType = "ControlType.Button"
                    }
                )
                Forbidden = @($copyPathName, $openInExplorerName)
            }
            break
        }
        "settings-menu-open" {
            @{
                Expected = @(
                    @{
                        Name = $lightName
                        ControlType = "ControlType.Button"
                    },
                    @{
                        Name = $darkName
                        ControlType = "ControlType.Button"
                    }
                )
                Forbidden = @()
            }
            break
        }
        "settings-menu-closed" {
            @{
                Expected = @(
                    @{
                        Name = $settingsName
                        ControlType = "ControlType.Text"
                    },
                    @{
                        Name = $themeDescriptionName
                        ControlType = "ControlType.Custom"
                    }
                )
                Forbidden = @($lightName, $darkName)
            }
            break
        }
        default {
            return "unknown Windows UIA probe phase '$Phase'"
        }
    }

    $snapshot = Get-AmeWindowsUiaSnapshot -TargetProcessId $TargetProcessId
    Publish-AmeWindowsUiaProbeProgress -Stage "asserting-contract"
    $records = @($snapshot.Records)
    $diagnostic = Format-AmeWindowsUiaDiagnosticSummary -Snapshot $snapshot
    if ($records.Count -eq 0) {
        return (
            "the Ame process has no reachable Windows UI Automation elements; " +
            $diagnostic
        )
    }
    foreach ($element in $contract.Expected) {
        $name = [string]$element.Name
        $controlType = [string]$element.ControlType
        $matches = @(
            $records |
                Where-Object {
                    $_.Name -ceq $name -and
                    $_.ControlType -ceq $controlType -and
                    -not $_.IsOffscreen -and
                    ($_.IsControlElement -or $_.IsContentElement)
                }
        )
        if ($matches.Count -eq 0) {
            $nearMatch = Format-AmeWindowsUiaNearMatch `
                -Records $records `
                -ExpectedName $name
            return (
                "reachable on-screen element '$name' with control type " +
                "'$controlType' is missing; " +
                "near_match=[$nearMatch]; $diagnostic"
            )
        }
    }
    foreach ($name in $contract.Forbidden) {
        $matches = @($records | Where-Object { $_.Name -ceq $name })
        if ($matches.Count -ne 0) {
            return (
                "dismissed overlay element '$name' remains in the native tree; " +
                $diagnostic
            )
        }
    }
    return $null
}

function Assert-AmeWindowsUiaPhase {
    param(
        [Parameter(Mandatory = $true)]
        [int]$TargetProcessId,
        [Parameter(Mandatory = $true)]
        [string]$Phase
    )

    $failure = $null
    do {
        $script:probeRecord.Attempt += 1
        $script:probeRecord.WindowCount = 0
        $script:probeRecord.ElementCount = 0
        try {
            $failure = Get-AmeWindowsUiaPhaseFailure `
                -TargetProcessId $TargetProcessId `
                -Phase $Phase
        } catch {
            $failure = $_.Exception.Message
        }
        if ($null -eq $failure) {
            return
        }
        $script:probeRecord.LastMismatch = $failure
        Publish-AmeWindowsUiaProbeProgress -Stage $script:probeRecord.Stage
        Start-Sleep -Milliseconds 50
    } while ($true)
}

$failure = $null
try {
    Publish-AmeWindowsUiaProbeProgress -Stage "loading-assemblies"
    if ([System.Threading.Thread]::CurrentThread.GetApartmentState() -ne 'MTA') {
        throw "Windows UIA probing requires an MTA thread without an application window"
    }
    Add-Type -AssemblyName UIAutomationClient
    Add-Type -AssemblyName UIAutomationTypes
    Assert-AmeWindowsUiaPhase -TargetProcessId $TargetProcessId -Phase $Phase
} catch {
    $failure = $_.Exception.Message
}
$script:probeRecord.Status = "complete"
$script:probeRecord.Failure = $failure
Publish-AmeWindowsUiaProbeProgress -Stage "complete"
