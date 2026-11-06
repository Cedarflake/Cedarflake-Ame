$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'quality_common.ps1')
. (Join-Path $PSScriptRoot 'acceptance_scratch_storage.ps1')
. (Join-Path $PSScriptRoot 'release_published_packages.ps1')
$workflow = [IO.File]::ReadAllText((Join-Path (Get-AmeRepositoryRoot) '.github/workflows/release_publish_windows.yml'), [Text.Encoding]::UTF8)

function Get-PublicationStep([string]$Name) {
    $pattern = '(?ms)^      - name: ' + [regex]::Escape($Name) + '\r?\n.*?^        run: \|\r?\n(?<body>.*?)(?=^      - name:|\z)'
    $match = [regex]::Match($workflow, $pattern)
    if (-not $match.Success) { throw "Publication step is missing: $Name" }
    $lines = foreach ($line in ($match.Groups['body'].Value -split "`r?`n")) {
        if ($line.StartsWith('          ')) { $line.Substring(10) }
        elseif ([string]::IsNullOrWhiteSpace($line)) { '' }
        else { throw 'Publication step contains unexpected YAML structure' }
    }
    [scriptblock]::Create(($lines -join "`n"))
}
$admission = Get-PublicationStep 'Validate immutable package pair'
$publish = Get-PublicationStep 'Publish GitHub Release from the verified source commit'
$fixtureTag = 'v1.2.3-alpha.1'
$fixtureCommit = 'a' * 40
$script:publicationState = $null
$script:fixtureArtifacts = @()

function Invoke-WebRequest {
    param([Uri]$Uri, [hashtable]$Headers, [switch]$SkipHttpErrorCheck, [int]$TimeoutSec)
    if (-not $SkipHttpErrorCheck -or $TimeoutSec -ne 30 -or
        $Headers.Authorization -cne 'Bearer fixture-token' -or
        $Headers['X-GitHub-Api-Version'] -cne '2022-11-28') { throw 'Unexpected publication HTTP request' }
    $state = $script:publicationState
    if ($state.LookupFailure) { return [pscustomobject]@{ StatusCode = 503; Content = '{}' } }
    if ($Uri.AbsolutePath -ceq "/repos/fixture/ame/releases/tags/$fixtureTag") {
        if (-not $state.Exists) { return [pscustomobject]@{ StatusCode = 404; Content = '{}' } }
        $content = @{ id = 42; tag_name = $fixtureTag; draft = $state.Draft } | ConvertTo-Json -Compress
    } elseif ($Uri.AbsolutePath -ceq '/repos/fixture/ame/releases/42/assets') {
        if ($Uri.Query -notmatch '^\?per_page=100&page=([1-9][0-9]*)$') { throw 'Unexpected asset pagination request' }
        $page = [int]$Matches[1]
        $state.Pages.Add($page)
        $offset = ($page - 1) * 100
        $content = ConvertTo-Json -InputObject @($state.Assets | Select-Object -Skip $offset -First 100) -Compress
    } else { throw "Unexpected publication endpoint: $Uri" }
    return [pscustomobject]@{ StatusCode = 200; Content = $content }
}

