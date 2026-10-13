$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "release_journal_broker_common.ps1")

function Assert-IdentityRejected {
    param([scriptblock]$Action, [string]$Message)

    $rejected = $false
    try {
        & $Action | Out-Null
    } catch {
        $rejected = $true
    }
    if (-not $rejected) {
        throw $Message
    }
}

Assert-AmeBrokerPublisherPolicy
Assert-AmeBrokerPublisherPolicy -ExpectedPublisher $null
Assert-AmeBrokerPublisherPolicy -ExpectedPublisher ""
foreach ($invalid in @(" ", "`t", ("A" * 513))) {
    Assert-IdentityRejected {
        Assert-AmeBrokerPublisherPolicy -ExpectedPublisher $invalid
    } "The optional publisher policy accepted an invalid nonempty subject"
}

$parent = [pscustomobject]@{
    Path = Get-AmeProductParentDirectory
    Existed = $false
    Sddl = $null
}
$state = New-AmeBrokerTransactionState -Operation upgrade -ParentPrestate $parent
Assert-AmeBrokerTransactionStateFacts -State $state
if ($state.Schema -ne 3 -or (Get-AmeBrokerRecoveryPublisher -State $state) -cne "") {
    throw "Unsigned transactions did not preserve their explicit recovery policy"
}
$state.Schema = 2
Assert-IdentityRejected {
    Assert-AmeBrokerTransactionStateFacts -State $state
} "A legacy transaction lost its mandatory publisher policy"
$publisher = "CN=Cedarflake Installer Fixture"
$state.ExpectedPublisher = $publisher
Assert-AmeBrokerTransactionStateFacts -State $state
if ((Get-AmeBrokerRecoveryPublisher -State $state) -cne $publisher) {
    throw "Legacy transaction recovery changed its original publisher policy"
}
$state.Schema = 3
if ((Get-AmeBrokerRecoveryPublisher -State $state) -cne "") {
    throw "New recovery incorrectly applied the incoming publisher to the previous payload"
}

