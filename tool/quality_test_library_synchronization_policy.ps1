$ErrorActionPreference = "Stop"

$generator = Join-Path $PSScriptRoot "quality_generate_library_synchronization_policy.ps1"
$temporaryParent = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd(
    [System.IO.Path]::DirectorySeparatorChar,
    [System.IO.Path]::AltDirectorySeparatorChar
)
$scratchRoot = Join-Path $temporaryParent (
    "cedarflake-ame-library-synchronization-policy-" + [guid]::NewGuid().ToString("N")
)
$scratchRoot = [System.IO.Path]::GetFullPath($scratchRoot)
if ([System.IO.Path]::GetDirectoryName($scratchRoot) -ne $temporaryParent) {
    throw "Library synchronization policy guardrail scratch root escaped the temporary directory."
}
[System.IO.Directory]::CreateDirectory($scratchRoot) | Out-Null

$policyPath = Join-Path $scratchRoot "policy.txt"
$outputPath = Join-Path $scratchRoot "library_synchronization_policy.g.dart"
$utf8WithoutBom = [System.Text.UTF8Encoding]::new($false)

function Write-PolicyBytes {
    param(
        [Parameter(Mandatory = $true)]
        [AllowEmptyCollection()]
        [byte[]]$Bytes
    )

    [System.IO.File]::WriteAllBytes($policyPath, $Bytes)
}

function Assert-GeneratorRejected {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Case,
        [switch]$CheckOnly
    )

    $wasRejected = $false
    try {
        if ($CheckOnly) {
            & $generator -PolicyPath $policyPath -OutputPath $outputPath -Check 2>$null | Out-Null
        } else {
            & $generator -PolicyPath $policyPath -OutputPath $outputPath 2>$null | Out-Null
        }
    } catch {
        $wasRejected = $true
    }
    if (-not $wasRejected) {
        throw "Library synchronization policy generator accepted invalid case: $Case"
    }
}

function Assert-EqualBytes {
    param(
        [Parameter(Mandatory = $true)]
        [byte[]]$Actual,
        [Parameter(Mandatory = $true)]
        [byte[]]$Expected,
        [Parameter(Mandatory = $true)]
        [string]$Message
    )

    if ($Actual.Length -ne $Expected.Length) {
        throw $Message
    }
    for ($index = 0; $index -lt $Expected.Length; $index += 1) {
        if ($Actual[$index] -ne $Expected[$index]) {
            throw $Message
        }
    }
}

