$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$repositoryRoot = Get-AmeRepositoryRoot
$workflowPath = Join-Path $repositoryRoot ".github/workflows/release_candidate_windows.yml"
$workflowText = [System.IO.File]::ReadAllText($workflowPath)
$jobMatch = [regex]::Match(
    $workflowText,
    '(?ms)^  publish_portable:\r?\n.*?(?=^  [a-z_]+:|\z)'
)
$stepMatch = [regex]::Match(
    $jobMatch.Value,
    '(?ms)^      - name: Publish GitHub Release from the verified source commit\r?\n' +
    '.*?^        run: \|\r?\n(?<Body>.*)\z'
)
if (-not $jobMatch.Success -or -not $stepMatch.Success) {
    throw "The no-checkout portable publication workflow step was not found"
}
$inlineSource = @(
    foreach ($line in ($stepMatch.Groups["Body"].Value -split "`r?`n")) {
        if ($line.StartsWith("          ")) {
            $line.Substring(10)
        } elseif ([string]::IsNullOrWhiteSpace($line)) {
            ""
        } else {
            throw "The portable publication step contains unexpected YAML structure"
        }
    }
) -join "`n"
$tokens = $null
$parseErrors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseInput(
    $inlineSource,
    [ref]$tokens,
    [ref]$parseErrors
)
if ($parseErrors.Count -ne 0) {
    throw "The inline portable publication PowerShell could not be parsed"
}
$definitions = @($ast.FindAll({
    param($node)
    $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
        $node.Name -ceq "Get-ReleasePublicationAction"
}, $true))
$calls = @($ast.FindAll({
    param($node)
    $node -is [System.Management.Automation.Language.CommandAst] -and
        $node.GetCommandName() -ceq "Get-ReleasePublicationAction"
}, $true))
if ($definitions.Count -ne 1 -or $calls.Count -ne 1) {
    throw "Publication must invoke exactly one workflow-owned asset identity decision"
}
foreach ($forbidden in @("--clobber", "actions/checkout@", "./tool/", ".\tool\")) {
    if ($jobMatch.Value.Contains($forbidden)) {
        throw "Portable publication admits an overwrite or repository-code execution"
    }
}
. ([scriptblock]::Create($definitions[0].Extent.Text))

$script:publicationResponses = [System.Collections.Generic.Queue[object]]::new()
$fixtureEndpoint = "https://api.github.com/repos/fixture/ame/releases"
$fixtureTag = "v1.2.3"
$fixtureName = "Cedarflake-Ame-v1.2.3-windows-x64-portable.zip"
$fixtureHash = "a" * 64
$fixtureRelease = '{"id":42,"tag_name":"v1.2.3"}'

function Invoke-WebRequest {
    [CmdletBinding()]
    param(
        [Uri]$Uri,
        [hashtable]$Headers,
        [switch]$SkipHttpErrorCheck,
        [int]$TimeoutSec
    )

    if (
        $script:publicationResponses.Count -eq 0 -or
        -not $SkipHttpErrorCheck -or $TimeoutSec -ne 30 -or
        $Headers.Authorization -cne "Bearer fixture-token" -or
        $Headers["X-GitHub-Api-Version"] -cne "2022-11-28"
    ) {
        throw "Publication fixture refused an unexpected HTTP request"
    }
    $response = $script:publicationResponses.Dequeue()
    if ($Uri.AbsoluteUri -cne $response.Uri) {
        throw "Publication fixture received an unexpected API endpoint"
    }
    if ($response.ContainsKey("Failure")) {
        throw $response.Failure
    }
    return [pscustomobject]@{
        StatusCode = $response.StatusCode
        Content = $response.Content
    }
}

function Assert-AmePortablePublicationFixture {
    param(
        [Parameter(Mandatory = $true)]
        [object[]]$Responses,
        [string]$ExpectedAction,
        [string]$ExpectedFailure
    )

    $script:publicationResponses.Clear()
    foreach ($response in $Responses) {
        $script:publicationResponses.Enqueue($response)
    }
    $actualFailure = $null
    $actualAction = $null
    try {
        $actualAction = Get-ReleasePublicationAction `
            -Repository "fixture/ame" `
            -ReleaseTag $fixtureTag `
            -ArchiveName $fixtureName `
            -ArchiveSha256 $fixtureHash `
            -Token "fixture-token"
    } catch {
        $actualFailure = $_.Exception.Message
    }
    if ($ExpectedFailure) {
        if ($actualFailure -cne $ExpectedFailure) {
            throw "Publication fixture did not reject at its owning boundary: $actualFailure"
        }
    } elseif ($null -ne $actualFailure -or $actualAction -cne $ExpectedAction) {
        throw "Publication fixture returned '$actualAction' / '$actualFailure' instead of '$ExpectedAction'"
    }
    if ($script:publicationResponses.Count -ne 0) {
        throw "Publication fixture did not complete the required remote lookup"
    }
}

$releaseResponse = @{
    Uri = "$fixtureEndpoint/tags/$fixtureTag"
    StatusCode = 200
    Content = $fixtureRelease
}
$assetEndpoint = "$fixtureEndpoint/42/assets?per_page=100&page=1"
Assert-AmePortablePublicationFixture -ExpectedAction "create" -Responses @(@{
    Uri = "$fixtureEndpoint/tags/$fixtureTag"
    StatusCode = 404
    Content = '{"message":"Not Found"}'
})
Assert-AmePortablePublicationFixture -ExpectedAction "upload" -Responses @(
    $releaseResponse
    @{ Uri = $assetEndpoint; StatusCode = 200; Content = "[]" }
)
foreach ($case in @(
    @{ Digest = "sha256:$fixtureHash"; Action = "skip"; Failure = $null }
    @{
        Digest = "sha256:$('b' * 64)"
        Action = $null
        Failure = "Existing portable asset differs from the verified ZIP identity"
    }
    @{
        Digest = $null
        Action = $null
        Failure = "Existing portable asset has no trustworthy uploaded SHA-256 digest"
    }
)) {
    $assetJson = ConvertTo-Json -InputObject @(@{
        name = $fixtureName
        state = "uploaded"
        digest = $case.Digest
    }) -Compress
    Assert-AmePortablePublicationFixture `
        -ExpectedAction $case.Action -ExpectedFailure $case.Failure -Responses @(
            $releaseResponse
            @{ Uri = $assetEndpoint; StatusCode = 200; Content = $assetJson }
        )
}
$missingDigestJson = ConvertTo-Json -InputObject @(@{
    name = $fixtureName
    state = "uploaded"
}) -Compress
Assert-AmePortablePublicationFixture `
    -ExpectedFailure "Existing portable asset has no trustworthy uploaded SHA-256 digest" `
    -Responses @(
        $releaseResponse
        @{ Uri = $assetEndpoint; StatusCode = 200; Content = $missingDigestJson }
    )
Assert-AmePortablePublicationFixture `
    -ExpectedFailure "GitHub release lookup failed with HTTP 503" -Responses @(@{
        Uri = "$fixtureEndpoint/tags/$fixtureTag"
        StatusCode = 503
        Content = '{"message":"Service Unavailable"}'
    })
Assert-AmePortablePublicationFixture `
    -ExpectedFailure "controlled release transport failure" -Responses @(@{
        Uri = "$fixtureEndpoint/tags/$fixtureTag"
        Failure = "controlled release transport failure"
    })
Assert-AmePortablePublicationFixture `
    -ExpectedFailure "GitHub release asset lookup failed with HTTP 503" -Responses @(
        $releaseResponse
        @{ Uri = $assetEndpoint; StatusCode = 503; Content = "{}" }
    )
Assert-AmePortablePublicationFixture `
    -ExpectedFailure "GitHub release asset lookup did not return an asset array" -Responses @(
        $releaseResponse
        @{ Uri = $assetEndpoint; StatusCode = 200; Content = "{}" }
    )
$firstPage = @(
    for ($index = 0; $index -lt 100; $index += 1) {
        @{ name = "other-$index.zip" }
    }
) | ConvertTo-Json -Compress
$matchingPage = ConvertTo-Json -InputObject @(@{
    name = $fixtureName
    state = "uploaded"
    digest = "sha256:$fixtureHash"
}) -Compress
Assert-AmePortablePublicationFixture -ExpectedAction "skip" -Responses @(
    $releaseResponse
    @{ Uri = $assetEndpoint; StatusCode = 200; Content = $firstPage }
    @{
        Uri = "$fixtureEndpoint/42/assets?per_page=100&page=2"
        StatusCode = 200
        Content = $matchingPage
    }
)
Write-Host "Portable publication identity fixtures passed"
