function Get-AmeR2cRTextSha256 {
    param([Parameter(Mandatory = $true)][AllowEmptyString()][string]$Text)
    $algorithm = [Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [Text.Encoding]::UTF8.GetBytes($Text.Replace("`r`n", "`n"))
        return ([BitConverter]::ToString($algorithm.ComputeHash($bytes)) -replace '-', '').ToLowerInvariant()
    } finally { $algorithm.Dispose() }
}

function Add-AmeR2cRDeletionAuditBudgetValue {
    param(
        [Parameter(Mandatory = $true)][string]$Key,
        [Parameter(Mandatory = $true)][uint64]$Current,
        [Parameter(Mandatory = $true)][uint64]$Delta,
        [Parameter(Mandatory = $true)][uint64]$Limit
    )
    if ($Delta -gt ([uint64]::MaxValue - $Current)) {
        throw "R2c-R deletion safety budget exceeded; budget=$Key limit=$Limit actual=overflow"
    }
    $actual = [uint64]($Current + $Delta)
    if ($actual -gt $Limit) {
        throw "R2c-R deletion safety budget exceeded; budget=$Key limit=$Limit actual=$actual"
    }
    return $actual
}

function Assert-AmeR2cRDeletionAuditBudgetValue {
    param(
        [Parameter(Mandatory = $true)][psobject]$State,
        [Parameter(Mandatory = $true)][string]$Key,
        [Parameter(Mandatory = $true)][uint64]$Current,
        [Parameter(Mandatory = $true)][uint64]$Delta
    )
    if (-not $State.BudgetLimits.Contains($Key)) {
        throw "R2c-R deletion safety budget key is unknown: $Key"
    }
    return Add-AmeR2cRDeletionAuditBudgetValue `
        -Key $Key `
        -Current $Current `
        -Delta $Delta `
        -Limit ([uint64]$State.BudgetLimits[$Key])
}