$fixtureRoot = Join-Path (Split-Path -Parent $PSScriptRoot) (
    ".build/journal-broker-identity-" + [Guid]::NewGuid().ToString("N")
)
[IO.Directory]::CreateDirectory($fixtureRoot) | Out-Null
$payload = Join-Path $fixtureRoot "unsigned-fixture.exe"
try {
    $pe = [byte[]]::new(70)
    $pe[0] = 0x4D
    $pe[1] = 0x5A
    $pe[0x3C] = 64
    $pe[64] = 0x50
    $pe[65] = 0x45
    $pe[68] = 0x64
    $pe[69] = 0x86
    [IO.File]::WriteAllBytes($payload, $pe)
    & {
        function Get-AmeBrokerProtocolFacts {
            throw "Passive payload and recovery inspection must never execute the image"
        }
        Assert-AmeBrokerPayload -Path $payload | Out-Null
        $facts = Get-AmeBrokerTransactionBinaryFacts -Path $payload
        if (-not $facts.Exists -or $facts.Length -ne $pe.Length) {
            throw "Unsigned payload facts were not captured without execution"
        }
        Assert-IdentityRejected {
            Assert-AmeBrokerPayload -Path $payload -ExpectedPublisher $publisher
        } "An explicit publisher policy accepted an unsigned binary"
    }

    $hash = (Get-FileHash -LiteralPath $payload -Algorithm SHA256).Hash
    $v3 = "AMEJBID3|protocol=5|max_frame=1048576|sha256=$hash"
    Assert-AmeBrokerIdentityManifestFacts -Manifest $v3
    Assert-AmeBrokerManifestBinary -BinaryPath $payload -Manifest $v3
    foreach ($invalidManifest in @(
        $v3.Replace("protocol=5", "protocol=05"),
        $v3.Replace("max_frame=1048576", "max_frame=1048577"),
        $v3.Replace($hash, $hash.ToLowerInvariant()),
        ($v3 + "|client_path=x"),
        ($v3 + "`n"),
        $v3.Replace("AMEJBID3", "AMEJBID4")
    )) {
        Assert-IdentityRejected {
            Assert-AmeBrokerIdentityManifestFacts -Manifest $invalidManifest
        } "The broker identity parser accepted a malformed or incompatible record"
    }
    Assert-IdentityRejected {
        Assert-AmeBrokerManifestBinary -BinaryPath $payload -Manifest $v3.Replace($hash, ("A" * 64))
    } "Installed broker validation accepted a payload hash mismatch"
    & {
        function Get-AmeBrokerProtocolFacts {
            param([string]$Path)
            if ($Path -cne $payload) { throw "Unexpected protocol probe path" }
            return [pscustomobject]@{ Protocol = 5; MaximumFrameBytes = 1048576 }
        }
        if ((New-AmeBrokerIdentityManifest -BinaryPath $payload) -cne $v3) {
            throw "New unsigned installation did not generate the strict V3 broker identity"
        }
    }

    $certificateBytes = [Text.Encoding]::UTF8.GetBytes("generated certificate fixture")
    $hasher = [Security.Cryptography.SHA256]::Create()
    try {
        $certificateHash = ConvertTo-AmeBrokerUpperHex -Bytes ($hasher.ComputeHash($certificateBytes))
    } finally {
        $hasher.Dispose()
    }
    $publisherHex = ConvertTo-AmeBrokerUpperHex -Bytes ([Text.Encoding]::Unicode.GetBytes($publisher))
    $clientHex = ConvertTo-AmeBrokerUpperHex -Bytes (
        [Text.Encoding]::Unicode.GetBytes((Get-AmeApplicationInstalledBinaryPath))
    )
    $v2 = "AMEJBID2|protocol=5|max_frame=1048576|sha256=$hash|" +
        "signer_sha256=$certificateHash|publisher_utf16le=$publisherHex|" +
        "client_path_utf16le=$clientHex|client_sha256=$hash|client_signer_sha256=$certificateHash"
    Assert-AmeBrokerIdentityManifestFacts -Manifest $v2
    & {
        function Get-AuthenticodeSignature {
            param([string]$LiteralPath)
            if ($LiteralPath -cne $payload) { throw "Unexpected signature inspection path" }
            return [pscustomobject]@{
                Status = "Valid"
                SignerCertificate = [pscustomobject]@{
                    Subject = $publisher
                    RawData = $certificateBytes
                }
            }
        }
        Assert-AmeBrokerManifestBinary -BinaryPath $payload -Manifest $v2
        Assert-IdentityRejected {
            Assert-AmeBrokerManifestBinary `
                -BinaryPath $payload -Manifest $v2.Replace($certificateHash, ("B" * 64))
        } "Legacy rollback accepted a different signing certificate"
        Assert-IdentityRejected {
            Assert-AmeBrokerPayload -Path $payload -ExpectedPublisher "CN=Another Publisher"
        } "An explicit publisher policy accepted the wrong signer"
    }

    $facts = Get-AmeBrokerTransactionBinaryFacts -Path $payload
    $state.ExpectedPreviousBinary = $facts
    $state.ExpectedNewBinary = $facts
    $state.PreviousIdentityManifest = $v2
    $state.NewIdentityManifest = $v3
    foreach ($manifest in @($v2, $v3)) {
        if (-not (Test-AmeBrokerTransactionServiceIdentity `
            -State $state -BinaryFacts $facts -Manifest $manifest)) {
            throw "Recovery rejected a matching old/new manifest for identical payload bytes"
        }
    }
    if (Test-AmeBrokerTransactionServiceIdentity `
        -State $state -BinaryFacts $facts -Manifest $v3.Replace($hash, ("C" * 64))) {
        throw "Recovery accepted an unrelated manifest"
    }
    $state.ExpectedPreviousBinary = New-AmeBrokerAbsentBinaryFacts
    if (Test-AmeBrokerTransactionServiceIdentity -State $state -BinaryFacts $facts -Manifest $v2) {
        throw "Missing-binary repair admitted an unavailable previous payload"
    }
} finally {
    [IO.File]::Delete($payload)
    [IO.Directory]::Delete($fixtureRoot, $false)
}

foreach ($name in @("install", "upgrade", "repair", "uninstall")) {
    $command = Get-Command (Join-Path $PSScriptRoot "release_${name}_journal_broker.ps1")
    $publisherParameters = @($command.Parameters.ExpectedPublisher.Attributes | Where-Object {
        $_ -is [Management.Automation.ParameterAttribute] -and $_.Mandatory
    })
    if ($publisherParameters.Count -ne 0) {
        throw "Broker lifecycle still requires signing credentials"
    }
}
$install = Get-Command (Join-Path $PSScriptRoot "release_install_journal_broker.ps1")
if (@($install.Parameters.ApplicationBundlePath.Attributes | Where-Object {
    $_ -is [Management.Automation.ParameterAttribute] -and $_.Mandatory
}).Count -ne 0) {
    throw "Broker installation still requires a fixed client application bundle"
}

Write-Output "journal_broker_identity_guardrails_passed"