function gh {
    param([Parameter(ValueFromRemainingArguments = $true)][object[]]$Arguments)
    $Arguments = @($Arguments | ForEach-Object { $_ })
    $global:LASTEXITCODE = 0
    $state = $script:publicationState
    if ($Arguments[0] -ceq 'api' -and $Arguments[1] -ceq "repos/fixture/ame/git/ref/tags/$fixtureTag") {
        return (@{ object = @{ type = 'commit'; sha = $fixtureCommit } } | ConvertTo-Json -Compress)
    }
    if ($Arguments[0] -cne 'release' -or $Arguments -ccontains '--clobber') { throw 'Unexpected publication command' }
    switch ($Arguments[1]) {
        'create' {
            if ($state.Exists -or $Arguments -cnotcontains '--draft' -or $Arguments -cnotcontains '--verify-tag') {
                throw 'A new release must begin as a tag-bound draft'
            }
            $state.Exists = $true
            $state.Draft = $true
            $state.Writes.Add('create-draft')
        }
        'upload' {
            foreach ($argument in $Arguments) {
                $artifact = @($script:fixtureArtifacts | Where-Object { [IO.Path]::GetFileName([string]$argument) -ceq $_.Name })
                if ($artifact.Count -eq 0) { continue }
                if ($state.FailMsi -and $artifact[0].Name.EndsWith('.msi')) {
                    $global:LASTEXITCODE = 1
                    return
                }
                $state.Assets.Add([pscustomobject]@{
                    name = $artifact[0].Name; state = 'uploaded'; size = $artifact[0].Size
                    digest = 'sha256:' + $artifact[0].Sha256
                })
                $state.Writes.Add('upload:' + $artifact[0].Name)
            }
        }
        'edit' {
            if ($Arguments -cnotcontains '--draft=false' -or
                @($state.Assets | Where-Object { $script:fixtureArtifacts.Name -ccontains $_.name }).Count -ne 2) {
                throw 'Publication was attempted without both packages'
            }
            $state.Draft = $false
            $state.Writes.Add('publish')
        }
        default { throw 'Unexpected release mutation' }
    }
}

