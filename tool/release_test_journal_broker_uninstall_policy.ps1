$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "release_journal_broker_common.ps1")

function Assert-UninstallPolicyRejected {
    param([scriptblock]$Action)

    $rejected = $false
    try {
        & $Action | Out-Null
    } catch {
        $rejected = $true
    }
    if (-not $rejected) {
        throw "The uninstall policy accepted an invalid removal intent"
    }
}

$parent = [pscustomobject]@{
    Path = Get-AmeProductParentDirectory
    Existed = $false
    Sddl = $null
}
$defaultState = New-AmeBrokerTransactionState -Operation uninstall -ParentPrestate $parent
Assert-AmeBrokerTransactionStateFacts -State $defaultState
if (Test-AmeBrokerUninstallApplicationRemoval -State $defaultState) {
    throw "New uninstall transactions must preserve the Application bundle by default"
}
foreach ($value in @($null, "false", 0, 1)) {
    $invalid = $defaultState | ConvertTo-Json -Depth 8 | ConvertFrom-Json
    $invalid.RemoveApplicationBundle = $value
    Assert-UninstallPolicyRejected { Assert-AmeBrokerTransactionStateFacts -State $invalid }
}
$invalid = $defaultState | ConvertTo-Json -Depth 8 | ConvertFrom-Json
$invalid.PSObject.Properties.Remove("RemoveApplicationBundle")
Assert-UninstallPolicyRejected { Assert-AmeBrokerTransactionStateFacts -State $invalid }
$invalid = $defaultState | ConvertTo-Json -Depth 8 | ConvertFrom-Json
$invalid.OwnedDirectories = @(Get-AmeApplicationInstallDirectory)
Assert-UninstallPolicyRejected { Assert-AmeBrokerTransactionStateFacts -State $invalid }

function Test-UninstallApplicationPolicyRecovery {
    param(
        [Parameter(Mandatory = $true)]
        [bool]$RemoveApplication,
        [switch]$Legacy
    )

    $state = New-AmeBrokerTransactionState `
        -Operation uninstall `
        -ParentPrestate $parent `
        -ExpectedPublisher "CN=Cedarflake Test Publisher"
    $state.RemoveApplicationBundle = $RemoveApplication
    if ($Legacy) {
        $state.Schema = 2
        $state.PSObject.Properties.Remove("RemoveApplicationBundle")
    }
    Assert-AmeBrokerTransactionStateFacts -State $state
    $physical = [pscustomobject]@{ ApplicationExists = $true; ApplicationAccesses = 0 }
    $marker = [pscustomobject]@{ Exists = $true; Last = $state }
    # This synchronous callback needs the validators in the enclosing script scope.
    $writer = {
        param([psobject]$Value)
        Assert-AmeBrokerTransactionStateFacts -State $Value
        Assert-AmeBrokerUninstallProgressFacts -State $Value
        $marker.Last = $Value | ConvertTo-Json -Depth 8 | ConvertFrom-Json
    }
    $reader = { $marker.Last }.GetNewClosure()
    $removeMarker = { $marker.Exists = $false }.GetNewClosure()
    $actions = @{}
    foreach ($step in Get-AmeBrokerUninstallStepDefinitions) {
        $actions[[string]$step.Property] = [pscustomobject]@{
            MutationAction = {}
            VerifyAction = {}
        }
    }
    $actions.UninstallApplicationRemoved = [pscustomobject]@{
        MutationAction = {
            $physical.ApplicationAccesses += 1
            if (-not $RemoveApplication) {
                throw "Broker-only uninstall accessed the Application tree"
            }
            $physical.ApplicationExists = $false
        }.GetNewClosure()
        VerifyAction = {
            $physical.ApplicationAccesses += 1
            if (-not $RemoveApplication -or $physical.ApplicationExists) {
                throw "Application removal postcondition used the wrong policy"
            }
        }.GetNewClosure()
    }
    $interrupted = $false
    try {
        Complete-AmeBrokerUninstallTransaction `
            -State $state `
            -StateWriter $writer `
            -MarkerRemovalAction $removeMarker `
            -ActionOverrides $actions `
            -FaultOperation UninstallApplicationRemoved `
            -FaultBoundary AfterMutation `
            -FaultAction { throw "injected application-policy recovery boundary" }
    } catch {
        if ($_.Exception.Message -cne "injected application-policy recovery boundary") {
            throw
        }
        $interrupted = $true
    }
    if (-not $interrupted -or -not $marker.Exists -or
        [string]$marker.Last.UninstallPendingOperation -cne "UninstallApplicationRemoved") {
        throw "The application policy boundary lost its recoverable prefix"
    }
    if ((Test-AmeBrokerUninstallApplicationRemoval -State $marker.Last) -ne $RemoveApplication) {
        throw "The persisted application removal intent changed after interruption"
    }
    Repair-AmeBrokerInterruptedTransaction `
        -ProcessProbe {
            param([int]$ProcessId)
            [pscustomobject]@{ Exists = $false; StartUtcTicks = [int64]0 }
        } `
        -StateReader $reader `
        -UninstallStateWriter $writer `
        -UninstallMarkerRemovalAction $removeMarker `
        -UninstallActionOverrides $actions
    if ($marker.Exists -or [string]$marker.Last.Phase -cne "final_cleanup_verified" -or
        $physical.ApplicationExists -eq $RemoveApplication) {
        throw "Recovered uninstall did not preserve its selected application outcome"
    }
    if (-not $RemoveApplication -and $physical.ApplicationAccesses -ne 0) {
        throw "Broker-only recovery probed or removed the Application tree"
    }
    $invalidPrefix = $marker.Last | ConvertTo-Json -Depth 8 | ConvertFrom-Json
    $invalidPrefix.UninstallApplicationRemoved = $false
    Assert-UninstallPolicyRejected { Assert-AmeBrokerUninstallProgressFacts -State $invalidPrefix }
}

Test-UninstallApplicationPolicyRecovery -RemoveApplication $false
Test-UninstallApplicationPolicyRecovery -RemoveApplication $true
Test-UninstallApplicationPolicyRecovery -RemoveApplication $true -Legacy

Write-Output "journal_broker_uninstall_policy_guardrails_passed"
