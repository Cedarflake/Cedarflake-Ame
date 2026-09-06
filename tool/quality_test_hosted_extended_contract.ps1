$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

function Get-AmeHostedJobSource {
    param([string]$Source, [string]$Job)

    $matches = [regex]::Matches(
        $Source, '(?ms)^  ' + [regex]::Escape($Job) + ':\r?\n.*?(?=^  [a-z_]+:|\z)'
    )
    if ($matches.Count -ne 1) {
        throw "Hosted job must occur exactly once: $Job"
    }
    return $matches[0].Value
}

function Assert-AmeHostedExtendedContract {
    param([string]$Gate, [string]$Synthetic)

    foreach ($entry in @(
        @{ Job = "quality_synthetic_windows"; Workflow = "quality_gate_synthetic_windows.yml" },
        @{ Job = "quality_unsigned_windows"; Workflow = "quality_gate_unsigned_windows.yml" }
    )) {
        $source = Get-AmeHostedJobSource $Gate $entry.Job
        foreach ($required in @(
            "    if: inputs.gate_mode == 'daily'",
            "    uses: ./.github/workflows/$($entry.Workflow)",
            '      checkout_ref: ${{ inputs.checkout_ref || github.sha }}'
        )) {
            if (-not $source.Contains($required)) {
                throw "Required hosted wiring is missing from $($entry.Job)"
            }
        }
    }
    $aggregate = Get-AmeHostedJobSource $Gate "quality_windows"
    foreach ($job in @(
        "daily_components", "quality_synthetic_windows", "quality_unsigned_windows",
        "release_verify_windows"
    )) {
        if ($aggregate -notmatch ('(?m)^      - ' + $job + '\r?$')) {
            throw "The aggregate gate does not depend on $job"
        }
        if (-not $aggregate.Contains(('${{ needs.' + $job + '.result }}'))) {
            throw "The aggregate gate does not consume $job"
        }
    }
    if (-not $aggregate.Contains("    if: always()")) {
        throw "The aggregate gate must run after an unsuccessful dependency"
    }
    $cases = @([regex]::Matches($Synthetic, '(?m)^          - case: ([a-z]+)\r?$') |
        ForEach-Object { $_.Groups[1].Value } | Sort-Object)
    if (($cases -join ",") -cne "jpeg,scan,usn") {
        throw "Hosted synthetic coverage must contain exactly jpeg, scan, and usn"
    }
    foreach ($required in @(
        "      fail-fast: false", "    timeout-minutes: 45", "  contents: read",
        '        run: ./tool/performance_run_synthetic.ps1 -Case $env:AME_SYNTHETIC_CASE',
        "        if: always()", "          if-no-files-found: error"
    )) {
        if (-not $Synthetic.Contains($required)) {
            throw "Synthetic workload admission or evidence contract is missing: $required"
        }
    }
    if ($Synthetic -match 'continue-on-error:|secrets:|pull_request_target:|contents: write') {
        throw "Synthetic verification cannot ignore failures or obtain privileged authority"
    }
    return $aggregate
}

$repositoryRoot = Get-AmeRepositoryRoot
$gate = Get-Content -LiteralPath (Join-Path $repositoryRoot `
    ".github/workflows/quality_gate_windows.yml") -Raw -Encoding UTF8
$synthetic = Get-Content -LiteralPath (Join-Path $repositoryRoot `
    ".github/workflows/quality_gate_synthetic_windows.yml") -Raw -Encoding UTF8
$aggregate = Assert-AmeHostedExtendedContract $gate $synthetic
foreach ($mutation in @(
    @{ Gate = $gate.Replace("      - quality_synthetic_windows", ""); Synthetic = $synthetic },
    @{ Gate = $gate.Replace("      - quality_unsigned_windows", ""); Synthetic = $synthetic },
    @{ Gate = $gate; Synthetic = $synthetic.Replace("          - case: usn", "          - case: skipped") },
    @{ Gate = $gate; Synthetic = $synthetic.Replace("      fail-fast: false", "      fail-fast: true") },
    @{ Gate = $gate; Synthetic = $synthetic + "`ncontinue-on-error: true`n" }
)) {
    $rejected = $false
    try { Assert-AmeHostedExtendedContract $mutation.Gate $mutation.Synthetic | Out-Null }
    catch { $rejected = $true }
    if (-not $rejected) { throw "Hosted coverage guard accepted a representative violation" }
}

$inline = [regex]::Match($aggregate, '(?ms)^        run: \|\r?\n(?<script>.*)\z')
if (-not $inline.Success -or $inline.Groups["script"].Value -match '\bexit\b') {
    throw "The aggregate must expose one returning inline result check"
}
$verify = [scriptblock]::Create(
    [regex]::Replace($inline.Groups["script"].Value, '(?m)^          ', '')
)
$resultNames = @("AME_DAILY_RESULT", "AME_SYNTHETIC_RESULT", "AME_UNSIGNED_RESULT")
$environmentNames = $resultNames + @("AME_GATE_MODE", "AME_RELEASE_RESULT")
$saved = @{}
foreach ($name in $environmentNames) { $saved[$name] = [Environment]::GetEnvironmentVariable($name) }
try {
    foreach ($name in $resultNames) { [Environment]::SetEnvironmentVariable($name, "success") }
    $env:AME_GATE_MODE = "daily"
    $env:AME_RELEASE_RESULT = "skipped"
    & $verify
    foreach ($name in $resultNames) {
        foreach ($failure in @("failure", "cancelled", "skipped", "")) {
            [Environment]::SetEnvironmentVariable($name, $failure)
            $rejected = $false
            try { & $verify } catch { $rejected = $true }
            if (-not $rejected) { throw "Aggregate accepted $name=$failure" }
        }
        [Environment]::SetEnvironmentVariable($name, "success")
    }
    $env:AME_GATE_MODE = "release"
    foreach ($name in $resultNames) { [Environment]::SetEnvironmentVariable($name, "skipped") }
    $env:AME_RELEASE_RESULT = "success"
    & $verify
    foreach ($failure in @("failure", "cancelled", "skipped", "")) {
        [Environment]::SetEnvironmentVariable("AME_RELEASE_RESULT", $failure)
        $rejected = $false
        try { & $verify } catch { $rejected = $true }
        if (-not $rejected) { throw "Aggregate accepted release=$failure" }
    }
    $env:AME_GATE_MODE = "unsupported"
    $rejected = $false
    try { & $verify } catch { $rejected = $true }
    if (-not $rejected) { throw "Aggregate accepted an unknown mode" }
} finally {
    foreach ($name in $environmentNames) {
        if ($null -eq $saved[$name]) {
            [Environment]::SetEnvironmentVariable($name, [NullString]::Value)
        } else {
            [Environment]::SetEnvironmentVariable($name, [string]$saved[$name])
        }
    }
}
Write-Output "Hosted extended coverage and failure propagation verified"