$savedEnvironment = @{}
foreach ($name in @('AME_ARCHIVE_NAME', 'AME_ARCHIVE_SHA256', 'AME_INSTALLER_NAME', 'AME_INSTALLER_SHA256',
        'AME_RELEASE_TAG', 'AME_SOURCE_COMMIT', 'GITHUB_REPOSITORY', 'GH_TOKEN')) {
    $savedEnvironment[$name] = [Environment]::GetEnvironmentVariable($name)
}
try {
    Invoke-AmeAcceptanceScratch -Name 'windows-publication' -Action {
        param($scratch)
        $directory = Join-Path $scratch 'build/publish'
        [void][IO.Directory]::CreateDirectory($directory)
        $script:fixtureArtifacts = @(foreach ($suffix in @('portable.zip', 'msi')) {
            $name = "Cedarflake-Ame-$fixtureTag-windows-x64-$suffix"
            if ($suffix -ceq 'msi') { $name = "Cedarflake-Ame-$fixtureTag-windows-x64.msi" }
            $path = Join-Path $directory $name
            [IO.File]::WriteAllText($path, $suffix, [Text.UTF8Encoding]::new($false))
            [pscustomobject]@{ Name = $name; Size = (Get-Item -LiteralPath $path).Length; Sha256 = (Get-FileHash -LiteralPath $path).Hash.ToLowerInvariant() }
        })
        $env:AME_ARCHIVE_NAME = $script:fixtureArtifacts[0].Name
        $env:AME_ARCHIVE_SHA256 = $script:fixtureArtifacts[0].Sha256
        $env:AME_INSTALLER_NAME = $script:fixtureArtifacts[1].Name
        $env:AME_INSTALLER_SHA256 = $script:fixtureArtifacts[1].Sha256
        $env:AME_RELEASE_TAG = $fixtureTag
        $env:AME_SOURCE_COMMIT = $fixtureCommit
        $env:GITHUB_REPOSITORY = 'fixture/ame'
        $env:GH_TOKEN = 'fixture-token'
        Push-Location $scratch
        try {
            foreach ($scenario in @('new', 'existing', 'missing-msi', 'failed-upload', 'conflicting-msi',
                    'missing-digest', 'lookup-failure', 'paginated', 'missing-local-msi')) {
                $state = @{
                    Exists = $scenario -notin @('new', 'failed-upload', 'missing-local-msi')
                    Draft = $false; FailMsi = ($scenario -ceq 'failed-upload')
                    LookupFailure = ($scenario -ceq 'lookup-failure')
                    Assets = [Collections.Generic.List[object]]::new()
                    Writes = [Collections.Generic.List[string]]::new()
                    Pages = [Collections.Generic.List[int]]::new()
                }
                if ($scenario -ceq 'paginated') {
                    for ($index = 0; $index -lt 100; $index++) { $state.Assets.Add([pscustomobject]@{ name = "other-$index" }) }
                }
                if ($state.Exists) {
                    foreach ($artifact in $script:fixtureArtifacts) {
                        if ($scenario -ceq 'missing-msi' -and $artifact.Name.EndsWith('.msi')) { continue }
                        $digest = 'sha256:' + $artifact.Sha256
                        if ($artifact.Name.EndsWith('.msi')) {
                            if ($scenario -ceq 'conflicting-msi') { $digest = 'sha256:' + ('b' * 64) }
                            if ($scenario -ceq 'missing-digest') { $digest = $null }
                        }
                        $state.Assets.Add([pscustomobject]@{ name = $artifact.Name; digest = $digest; state = 'uploaded'; size = $artifact.Size })
                    }
                }
                $script:publicationState = $state
                $failure = $null
                if ($scenario -ceq 'missing-local-msi') { Remove-Item -LiteralPath (Join-Path $directory $env:AME_INSTALLER_NAME) }
                try { & $admission; & $publish } catch { $failure = $_.Exception.Message }
                switch ($scenario) {
                    'failed-upload' {
                        if ($null -eq $failure -or -not $state.Draft -or $state.Writes -ccontains 'publish') { throw 'A partial upload exposed a public release' }
                    }
                    { $_ -in @('conflicting-msi', 'missing-digest', 'lookup-failure', 'missing-local-msi') } {
                        if ($null -eq $failure -or $state.Writes.Count -ne 0) { throw "Unsafe publication was not rejected before mutation: $scenario" }
                    }
                    default {
                        if ($null -ne $failure -or $state.Draft) { throw "Publication fixture failed: $scenario / $failure" }
                        $expectedWrites = switch ($scenario) { 'new' { 4 }; 'missing-msi' { 1 }; default { 0 } }
                        if ($state.Writes.Count -ne $expectedWrites) { throw "Publication performed unintended writes: $scenario" }
                        if ($scenario -ceq 'paginated' -and $state.Pages -notcontains 2) { throw 'Publication ignored later asset pages' }
                        $release = [pscustomobject]@{ tag_name = $fixtureTag; draft = $false; assets = $state.Assets }
                        Assert-AmePublishedPackages -Release $release -Tag $fixtureTag -Directory $directory `
                            -ExpectedArchiveSha256 $env:AME_ARCHIVE_SHA256 -ExpectedInstallerSha256 $env:AME_INSTALLER_SHA256
                    }
                }
            }
            $publishedPair = [pscustomobject]@{
                tag_name = $fixtureTag; draft = $false
                assets = @($script:fixtureArtifacts | ForEach-Object {
                    [pscustomobject]@{ name = $_.Name; state = 'uploaded'; size = $_.Size; digest = 'sha256:' + $_.Sha256 }
                })
            }
            foreach ($case in @('missing-installer', 'changed-installer')) {
                if ($case -ceq 'changed-installer') {
                    [IO.File]::WriteAllText((Join-Path $directory $env:AME_INSTALLER_NAME), 'corrupt', [Text.UTF8Encoding]::new($false))
                }
                $rejected = $false
                try { Assert-AmePublishedPackages -Release $publishedPair -Tag $fixtureTag -Directory $directory }
                catch { $rejected = $true }
                if (-not $rejected) { throw "Published download admitted $case" }
            }
        } finally { Pop-Location }
    }
} finally {
    foreach ($name in $savedEnvironment.Keys) { [Environment]::SetEnvironmentVariable($name, $savedEnvironment[$name]) }
}
Write-Host 'Windows two-package publication fixtures passed'
