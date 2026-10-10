[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$LocalRoot,
    [Parameter(Mandatory = $true)]
    [string]$CloudRoot,
    [Parameter(Mandatory = $true)]
    [string]$SourceCatalogPath,
    [Parameter(Mandatory = $true)]
    [string]$StorageRoot,
    [Parameter(Mandatory = $true)]
    [string]$AuthorizationToken,
    [switch]$AcknowledgeCloudReadOnly,
    [ValidateRange(60, 3600)]
    [int]$TimeLimitSeconds = 1800,
    [ValidateRange(268435456, 4294967296)]
    [UInt64]$MemoryLimitBytes = 2147483648,
    [switch]$ValidationOnly
)

$ErrorActionPreference = "Stop"

& "$PSScriptRoot\acceptance_run_r2c_reliability.ps1" `
    @PSBoundParameters `
    -AcceptanceProfile Replacement
