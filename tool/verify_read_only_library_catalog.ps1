param(
    [Parameter(Mandatory = $true)]
    [string]$StorageRoot,
    [Parameter(Mandatory = $true)]
    [string]$RootA,
    [Parameter(Mandatory = $true)]
    [string]$RootB,
    [Parameter(Mandatory = $true)]
    [string]$AuthorizationToken
)

$ErrorActionPreference = "Stop"
$requiredToken = "CEDARFLAKE_AME_READ_ONLY_ACCEPTANCE_V1"
if ($AuthorizationToken -cne $requiredToken) {
    throw "The exact current read-only acceptance authorization token is required"
}

$resolvedStorage = [System.IO.Path]::GetFullPath($StorageRoot)
$resolvedRootA = [System.IO.Path]::GetFullPath($RootA)
$resolvedRootB = [System.IO.Path]::GetFullPath($RootB)
$catalogPath = Join-Path $resolvedStorage "catalog\ame.sqlite3"
if (-not (Test-Path -LiteralPath $catalogPath -PathType Leaf)) {
    throw "The retained acceptance catalog is missing: $catalogPath"
}
foreach ($root in @($resolvedRootA, $resolvedRootB)) {
    if (-not (Test-Path -LiteralPath $root -PathType Container)) {
        throw "An expected library root is unavailable: $root"
    }
}

$environment = @{
    CEDARFLAKE_AME_ACCEPTANCE_CONSENT = $requiredToken
    CEDARFLAKE_AME_ACCEPTANCE_STORAGE_ROOT = $resolvedStorage
    CEDARFLAKE_AME_TEST_STORAGE_ROOT = $resolvedStorage
    CEDARFLAKE_AME_COMBINED_ROOT_A = $resolvedRootA
    CEDARFLAKE_AME_COMBINED_ROOT_B = $resolvedRootB
}
$previousEnvironment = @{}
foreach ($name in $environment.Keys) {
    $previousEnvironment[$name] = [System.Environment]::GetEnvironmentVariable($name, "Process")
    [System.Environment]::SetEnvironmentVariable($name, $environment[$name], "Process")
}

try {
    $repositoryRoot = Split-Path -Parent $PSScriptRoot
    $cargo = (Get-Command cargo -ErrorAction Stop).Source
    & $cargo test --manifest-path (Join-Path $repositoryRoot "rust\Cargo.toml") `
        user_authorized_combined_catalog_load_acceptance `
        -- --ignored --nocapture --test-threads=1
    if ($LASTEXITCODE -ne 0) {
        throw "Combined catalog acceptance failed with exit code $LASTEXITCODE"
    }
} finally {
    foreach ($name in $environment.Keys) {
        [System.Environment]::SetEnvironmentVariable(
            $name,
            $previousEnvironment[$name],
            "Process"
        )
    }
}