try {
    Write-PolicyBytes -Bytes $utf8WithoutBom.GetBytes("250`n")
    & $generator -PolicyPath $policyPath -OutputPath $outputPath | Out-Null
    & $generator -PolicyPath $policyPath -OutputPath $outputPath -Check | Out-Null
    $generated250 = [System.IO.File]::ReadAllBytes($outputPath)

    Write-PolicyBytes -Bytes $utf8WithoutBom.GetBytes("875`n")
    Assert-GeneratorRejected -Case "stale generated output" -CheckOnly
    Assert-EqualBytes `
        -Actual ([System.IO.File]::ReadAllBytes($outputPath)) `
        -Expected $generated250 `
        -Message "Generator check mode modified stale generated output."
    & $generator -PolicyPath $policyPath -OutputPath $outputPath | Out-Null
    & $generator -PolicyPath $policyPath -OutputPath $outputPath -Check | Out-Null
    $generated875 = $utf8WithoutBom.GetString([System.IO.File]::ReadAllBytes($outputPath))
    if (-not $generated875.Contains("milliseconds: 875,")) {
        throw "Generator did not deterministically publish the changed policy value."
    }

    Write-PolicyBytes -Bytes $utf8WithoutBom.GetBytes("9223372036854775`n")
    & $generator -PolicyPath $policyPath -OutputPath $outputPath | Out-Null
    & $generator -PolicyPath $policyPath -OutputPath $outputPath -Check | Out-Null
    $generatedMaximum = $utf8WithoutBom.GetString(
        [System.IO.File]::ReadAllBytes($outputPath)
    )
    if (-not $generatedMaximum.Contains("milliseconds: 9223372036854775,")) {
        throw "Generator rejected or changed the maximum Dart-safe millisecond policy."
    }

    $invalidCases = @(
        @{ Name = "empty"; Bytes = [byte[]]@() },
        @{ Name = "zero"; Bytes = $utf8WithoutBom.GetBytes("0`n") },
        @{ Name = "leading zero"; Bytes = $utf8WithoutBom.GetBytes("01`n") },
        @{ Name = "plus sign"; Bytes = $utf8WithoutBom.GetBytes("+1`n") },
        @{ Name = "minus sign"; Bytes = $utf8WithoutBom.GetBytes("-1`n") },
        @{ Name = "unit suffix"; Bytes = $utf8WithoutBom.GetBytes("1ms`n") },
        @{ Name = "leading whitespace"; Bytes = $utf8WithoutBom.GetBytes(" 1`n") },
        @{ Name = "trailing whitespace"; Bytes = $utf8WithoutBom.GetBytes("1 `n") },
        @{ Name = "CRLF"; Bytes = $utf8WithoutBom.GetBytes("1`r`n") },
        @{ Name = "multiple lines"; Bytes = $utf8WithoutBom.GetBytes("1`n2`n") },
        @{ Name = "missing LF"; Bytes = $utf8WithoutBom.GetBytes("1") },
        @{ Name = "UTF-8 BOM"; Bytes = [byte[]]@(0xEF, 0xBB, 0xBF, 0x31, 0x0A) },
        @{
            Name = "unsigned 64-bit overflow"
            Bytes = $utf8WithoutBom.GetBytes("18446744073709551616`n")
        },
        @{
            Name = "first unsafe Dart Duration millisecond value"
            Bytes = $utf8WithoutBom.GetBytes("9223372036854776`n")
        },
        @{
            Name = "unsigned 64-bit maximum exceeds Dart Duration"
            Bytes = $utf8WithoutBom.GetBytes("18446744073709551615`n")
        }
    )
    $sentinel = $utf8WithoutBom.GetBytes("sentinel`n")
    foreach ($invalidCase in $invalidCases) {
        Write-PolicyBytes -Bytes $invalidCase.Bytes
        [System.IO.File]::WriteAllBytes($outputPath, $sentinel)
        Assert-GeneratorRejected -Case "$($invalidCase.Name) in check mode" -CheckOnly
        Assert-EqualBytes `
            -Actual ([System.IO.File]::ReadAllBytes($outputPath)) `
            -Expected $sentinel `
            -Message "Invalid policy check case modified generated output: $($invalidCase.Name)"
        Assert-GeneratorRejected -Case $invalidCase.Name
        Assert-EqualBytes `
            -Actual ([System.IO.File]::ReadAllBytes($outputPath)) `
            -Expected $sentinel `
            -Message "Invalid policy case modified generated output: $($invalidCase.Name)"
    }

    Write-PolicyBytes -Bytes $utf8WithoutBom.GetBytes("250`n")
    & $generator -PolicyPath $policyPath -OutputPath $outputPath | Out-Null
    & $generator -PolicyPath $policyPath -OutputPath $outputPath -Check | Out-Null
    Write-Output "Library synchronization policy generator guardrails passed."
} finally {
    $resolvedScratchRoot = [System.IO.Path]::GetFullPath($scratchRoot)
    if ([System.IO.Path]::GetDirectoryName($resolvedScratchRoot) -ne $temporaryParent) {
        throw "Refusing to clean an unbound library synchronization policy scratch root."
    }
    if ([System.IO.Directory]::Exists($resolvedScratchRoot)) {
        [System.IO.Directory]::Delete($resolvedScratchRoot, $true)
    }
}