function New-AmeR2cRDeletionAuditState {
    param([AllowNull()][hashtable]$BudgetOverridesForGuardrail)
    $limits = [ordered]@{}
    foreach ($key in $script:AmeR2cRDeletionAuditBudgetLimits.Keys) {
        $limits[$key] = [uint64]$script:AmeR2cRDeletionAuditBudgetLimits[$key]
    }
    if ($null -ne $BudgetOverridesForGuardrail) {
        foreach ($key in $BudgetOverridesForGuardrail.Keys) {
            if (-not $limits.Contains($key)) {
                throw "R2c-R deletion safety budget override key is unknown: $key"
            }
            $limits[$key] = [uint64]$BudgetOverridesForGuardrail[$key]
        }
    }
    return [pscustomobject]@{
        AuditedScopes = [Collections.Generic.HashSet[string]]::new(
            [StringComparer]::Ordinal
        )
        BudgetActual = [ordered]@{
            "source-count" = [uint64]0
            "total-bytes" = [uint64]0
            "ast-nodes" = [uint64]0
            "function-count" = [uint64]0
            "scope-count" = [uint64]0
            "queue-count" = [uint64]0
        }
        BudgetVisitedHighWater = [ordered]@{
            "ast-nodes" = [uint64]0
        }
        BudgetLimits = $limits
        Functions = @{}
        HeldStreams = [Collections.Generic.List[IDisposable]]::new()
        Queue = [Collections.Generic.Queue[object]]::new()
        SourceByIdentity = [Collections.Generic.Dictionary[string, object]]::new(
            [StringComparer]::Ordinal
        )
        SourceByKey = [Collections.Generic.Dictionary[string, object]]::new(
            [StringComparer]::Ordinal
        )
        SourceByPath = [Collections.Generic.Dictionary[string, object]]::new(
            [StringComparer]::Ordinal
        )
        Sources = [Collections.Generic.List[object]]::new()
        ToolRoot = [IO.Path]::GetFullPath($PSScriptRoot).TrimEnd('\')
    }
}

function Close-AmeR2cRDeletionAuditState {
    param([Parameter(Mandatory = $true)][psobject]$State)
    for ($index = $State.HeldStreams.Count - 1; $index -ge 0; $index -= 1) {
        $State.HeldStreams[$index].Dispose()
    }
    $State.HeldStreams.Clear()
}

function Assert-AmeR2cRDeletionAuditSnapshotsHeld {
    param([Parameter(Mandatory = $true)][psobject]$State)
    foreach ($source in $State.Sources) {
        if ($null -ne $source.Snapshot) {
            $source.Snapshot.Revalidate()
        }
    }
}

function Read-AmeR2cRAuditedScriptSnapshot {
    param(
        [Parameter(Mandatory = $true)][psobject]$State,
        [Parameter(Mandatory = $true)][string]$Path
    )
    $fullPath = [IO.Path]::GetFullPath($Path)
    if ([IO.Path]::GetExtension($fullPath) -cne ".ps1") {
        throw "R2c-R deletion safety audit rejected a source without the exact .ps1 extension"
    }
    if ($null -eq ("AmeR2cRAuditedScriptSnapshot" -as [type])) {
        throw "R2c-R native audited-script identity types are not initialized"
    }
    $snapshot = [AmeR2cRAuditedScriptSnapshot]::Open($State.ToolRoot, $fullPath)
    try {
        $byteLength = [uint64]$snapshot.Length
        Assert-AmeR2cRDeletionAuditBudgetValue `
            -State $State `
            -Key "per-source-bytes" `
            -Current 0 `
            -Delta $byteLength | Out-Null
        $text = $snapshot.ReadUtf8Text()
        $result = [pscustomobject]@{
            ByteLength = $byteLength
            Digest = Get-AmeR2cRTextSha256 -Text $text
            Identity = [string]$snapshot.IdentityToken
            NativeSnapshot = $snapshot
            Path = $fullPath
            Text = $text
        }
        $snapshot = $null
        return $result
    } finally {
        if ($null -ne $snapshot) { $snapshot.Dispose() }
    }
}

function Get-AmeR2cRAstOwningFunction {
    param([Parameter(Mandatory = $true)][Management.Automation.Language.Ast]$Node)
    $current = $Node.Parent
    while ($null -ne $current) {
        if ($current -is [Management.Automation.Language.FunctionDefinitionAst]) {
            return $current
        }
        $current = $current.Parent
    }
    return $null
}

function Resolve-AmeR2cRAuditDotSourcePath {
    param(
        [Parameter(Mandatory = $true)]
        [Management.Automation.Language.CommandAst]$Command,
        [AllowNull()][string]$SourcePath
    )
    $target = $Command.CommandElements[0]
    $candidate = $null
    if ($target -is [Management.Automation.Language.StringConstantExpressionAst]) {
        $candidate = [string]$target.Value
    } elseif ($target -is [Management.Automation.Language.ParenExpressionAst] -and
        $null -ne $SourcePath) {
        $pipeline = $target.Pipeline
        if ($pipeline.PipelineElements.Count -eq 1 -and
            $pipeline.PipelineElements[0] -is [Management.Automation.Language.CommandAst]) {
            $join = $pipeline.PipelineElements[0]
            if ($join.GetCommandName() -ceq "Join-Path" -and
                $join.CommandElements.Count -eq 3 -and
                $join.CommandElements[1] -is [Management.Automation.Language.VariableExpressionAst] -and
                $join.CommandElements[1].VariablePath.UserPath -ceq "PSScriptRoot" -and
                $join.CommandElements[2] -is [Management.Automation.Language.StringConstantExpressionAst]) {
                $leaf = [string]$join.CommandElements[2].Value
                if ([IO.Path]::GetFileName($leaf) -cne $leaf) {
                    throw "R2c-R deletion safety audit rejected non-leaf PSScriptRoot loader"
                }
                $candidate = Join-Path ([IO.Path]::GetDirectoryName($SourcePath)) $leaf
            }
        }
    }
    if ([string]::IsNullOrEmpty($candidate)) {
        throw "R2c-R deletion safety audit rejected unresolved dot-source"
    }
    if (-not [IO.Path]::IsPathRooted($candidate)) {
        if ($null -eq $SourcePath) {
            throw "R2c-R deletion safety audit rejected unresolved relative dot-source"
        }
        $candidate = Join-Path ([IO.Path]::GetDirectoryName($SourcePath)) $candidate
    }
    return [IO.Path]::GetFullPath($candidate)
}

function Add-AmeR2cRDeletionAuditAstNodesBounded {
    param(
        [Parameter(Mandatory = $true)][psobject]$State,
        [Parameter(Mandatory = $true)][Management.Automation.Language.Ast]$Ast
    )
    $counter = [pscustomobject]@{
        Current = [uint64]$State.BudgetActual["ast-nodes"]
    }
    $limit = [uint64]$State.BudgetLimits["ast-nodes"]
    $null = $Ast.FindAll({
        param($node)
        $actual = [uint64]($counter.Current + 1)
        if ($actual -gt [uint64]$State.BudgetVisitedHighWater["ast-nodes"]) {
            $State.BudgetVisitedHighWater["ast-nodes"] = $actual
        }
        if ($actual -gt $limit) {
            Assert-AmeR2cRDeletionAuditBudgetValue `
                -State $State `
                -Key "ast-nodes" `
                -Current ([uint64]$counter.Current) `
                -Delta 1 | Out-Null
        }
        $counter.Current = $actual
        $State.BudgetActual["ast-nodes"] = [uint64]$counter.Current
        return $false
    }, $true)
}

function Add-AmeR2cRDeletionAuditSource {
    param(
        [Parameter(Mandatory = $true)][psobject]$State,
        [AllowNull()][object]$SourceText,
        [Parameter(Mandatory = $true)][string]$Label,
        [AllowNull()][string]$SourcePath,
        [Parameter(Mandatory = $true)][bool]$IsRoot,
        [Parameter(Mandatory = $true)][uint64]$Depth
    )
    if ($null -ne $SourceText -and $SourceText -isnot [string]) {
        throw "R2c-R deletion safety audit source text must be a string"
    }
    Assert-AmeR2cRDeletionAuditBudgetValue `
        -State $State `
        -Key "dot-source-depth" `
        -Current 0 `
        -Delta $Depth | Out-Null
    $newSourceCount = Assert-AmeR2cRDeletionAuditBudgetValue `
        -State $State `
        -Key "source-count" `
        -Current ([uint64]$State.BudgetActual["source-count"]) `
        -Delta 1
    $State.BudgetActual["source-count"] = $newSourceCount
    $snapshot = $null
    $nativeSnapshot = $null
    $pathKey = $null
    $identity = $null
    try {
        if (-not [string]::IsNullOrEmpty($SourcePath)) {
            $snapshot = Read-AmeR2cRAuditedScriptSnapshot -State $State -Path $SourcePath
            $nativeSnapshot = $snapshot.NativeSnapshot
            if ($null -ne $SourceText -and $SourceText -cne $snapshot.Text) {
                throw "R2c-R deletion safety audit source text changed before identity binding"
            }
            $SourceText = [string]$snapshot.Text
            $SourcePath = [string]$snapshot.Path
            $pathKey = $SourcePath
            $byteLength = [uint64]$snapshot.ByteLength
            $sourceDigest = [string]$snapshot.Digest
            $identity = [string]$snapshot.Identity
            if ($State.SourceByIdentity.ContainsKey($identity)) {
                $existing = $State.SourceByIdentity[$identity]
                if ($existing.Digest -cne $sourceDigest -or $existing.Text -cne $SourceText) {
                    throw "R2c-R deletion safety audit held identity produced conflicting source bytes"
                }
                $State.SourceByPath[$pathKey] = $existing
                return $existing
            }
            $key = "file:${identity}:$sourceDigest"
            $newTotal = Assert-AmeR2cRDeletionAuditBudgetValue `
                -State $State `
                -Key "total-bytes" `
                -Current ([uint64]$State.BudgetActual["total-bytes"]) `
                -Delta $byteLength
            $State.BudgetActual["total-bytes"] = $newTotal
        } else {
            if ($null -eq $SourceText) {
                throw "R2c-R deletion safety audit received no source text"
            }
            $byteLength = [uint64][Text.Encoding]::UTF8.GetByteCount($SourceText)
            Assert-AmeR2cRDeletionAuditBudgetValue `
                -State $State `
                -Key "per-source-bytes" `
                -Current 0 `
                -Delta $byteLength | Out-Null
            $newTotal = Assert-AmeR2cRDeletionAuditBudgetValue `
                -State $State `
                -Key "total-bytes" `
                -Current ([uint64]$State.BudgetActual["total-bytes"]) `
                -Delta $byteLength
            $State.BudgetActual["total-bytes"] = $newTotal
            $sourceDigest = Get-AmeR2cRTextSha256 -Text $SourceText
            $key = "memory:${Label}:$sourceDigest"
        }
        if ($State.SourceByKey.ContainsKey($key)) {
            return $State.SourceByKey[$key]
        }
        $tokens = $null
        $parseErrors = $null
        $ast = [Management.Automation.Language.Parser]::ParseInput(
            $SourceText,
            [ref]$tokens,
            [ref]$parseErrors
        )
        if ($parseErrors.Count -ne 0) {
            throw "R2c-R deletion safety audit could not parse '$Label'"
        }
        Add-AmeR2cRDeletionAuditAstNodesBounded -State $State -Ast $ast
        $functions = @($ast.FindAll({
            param($node)
            $node -is [Management.Automation.Language.FunctionDefinitionAst]
        }, $true))
        $newFunctionCount = Assert-AmeR2cRDeletionAuditBudgetValue `
            -State $State `
            -Key "function-count" `
            -Current ([uint64]$State.BudgetActual["function-count"]) `
            -Delta ([uint64]$functions.Count)
        $State.BudgetActual["function-count"] = $newFunctionCount
        $record = [pscustomobject]@{
            Ast = $ast
            ByteLength = $byteLength
            Depth = $Depth
            Digest = $sourceDigest
            IsRoot = $IsRoot
            Identity = $identity
            Key = $key
            Label = $Label
            Path = $SourcePath
            Snapshot = $nativeSnapshot
            Text = $SourceText
        }
        if ($null -ne $nativeSnapshot) {
            $State.HeldStreams.Add($nativeSnapshot)
            $nativeSnapshot = $null
        }
        $State.SourceByKey[$key] = $record
        if ($null -ne $identity) { $State.SourceByIdentity[$identity] = $record }
        if ($null -ne $pathKey) { $State.SourceByPath[$pathKey] = $record }
        $State.Sources.Add($record)
        foreach ($function in $functions) {
            if (-not $State.Functions.ContainsKey($function.Name)) {
                $State.Functions[$function.Name] = [Collections.Generic.List[object]]::new()
            }
            $State.Functions[$function.Name].Add([pscustomobject]@{
                Ast = $function
                Record = $record
            })
        }
        return $record
    } finally {
        if ($null -ne $nativeSnapshot) { $nativeSnapshot.Dispose() }
    }
}

function Add-AmeR2cRDeletionAuditScope {
    param(
        [Parameter(Mandatory = $true)][psobject]$State,
        [Parameter(Mandatory = $true)][psobject]$Record,
        [AllowNull()][Management.Automation.Language.FunctionDefinitionAst]$Function
    )
    $scopeKey = if ($null -eq $Function) {
        "$($Record.Key):top"
    } else {
        "$($Record.Key):function:$($Function.Name):$($Function.Extent.StartOffset)"
    }
    if (-not $State.AuditedScopes.Contains($scopeKey)) {
        $newScopeCount = Assert-AmeR2cRDeletionAuditBudgetValue `
            -State $State `
            -Key "scope-count" `
            -Current ([uint64]$State.BudgetActual["scope-count"]) `
            -Delta 1
        $newQueueCount = Assert-AmeR2cRDeletionAuditBudgetValue `
            -State $State `
            -Key "queue-count" `
            -Current ([uint64]$State.Queue.Count) `
            -Delta 1
        $State.AuditedScopes.Add($scopeKey) | Out-Null
        $State.Queue.Enqueue([pscustomobject]@{
            Function = $Function
            Key = $scopeKey
            Record = $Record
        })
        $State.BudgetActual["scope-count"] = $newScopeCount
        if ($newQueueCount -gt [uint64]$State.BudgetActual["queue-count"]) {
            $State.BudgetActual["queue-count"] = $newQueueCount
        }
    }
}

function Assert-AmeR2cRProcessBoundaryContract {
    param([Parameter(Mandatory = $true)][psobject]$State)
    if (-not $State.Functions.ContainsKey("Invoke-AmeR2cROwnedProcess")) { return }
    Assert-AmeR2cRCargoBoundaryContract -State $State
    $matches = @($State.Functions["Invoke-AmeR2cROwnedProcess"])
    if ($matches.Count -ne 1) {
        throw "R2c-R process execution boundary was ambiguous"
    }
    $actual = Get-AmeR2cRTextSha256 -Text $matches[0].Ast.Extent.Text
    if ($actual -cne $script:AmeR2cRVerifiedProcessBoundaryDigest) {
        throw (
            "R2c-R process boundary item Invoke-AmeR2cROwnedProcess changed; " +
            "expected=$script:AmeR2cRVerifiedProcessBoundaryDigest actual=$actual"
        )
    }
    $nativeDefinitions = @(
        foreach ($record in $State.Sources) {
            $assignments = @($record.Ast.FindAll({
                param($node)
                    $node -is [Management.Automation.Language.AssignmentStatementAst] -and
                    $node.Left -is [Management.Automation.Language.VariableExpressionAst] -and
                    $node.Left.VariablePath.UserPath -ceq "nativeTypeDefinition" -and
                    $node.Right -is [Management.Automation.Language.CommandExpressionAst] -and
                    $node.Right.Expression -is
                        [Management.Automation.Language.StringConstantExpressionAst] -and
                    $node.Right.Expression.Value.IndexOf(
                        "public sealed class AmeR2cRProcessJob",
                        [StringComparison]::Ordinal
                    ) -ge 0
            }, $true))
            foreach ($assignment in $assignments) {
                [string]$assignment.Right.Expression.Value
            }
        }
    )
    if ($nativeDefinitions.Count -ne 1) {
        throw "R2c-R native execution definition was missing or ambiguous"
    }
    $definitionActual = Get-AmeR2cRTextSha256 -Text $nativeDefinitions[0]
    if ($definitionActual -cne $script:AmeR2cRNativeDefinitionDigest) {
        throw (
            "R2c-R native boundary item nativeTypeDefinition changed; " +
            "expected=$script:AmeR2cRNativeDefinitionDigest actual=$definitionActual"
        )
    }
    $snapshotClasses = @(
        foreach ($value in $nativeDefinitions) {
            $snapshotStart = $value.IndexOf(
                "public sealed class AmeR2cRAuditedScriptSnapshot",
                [StringComparison]::Ordinal
            )
            $processStart = $value.IndexOf(
                "public sealed class AmeR2cRProcessJob",
                [StringComparison]::Ordinal
            )
            if ($snapshotStart -lt 0 -or $processStart -le $snapshotStart) {
                throw "R2c-R native audited-script identity boundary was missing"
            }
            $value.Substring($snapshotStart, $processStart - $snapshotStart)
        }
    )
    if ($snapshotClasses.Count -ne 1) {
        throw "R2c-R native audited-script identity boundary was ambiguous"
    }
    $snapshotActual = Get-AmeR2cRTextSha256 -Text $snapshotClasses[0]
    if ($snapshotActual -cne $script:AmeR2cRAuditedScriptSnapshotDigest) {
        throw (
            "R2c-R native boundary item AmeR2cRAuditedScriptSnapshot changed; " +
            "expected=$script:AmeR2cRAuditedScriptSnapshotDigest actual=$snapshotActual"
        )
    }
    $nativeClasses = @(
        foreach ($value in $nativeDefinitions) {
            $start = $value.IndexOf(
                "public sealed class AmeR2cRProcessJob",
                [StringComparison]::Ordinal
            )
            $value.Substring($start)
        }
    )
    if ($nativeClasses.Count -ne 1) {
        throw "R2c-R native process execution boundary was missing or ambiguous"
    }
    $nativeActual = Get-AmeR2cRTextSha256 -Text $nativeClasses[0]
    if ($nativeActual -cne $script:AmeR2cRNativeProcessBoundaryDigest) {
        throw (
            "R2c-R native process boundary item AmeR2cRProcessJob changed; " +
            "expected=$script:AmeR2cRNativeProcessBoundaryDigest actual=$nativeActual"
        )
    }
}

function Add-AmeR2cRDeletionAuditModuleScope {
    param(
        [Parameter(Mandatory = $true)][psobject]$State,
        [Parameter(Mandatory = $true)][psobject]$Record
    )
    if ($State.AuditedScopes.Contains("$($Record.Key):top")) { return }
    Add-AmeR2cRDeletionAuditScope -State $State -Record $Record -Function $null
    # Resolve static module imports before calls to functions owned by their dependencies.
    $imports = @($Record.Ast.FindAll({
        param($node)
        $node -is [Management.Automation.Language.CommandAst] -and
            $node.InvocationOperator -eq [Management.Automation.Language.TokenKind]::Dot -and
            $null -eq (Get-AmeR2cRAstOwningFunction -Node $node)
    }, $true))
    foreach ($command in $imports) {
        $depth = Assert-AmeR2cRDeletionAuditBudgetValue `
            -State $State `
            -Key "dot-source-depth" `
            -Current ([uint64]$Record.Depth) `
            -Delta 1
        $path = Resolve-AmeR2cRAuditDotSourcePath -Command $command -SourcePath $Record.Path
        $dependency = Add-AmeR2cRDeletionAuditSource `
            -State $State `
            -SourceText $null `
            -Label "dot-source:$path" `
            -SourcePath $path `
            -IsRoot $false `
            -Depth $depth
        Add-AmeR2cRDeletionAuditModuleScope -State $State -Record $dependency
    }
}

function Invoke-AmeR2cRDeletionAuditClosure {
    param(
        [Parameter(Mandatory = $true)][psobject]$State,
        [Parameter(Mandatory = $true)][psobject]$Root
    )
    Add-AmeR2cRDeletionAuditModuleScope -State $State -Record $Root
    foreach ($function in @($Root.Ast.FindAll({
        param($node)
        $node -is [Management.Automation.Language.FunctionDefinitionAst]
    }, $true))) {
        Add-AmeR2cRDeletionAuditScope -State $State -Record $Root -Function $function
    }
    while ($State.Queue.Count -gt 0) {
        $scope = $State.Queue.Dequeue()
        $ownerName = if ($null -eq $scope.Function) { "<top>" } else { $scope.Function.Name }
        $scopeAst = if ($null -eq $scope.Function) { $scope.Record.Ast } else { $scope.Function.Body }
        $commands = @($scopeAst.FindAll({
            param($node)
            if ($node -isnot [Management.Automation.Language.CommandAst]) { return $false }
            if ($null -ne $scope.Function) { return $true }
            return $null -eq (Get-AmeR2cRAstOwningFunction -Node $node)
        }, $true))
        foreach ($command in $commands) {
            if ($command.InvocationOperator -eq [Management.Automation.Language.TokenKind]::Dot) {
                if ($null -eq $scope.Function) { continue }
                $dependencyDepth = Assert-AmeR2cRDeletionAuditBudgetValue `
                    -State $State `
                    -Key "dot-source-depth" `
                    -Current ([uint64]$scope.Record.Depth) `
                    -Delta 1
                $dotPath = Resolve-AmeR2cRAuditDotSourcePath `
                    -Command $command `
                    -SourcePath $scope.Record.Path
                $dependency = Add-AmeR2cRDeletionAuditSource `
                    -State $State `
                    -SourceText $null `
                    -Label "dot-source:$dotPath" `
                    -SourcePath $dotPath `
                    -IsRoot $false `
                    -Depth $dependencyDepth
                Add-AmeR2cRDeletionAuditModuleScope `
                    -State $State `
                    -Record $dependency
                continue
            }
            $commandName = $command.GetCommandName()
            if ($command.InvocationOperator -ne
                    [Management.Automation.Language.TokenKind]::Unknown -or
                [string]::IsNullOrEmpty($commandName)) {
                throw (
                    "R2c-R deletion safety audit rejected unresolved command invocation " +
                    "in '$($scope.Record.Label)' scope '$ownerName'"
                )
            }
            $leafName = [string]$commandName
            $moduleName = $null
            $separatorIndex = $leafName.LastIndexOf('\', [StringComparison]::Ordinal)
            if ($separatorIndex -ge 0) {
                $moduleName = $leafName.Substring(0, $separatorIndex)
                $leafName = $leafName.Substring($separatorIndex + 1)
            }
            if ($leafName -iin @(
                "Remove-Item", "ri", "rm", "del", "erase", "rmdir", "rd",
                "Set-Alias", "sal", "New-Alias", "nal", "Import-Alias", "ipal",
                "Invoke-Expression", "iex", "Invoke-Command"
            )) {
                throw "R2c-R deletion safety audit rejected command '$leafName'"
            }
            if ($State.Functions.ContainsKey($leafName)) {
                $matches = @($State.Functions[$leafName])
                if ($matches.Count -ne 1) {
                    throw "R2c-R deletion safety audit rejected ambiguous helper '$leafName'"
                }
                Add-AmeR2cRDeletionAuditScope `
                    -State $State `
                    -Record $matches[0].Record `
                    -Function $matches[0].Ast
                continue
            }
            if ($leafName -ceq "Add-Type") {
                if ($ownerName -cne "Initialize-AmeR2cRNativeTypes" -or
                    $command.CommandElements.Count -ne 3 -or
                    $command.CommandElements[1] -isnot
                        [Management.Automation.Language.CommandParameterAst] -or
                    $command.CommandElements[1].ParameterName -cne "TypeDefinition" -or
                    $command.CommandElements[2] -isnot
                        [Management.Automation.Language.VariableExpressionAst] -or
                    $command.CommandElements[2].VariablePath.UserPath -cne
                        "nativeTypeDefinition") {
                    throw "R2c-R deletion safety audit rejected Add-Type outside its exact native boundary"
                }
            }
            if ($leafName -ceq "Move-Item") {
                if ($command.CommandElements.Count -ne 5 -or
                    $command.CommandElements[1] -isnot
                        [Management.Automation.Language.CommandParameterAst] -or
                    $command.CommandElements[1].ParameterName -cne "LiteralPath" -or
                    $command.CommandElements[2] -isnot
                        [Management.Automation.Language.StringConstantExpressionAst] -or
                    $command.CommandElements[3] -isnot
                        [Management.Automation.Language.CommandParameterAst] -or
                    $command.CommandElements[3].ParameterName -cne "Destination" -or
                    $command.CommandElements[4] -isnot
                        [Management.Automation.Language.StringConstantExpressionAst]) {
                    throw "R2c-R deletion safety audit rejected non-literal Move-Item"
                }
                $moveSource = [IO.Path]::GetFullPath(
                    [string]$command.CommandElements[2].Value
                )
                $moveDestination = [IO.Path]::GetFullPath(
                    [string]$command.CommandElements[4].Value
                )
                $moveParent = [IO.Path]::GetDirectoryName($moveSource).TrimEnd('\')
                $moveLeaf = [IO.Path]::GetFileName($moveSource)
                if (-not $moveParent.Equals(
                        $State.ToolRoot,
                        [StringComparison]::OrdinalIgnoreCase
                    ) -or
                    $moveLeaf -cnotmatch '^\.ame-r2c-r-bootstrap-[0-9a-f]{32}$' -or
                    $moveDestination -cne ($moveSource + "-moved")) {
                    throw "R2c-R deletion safety audit rejected Move-Item outside the compiler bootstrap contract"
                }
            }
            if ($leafName -ceq "New-Item" -and
                @($command.CommandElements | Where-Object {
                    $_ -is [Management.Automation.Language.CommandParameterAst] -and
                        $_.ParameterName -ceq "Force"
                }).Count -ne 0) {
                throw "R2c-R deletion safety audit rejected forceful New-Item"
            }
            if (-not $script:AmeR2cRAllowedCmdlets.ContainsKey($leafName)) {
                throw (
                    "R2c-R deletion safety audit rejected unresolved helper or external " +
                    "executable '$leafName' in '$($scope.Record.Label)'"
                )
            }
            $expectedModule = [string]$script:AmeR2cRAllowedCmdlets[$leafName]
            if ($null -ne $moduleName -and $moduleName -cne $expectedModule) {
                throw (
                    "R2c-R deletion safety audit rejected module '$moduleName' for " +
                    "cmdlet '$leafName'; expected '$expectedModule'"
                )
            }
            if ($leafName -ceq "Start-Process" -and
                $ownerName -cne "Invoke-AmeR2cROwnedProcess") {
                throw "R2c-R deletion safety audit rejected Start-Process outside the verified boundary"
            }
        }

        $members = @($scopeAst.FindAll({
            param($node)
            if ($node -isnot [Management.Automation.Language.InvokeMemberExpressionAst]) {
                return $false
            }
            if ($null -ne $scope.Function) { return $true }
            return $null -eq (Get-AmeR2cRAstOwningFunction -Node $node)
        }, $true))
        foreach ($member in $members) {
            $memberName = [string]$member.Member.Value
            if ($memberName -ieq "Delete") {
                throw "R2c-R deletion safety audit rejected path-addressed Delete"
            }
            if ($memberName -iin @("Move", "Replace") -and
                $member.Static -and
                $member.Expression -is [Management.Automation.Language.TypeExpressionAst] -and
                [string]$member.Expression.TypeName.FullName -iin @(
                    "IO.File", "System.IO.File", "IO.Directory", "System.IO.Directory"
                )) {
                $isLockedGuardrailMove = (
                    $memberName -ceq "Move" -and
                    [string]$member.Expression.TypeName.FullName -iin @(
                        "IO.Directory", "System.IO.Directory",
                        "IO.File", "System.IO.File"
                    ) -and
                    -not [string]::IsNullOrEmpty($scope.Record.Path) -and
                    [IO.Path]::GetFileName($scope.Record.Path) -ceq
                        "acceptance_test_r2c_change_driven_reliability_guardrails.ps1" -and
                    $scope.Record.Digest -ceq $script:AmeR2cRGuardrailMoveSourceDigest
                )
                if (-not $isLockedGuardrailMove) {
                    throw "R2c-R deletion safety audit rejected path-addressed move or replace"
                }
            }
            if ($memberName -ieq "MoveTo") {
                throw "R2c-R deletion safety audit rejected path-addressed MoveTo"
            }
            if ($memberName -iin @("Invoke", "InvokeMember", "DynamicInvoke", "CreateDelegate")) {
                throw "R2c-R deletion safety audit rejected reflection invocation"
            }
            if ($member.Static -and $memberName -ieq "Create" -and
                $member.Expression -is [Management.Automation.Language.TypeExpressionAst] -and
                [string]$member.Expression.TypeName.FullName -iin @(
                    "scriptblock", "System.Management.Automation.ScriptBlock"
                )) {
                throw "R2c-R deletion safety audit rejected ScriptBlock.Create"
            }
            if ($memberName -ceq "Start" -and
                $ownerName -cne "Invoke-AmeR2cROwnedProcess") {
                throw "R2c-R deletion safety audit rejected process Start outside the verified boundary"
            }
        }
    }
    Assert-AmeR2cRProcessBoundaryContract -State $State
}

function Assert-AmeR2cRDeletionSafetySource {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)][AllowNull()][object]$SourceText,
        [Parameter(Mandatory = $true)][ValidateNotNullOrEmpty()][string]$Label,
        [AllowNull()][string]$SourcePath,
        [switch]$RetainSnapshots
    )
    if ($null -ne $SourceText -and $SourceText -isnot [string]) {
        throw "R2c-R deletion safety audit source text must be a string"
    }
    $state = New-AmeR2cRDeletionAuditState
    $retain = $false
    try {
        $root = Add-AmeR2cRDeletionAuditSource `
            -State $state `
            -SourceText $SourceText `
            -Label $Label `
            -SourcePath $SourcePath `
            -IsRoot $true `
            -Depth 0
        Invoke-AmeR2cRDeletionAuditClosure -State $state -Root $root
        if ($RetainSnapshots) {
            $retain = $true
            return $state
        }
    } finally {
        if (-not $retain) { Close-AmeR2cRDeletionAuditState -State $state }
    }
}
