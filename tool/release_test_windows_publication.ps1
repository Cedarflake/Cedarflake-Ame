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
$readNotes = Get-PublicationStep 'Read versioned release notes'
$publish = Get-PublicationStep 'Publish GitHub Release from the verified source commit'
$fixtureTag = 'v1.2.3-alpha.1'
$fixtureCommit = 'a' * 40
$fixtureNotesCommit = 'c' * 40
$fixtureNotes = "# Changes`n`n- $([char]0x66f4)$([char]0x65b0)`n"
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
        if (-not $state.Exists -or $state.Draft) { return [pscustomobject]@{ StatusCode = 404; Content = '{}' } }
        $content = @{ id = 42; tag_name = $fixtureTag; draft = $state.Draft } | ConvertTo-Json -Compress
    } elseif ($Uri.AbsolutePath -ceq '/repos/fixture/ame/releases') {
        if ($Uri.Query -notmatch '^\?per_page=100&page=([1-9][0-9]*)$') { throw 'Unexpected release pagination request' }
        $page = [int]$Matches[1]
        $state.ReleasePages.Add($page)
        $releases = @(
            if ($state.PaginatedReleases) {
                for ($index = 0; $index -lt 100; $index++) {
                    @{ id = 1000 + $index; tag_name = "other-$index"; draft = $false }
                }
            }
            if ($state.Exists) { @{ id = 42; tag_name = $fixtureTag; draft = $state.Draft } }
            if ($state.DuplicateRelease) { @{ id = 43; tag_name = $fixtureTag; draft = $true } }
        )
        $content = ConvertTo-Json -InputObject @($releases | Select-Object -Skip (($page - 1) * 100) -First 100) -Compress
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
    if ($Arguments[0] -ceq 'api' -and $Arguments[1] -ceq
        "repos/fixture/ame/contents/docs/releases/$fixtureTag.md?ref=$fixtureNotesCommit") {
        if ($state.NotesMissing) { $global:LASTEXITCODE = 1; return }
        $text = if ($state.NotesEmpty) { " `n " } else { $fixtureNotes }
        $bytes = [Text.Encoding]::UTF8.GetBytes($text)
        $path = if ($state.NotesWrongVersion) { 'docs/releases/v9.9.9.md' } else { "docs/releases/$fixtureTag.md" }
        return (@{ type = 'file'; path = $path; encoding = 'base64'; size = $bytes.Length
            content = [Convert]::ToBase64String($bytes) } | ConvertTo-Json -Compress)
    }
    if ($Arguments[0] -cne 'release' -or $Arguments -ccontains '--clobber') { throw 'Unexpected publication command' }
    switch ($Arguments[1]) {
        'create' {
            if ($state.Exists -or $Arguments -cnotcontains '--draft' -or $Arguments -cnotcontains '--verify-tag') {
                throw 'A new release must begin as a tag-bound draft'
            }
            $state.Exists = $true
            $state.Draft = $true
            $state.Body = Read-FixtureNotes $Arguments
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
            $state.Body = Read-FixtureNotes $Arguments
            $state.Writes.Add('publish')
        }
        default { throw 'Unexpected release mutation' }
    }
}

function Read-FixtureNotes([object[]]$Arguments) {
    $index = [Array]::IndexOf($Arguments, '--notes-file')
    if ($index -lt 0 -or $Arguments -ccontains '--generate-notes') { throw 'Versioned release notes were not supplied' }
    $text = [IO.File]::ReadAllText([string]$Arguments[$index + 1], [Text.Encoding]::UTF8)
    if ($text -cne $fixtureNotes) { throw 'Published release notes changed their Markdown or Unicode content' }
    return $text
}

$savedEnvironment = @{}
foreach ($name in @('AME_ARCHIVE_NAME', 'AME_ARCHIVE_SHA256', 'AME_INSTALLER_NAME', 'AME_INSTALLER_SHA256',
        'AME_RELEASE_TAG', 'AME_SOURCE_COMMIT', 'AME_NOTES_COMMIT', 'GITHUB_REPOSITORY', 'GH_TOKEN')) {
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
        $env:AME_NOTES_COMMIT = $fixtureNotesCommit
        $env:GITHUB_REPOSITORY = 'fixture/ame'
        $env:GH_TOKEN = 'fixture-token'
        Push-Location $scratch
        try {
            foreach ($scenario in @('new', 'existing', 'draft', 'paginated-releases', 'duplicate-releases',
                    'missing-msi', 'failed-upload', 'conflicting-msi',
                    'missing-digest', 'lookup-failure', 'paginated', 'missing-notes', 'empty-notes',
                    'wrong-version-notes', 'missing-local-msi')) {
                $state = @{
                    Exists = $scenario -notin @('new', 'failed-upload', 'missing-local-msi')
                    Draft = ($scenario -in @('draft', 'paginated-releases', 'duplicate-releases'))
                    FailMsi = ($scenario -ceq 'failed-upload')
                    PaginatedReleases = ($scenario -ceq 'paginated-releases')
                    DuplicateRelease = ($scenario -ceq 'duplicate-releases')
                    LookupFailure = ($scenario -ceq 'lookup-failure')
                    NotesMissing = ($scenario -ceq 'missing-notes')
                    NotesEmpty = ($scenario -ceq 'empty-notes')
                    NotesWrongVersion = ($scenario -ceq 'wrong-version-notes')
                    Body = 'Previous release notes'
                    Assets = [Collections.Generic.List[object]]::new()
                    Writes = [Collections.Generic.List[string]]::new()
                    Pages = [Collections.Generic.List[int]]::new()
                    ReleasePages = [Collections.Generic.List[int]]::new()
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
                try { & $admission; & $readNotes; & $publish } catch { $failure = $_.Exception.Message }
                switch ($scenario) {
                    'failed-upload' {
                        if ($null -eq $failure -or -not $state.Draft -or $state.Writes -ccontains 'publish') { throw 'A partial upload exposed a public release' }
                    }
                    { $_ -in @('duplicate-releases', 'conflicting-msi', 'missing-digest', 'lookup-failure', 'missing-local-msi',
                            'missing-notes', 'empty-notes', 'wrong-version-notes') } {
                        if ($null -eq $failure -or $state.Writes.Count -ne 0) { throw "Unsafe publication was not rejected before mutation: $scenario" }
                    }
                    default {
                        if ($null -ne $failure -or $state.Draft) { throw "Publication fixture failed: $scenario / $failure" }
                        if ($state.Body -cne $fixtureNotes) { throw 'The release body does not match the versioned notes' }
                        $expectedWrites = switch ($scenario) { 'new' { 4 }; 'missing-msi' { 2 }; default { 1 } }
                        if ($state.Writes.Count -ne $expectedWrites) { throw "Publication performed unintended writes: $scenario" }
                        if ($scenario -ceq 'paginated' -and $state.Pages -notcontains 2) { throw 'Publication ignored later asset pages' }
                        if ($scenario -ceq 'paginated-releases' -and $state.ReleasePages -notcontains 2) { throw 'Publication ignored a draft on a later release page' }
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
