[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$CandidateRoot,
    [Parameter(Mandatory = $true)][string]$Tag,
    [Parameter(Mandatory = $true)][string]$SourceCommit,
    [ValidateSet('unsigned', 'signed')][string]$SigningMode = 'unsigned',
    [string]$ExpectedBrokerPublisher = ''
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "release_journal_broker_common.ps1")
. (Join-Path $PSScriptRoot "release_classification_runtime.ps1")

if (($SigningMode -ceq 'signed' -and [string]::IsNullOrWhiteSpace($ExpectedBrokerPublisher)) -or
    ($SigningMode -ceq 'unsigned' -and -not [string]::IsNullOrWhiteSpace($ExpectedBrokerPublisher))) {
    throw 'Candidate signing mode and publisher must agree'
}

$root = [System.IO.Path]::GetFullPath($CandidateRoot)
$payloadRoot = Join-Path $root "payload"
$manifestPath = Join-Path $root "manifest.json"
$topItems = @(Get-ChildItem -LiteralPath $root -Force)
if (
    $topItems.Count -ne 2 -or
    @($topItems | Where-Object { $_.Name -ceq "manifest.json" -and -not $_.PSIsContainer }).Count -ne 1 -or
    @($topItems | Where-Object { $_.Name -ceq "payload" -and $_.PSIsContainer }).Count -ne 1
) {
    throw "Unsigned candidate requires exactly one manifest.json and payload directory"
}
$allItems = @(Get-ChildItem -LiteralPath $payloadRoot -Recurse -Force)
if ($allItems.Count -eq 0 -or $allItems.Count -gt 4096) {
    throw "Unsigned candidate item count is outside the release bound"
}
foreach ($item in @($topItems + $allItems)) {
    if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "Unsigned candidate contains a reparse point"
    }
}
$manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
$expectedProperties = @("format", "tag", "sourceCommit", "signTargets", "files")
$actualProperties = @($manifest.PSObject.Properties.Name)
if (
    $actualProperties.Count -ne $expectedProperties.Count -or
    @($expectedProperties | Where-Object { $actualProperties -cnotcontains $_ }).Count -ne 0 -or
    $manifest.format -cne "cedarflake-ame-$SigningMode-v1" -or
    $manifest.tag -cne $Tag -or
    $SourceCommit -cnotmatch '^[0-9a-f]{40}$' -or
    $manifest.sourceCommit -cne $SourceCommit
) {
    throw "Unsigned candidate manifest contract or source identity is invalid"
}
$fixedTargets = @("cedarflake_ame.exe", "cedarflake_ame_journal_broker.exe")
$manifestTargets = @($manifest.signTargets)
if (
    $manifestTargets.Count -ne 2 -or
    $manifestTargets[0] -cne $fixedTargets[0] -or
    $manifestTargets[1] -cne $fixedTargets[1]
) {
    throw "Unsigned candidate fixed executable list changed"
}
$manifestPaths = [System.Collections.Generic.HashSet[string]]::new(
    [System.StringComparer]::OrdinalIgnoreCase
)
$payloadPrefix = $payloadRoot.TrimEnd("\") + "\"
$totalLength = 0L
foreach ($entry in @($manifest.files)) {
    $entryProperties = @($entry.PSObject.Properties.Name)
    $relativePath = [string]$entry.relativePath
    $segments = @($relativePath.Split("/"))
    if (
        $entryProperties.Count -ne 3 -or
        $entryProperties -cnotcontains "relativePath" -or
        $entryProperties -cnotcontains "length" -or
        $entryProperties -cnotcontains "sha256" -or
        [string]::IsNullOrWhiteSpace($relativePath) -or
        $relativePath.Contains("\") -or
        $relativePath.Contains(":") -or
        [System.IO.Path]::IsPathRooted($relativePath) -or
        @($segments | Where-Object { [string]::IsNullOrEmpty($_) -or $_ -eq "." -or $_ -eq ".." }).Count -ne 0 -or
        -not $manifestPaths.Add($relativePath)
    ) {
        throw "Unsigned candidate manifest contains an unsafe or duplicate path"
    }
    $filePath = [System.IO.Path]::GetFullPath(
        (Join-Path $payloadRoot $relativePath.Replace("/", "\"))
    )
    if (
        -not $filePath.StartsWith($payloadPrefix, [System.StringComparison]::OrdinalIgnoreCase) -or
        -not (Test-Path -LiteralPath $filePath -PathType Leaf)
    ) {
        throw "Unsigned candidate manifest path escapes or is missing from payload"
    }
    $file = Get-Item -LiteralPath $filePath -Force
    $expectedLength = [Convert]::ToInt64($entry.length)
    $expectedHash = [string]$entry.sha256
    if (
        $expectedLength -lt 0 -or
        $file.Length -ne $expectedLength -or
        $expectedHash -cnotmatch '^[0-9a-f]{64}$' -or
        (Get-FileHash -LiteralPath $filePath -Algorithm SHA256).Hash.ToLowerInvariant() -cne $expectedHash
    ) {
        throw "Unsigned candidate manifest length or SHA-256 verification failed"
    }
    $totalLength += $expectedLength
}
$files = @($allItems | Where-Object { -not $_.PSIsContainer })
if (
    $files.Count -eq 0 -or
    $files.Count -ne $manifestPaths.Count -or
    $totalLength -gt 2147483648
) {
    throw "Unsigned candidate manifest does not exactly cover its bounded payload"
}
foreach ($file in $files) {
    $relativePath = [System.IO.Path]::GetRelativePath(
        $payloadRoot, $file.FullName
    ).Replace("\", "/")
    if (-not $manifestPaths.Contains($relativePath)) {
        throw "Unsigned candidate payload contains an unlisted file"
    }
    if ($file.Extension -in @(".exe", ".dll")) {
        Assert-AmeBrokerMachineFacts -Machine (Get-AmeBrokerPeMachine -Path $file.FullName)
    }
}
$requiredFiles = @(
    "cedarflake_ame.exe",
    "cedarflake_ame_journal_broker.exe",
    "cedarflake_ame_classification_worker.exe",
    "rust_lib_cedarflake_ame.dll",
    "flutter_windows.dll",
    "data/app.so",
    "data/icudtl.dat"
) + @(Get-AmeClassificationRuntimePayloadNames)
foreach ($requiredFile in $requiredFiles) {
    if (-not $manifestPaths.Contains($requiredFile)) {
        throw "Unsigned candidate misses required runtime file: $requiredFile"
    }
}
if (@($files | Where-Object {
    $rel = [System.IO.Path]::GetRelativePath($payloadRoot, $_.FullName).Replace("\", "/")
    $rel.StartsWith("data/flutter_assets/", [System.StringComparison]::OrdinalIgnoreCase)
}).Count -eq 0) {
    throw "Unsigned candidate is missing Flutter assets"
}
foreach ($target in $fixedTargets) {
    $path = Join-Path $payloadRoot $target
    Assert-AmeBrokerPayload -Path $path -ExpectedPublisher $ExpectedBrokerPublisher | Out-Null
    if ($SigningMode -ceq 'unsigned') {
        $signature = Get-AuthenticodeSignature -LiteralPath $path
        if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::NotSigned) {
            throw "Unsigned candidate executable unexpectedly carries a signature: $target"
        }
    }
}
Assert-AmeBrokerBinary -Path (Join-Path $payloadRoot $fixedTargets[1]) `
    -ExpectedPublisher $ExpectedBrokerPublisher | Out-Null
Write-Output "${SigningMode}_candidate_verified"
