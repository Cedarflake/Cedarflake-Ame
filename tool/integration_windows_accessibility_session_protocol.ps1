. (Join-Path $PSScriptRoot "integration_windows_accessibility_assembly_progress.ps1")

function Assert-AmeWindowsUiaSessionRequest {
    param([object]$Request, [string]$Token, [int]$Sequence)

    if ($Request.format -cne "ame-windows-uia-session-v1" -or
        $Request.token -cne $Token -or $Request.sequence -ne $Sequence -or
        ($Request.sequence -isnot [int] -and $Request.sequence -isnot [long]) -or
        ($Request.targetProcessId -isnot [int] -and $Request.targetProcessId -isnot [long]) -or
        $Request.targetProcessId -le 0 -or $Request.targetProcessId -gt [int]::MaxValue -or
        $Request.resultFile -isnot [string] -or
        $Request.resultFile -cnotmatch '^[a-z0-9-]+\.json$' -or
        $Request.phase -cnotin @(
            "runtime-preparation", "native-semantics-ready", "application-ready",
            "source-menu-open", "source-menu-closed", "square-range-loading",
            "viewer-open", "viewer-menu-open", "viewer-menu-closed",
            "settings-menu-open", "settings-menu-closed"
        )) {
        throw "Windows UIA session request does not match its owned sequence"
    }
}

function Get-AmeWindowsUiaSessionCompletionName {
    param([string]$ResultPath, [string]$Token, [string]$Phase, [int]$TargetProcessId)

    $names = Get-AmeWindowsUiaAssemblyProgressNames -ResultPath $ResultPath -Token $Token `
        -Phase $Phase -TargetProcessId $TargetProcessId
    return "$($names['UIAutomationTypes-entered'])-invocation-returned"
}

function Write-AmeWindowsUiaSessionRequest {
    param([string]$ScratchRoot, [object]$Request)

    Assert-AmeWindowsUiaSessionRequest -Request $Request -Token $Request.token -Sequence $Request.sequence
    $path = Join-Path $ScratchRoot ("uia-command-{0:D2}.json" -f $Request.sequence)
    $draft = [IO.Path]::ChangeExtension($path, "tmp")
    if ([IO.File]::Exists($path) -or [IO.File]::Exists($draft)) {
        throw "Windows UIA session command must use fresh scratch storage"
    }
    [IO.File]::WriteAllText($draft, ($Request | ConvertTo-Json -Compress), [Text.UTF8Encoding]::new($false))
    [IO.File]::Move($draft, $path)
}

function Read-AmeWindowsUiaSessionRequest {
    param([string]$Path, [string]$Token, [int]$Sequence)

    if ((Get-Item -LiteralPath $Path).Length -gt 4096) {
        throw "Windows UIA session command exceeds its bounded record size"
    }
    $request = [IO.File]::ReadAllText($Path) | ConvertFrom-Json
    Assert-AmeWindowsUiaSessionRequest -Request $request -Token $Token -Sequence $Sequence
    return $request
}
