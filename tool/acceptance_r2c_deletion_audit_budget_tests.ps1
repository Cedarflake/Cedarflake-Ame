# Internal node-budget regression owner, invoked by the R2c-R guardrail entrypoint.
function Test-AmeR2cRDeletionAuditNodeBudgets {
    $knownEmptyAstNodeCount = [uint64]2
    $emptyNodeState = New-AmeR2cRDeletionAuditState `
        -BudgetOverridesForGuardrail @{ "ast-nodes" = $knownEmptyAstNodeCount }
    try {
        Add-AmeR2cRDeletionAuditSource `
            -State $emptyNodeState `
            -SourceText "" `
            -Label "known empty AST" `
            -SourcePath $null `
            -IsRoot $true `
            -Depth 0 | Out-Null
        if ($emptyNodeState.BudgetActual["ast-nodes"] -ne $knownEmptyAstNodeCount -or
            $emptyNodeState.BudgetVisitedHighWater["ast-nodes"] -ne $knownEmptyAstNodeCount) {
            throw "R2c-R empty AST node count did not match the independent known assertion"
        }
    } finally { Close-AmeR2cRDeletionAuditState -State $emptyNodeState }

    $knownLiteralAstNodeCount = [uint64]5
    foreach ($fixture in @(
        [pscustomobject]@{ Label = "limit-1"; Limit = [uint64]6; MustPass = $true },
        [pscustomobject]@{ Label = "limit"; Limit = [uint64]5; MustPass = $true },
        [pscustomobject]@{ Label = "limit+1"; Limit = [uint64]4; MustPass = $false }
    )) {
        $nodeState = New-AmeR2cRDeletionAuditState `
            -BudgetOverridesForGuardrail @{ "ast-nodes" = [uint64]$fixture.Limit }
        try {
            if ($fixture.MustPass) {
                Add-AmeR2cRDeletionAuditSource `
                    -State $nodeState `
                    -SourceText "1" `
                    -Label "AST $($fixture.Label)" `
                    -SourcePath $null `
                    -IsRoot $true `
                    -Depth 0 | Out-Null
                if ($nodeState.BudgetActual["ast-nodes"] -ne $knownLiteralAstNodeCount) {
                    throw "R2c-R AST $($fixture.Label) fixture recorded the wrong node count"
                }
            } else {
                try {
                    Add-AmeR2cRDeletionAuditSource `
                        -State $nodeState `
                        -SourceText "1" `
                        -Label "AST $($fixture.Label)" `
                        -SourcePath $null `
                        -IsRoot $true `
                        -Depth 0 | Out-Null
                    throw "R2c-R AST node budget accepted limit+1"
                } catch {
                    Assert-Contains $_.Exception.Message "budget=ast-nodes limit=4 actual=5"
                }
            }
            if ($nodeState.BudgetVisitedHighWater["ast-nodes"] -ne $knownLiteralAstNodeCount) {
                throw "R2c-R AST $($fixture.Label) fixture lost its visited high-water"
            }
        } finally { Close-AmeR2cRDeletionAuditState -State $nodeState }
    }

    $cumulativeState = New-AmeR2cRDeletionAuditState `
        -BudgetOverridesForGuardrail @{ "ast-nodes" = [uint64]7 }
    try {
        foreach ($source in @(
            [pscustomobject]@{ Text = "# first"; IsRoot = $true },
            [pscustomobject]@{ Text = "1"; IsRoot = $false }
        )) {
            Add-AmeR2cRDeletionAuditSource `
                -State $cumulativeState `
                -SourceText $source.Text `
                -Label $source.Text `
                -SourcePath $null `
                -IsRoot $source.IsRoot `
                -Depth 0 | Out-Null
        }
        if ($cumulativeState.BudgetActual["ast-nodes"] -ne 7 -or
            $cumulativeState.BudgetVisitedHighWater["ast-nodes"] -ne 7) {
            throw "R2c-R AST budget did not accumulate across sources"
        }
        try {
            Add-AmeR2cRDeletionAuditSource `
                -State $cumulativeState `
                -SourceText ("2; " * 10000) `
                -Label "bounded unvisited tail" `
                -SourcePath $null `
                -IsRoot $false `
                -Depth 0 | Out-Null
            throw "R2c-R cumulative AST budget accepted an oversized tail"
        } catch {
            Assert-Contains $_.Exception.Message "budget=ast-nodes limit=7 actual=8"
        }
        if ($cumulativeState.BudgetActual["ast-nodes"] -ne 7 -or
            $cumulativeState.BudgetVisitedHighWater["ast-nodes"] -ne 8) {
            throw "R2c-R AST budget continued beyond the first rejected node"
        }
    } finally { Close-AmeR2cRDeletionAuditState -State $cumulativeState }
}
