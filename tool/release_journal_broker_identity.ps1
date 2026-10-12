function Assert-AmeBrokerSignatureFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Status,
        [AllowNull()]
        [string]$SignerSubject,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedPublisher
    )

    if ($Status -cne "Valid") {
        throw "The broker binary must have a valid Authenticode signature"
    }
    if ([string]::IsNullOrWhiteSpace($ExpectedPublisher)) {
        throw "The expected broker publisher must be explicit"
    }
    if ([string]::IsNullOrWhiteSpace($SignerSubject) -or -not $SignerSubject.Equals(
        $ExpectedPublisher,
        [System.StringComparison]::Ordinal
    )) {
        throw "The broker signer does not match the expected publisher"
    }
}

function Assert-AmeBrokerMachineFacts {
    param(
        [Parameter(Mandatory = $true)]
        [UInt16]$Machine
    )

    if ($Machine -ne 0x8664) {
        throw "The broker binary must be a Windows x64 PE image"
    }
}

function Get-AmeBrokerPeMachine {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $stream = [System.IO.File]::Open(
        $Path,
        [System.IO.FileMode]::Open,
        [System.IO.FileAccess]::Read,
        [System.IO.FileShare]::Read
    )
    try {
        if ($stream.Length -lt 64) {
            throw "The broker binary is not a complete PE image"
        }
        $reader = [System.IO.BinaryReader]::new($stream)
        try {
            if ($reader.ReadUInt16() -ne 0x5A4D) {
                throw "The broker binary is missing the DOS PE signature"
            }
            $stream.Position = 0x3C
            $peOffset = $reader.ReadUInt32()
            if ($peOffset -gt $stream.Length - 6) {
                throw "The broker PE header offset is invalid"
            }
            $stream.Position = $peOffset
            if ($reader.ReadUInt32() -ne 0x00004550) {
                throw "The broker binary is missing the NT PE signature"
            }
            return $reader.ReadUInt16()
        } finally {
            $reader.Dispose()
        }
    } finally {
        $stream.Dispose()
    }
}

function Assert-AmeSignedX64Binary {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedPublisher
    )

    $resolvedPath = [System.IO.Path]::GetFullPath($Path)
    if (-not (Test-Path -LiteralPath $resolvedPath -PathType Leaf)) {
        throw "The broker binary was not found: $resolvedPath"
    }
    Assert-AmeBrokerMachineFacts -Machine (Get-AmeBrokerPeMachine -Path $resolvedPath)
    $signature = Get-AuthenticodeSignature -LiteralPath $resolvedPath
    $signerSubject = if ($null -eq $signature.SignerCertificate) {
        $null
    } else {
        $signature.SignerCertificate.Subject
    }
    Assert-AmeBrokerSignatureFacts `
        -Status ([string]$signature.Status) `
        -SignerSubject $signerSubject `
        -ExpectedPublisher $ExpectedPublisher
    return $resolvedPath
}

function Assert-AmeBrokerBinary {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [AllowEmptyString()]
        [string]$ExpectedPublisher = ""
    )

    $resolvedPath = Assert-AmeBrokerPayload `
        -Path $Path `
        -ExpectedPublisher $ExpectedPublisher
    Get-AmeBrokerProtocolFacts -Path $resolvedPath | Out-Null
    return $resolvedPath
}

function Assert-AmeBrokerPublisherPolicy {
    param([AllowEmptyString()][string]$ExpectedPublisher = "")

    if ($ExpectedPublisher.Length -gt 512 -or
        ($ExpectedPublisher.Length -gt 0 -and
            [string]::IsNullOrWhiteSpace($ExpectedPublisher))) {
        throw "The optional publisher must be empty or an explicit certificate subject"
    }
}

function Assert-AmeBrokerPayload {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [AllowEmptyString()]
        [string]$ExpectedPublisher = ""
    )

    Assert-AmeBrokerPublisherPolicy -ExpectedPublisher $ExpectedPublisher
    if ($ExpectedPublisher.Length -gt 0) {
        return Assert-AmeSignedX64Binary -Path $Path -ExpectedPublisher $ExpectedPublisher
    }
    $resolvedPath = [IO.Path]::GetFullPath($Path)
    Assert-AmeBrokerMachineFacts -Machine (Get-AmeBrokerPeMachine -Path $resolvedPath)
    return $resolvedPath
}

function Copy-AmeBrokerStagedPayload {
    param(
        [Parameter(Mandatory = $true)]
        [string]$SourcePath,
        [Parameter(Mandatory = $true)]
        [string]$StagedPath,
        [Parameter(Mandatory = $true)]
        [bool]$IncludeServiceSid,
        [AllowEmptyString()]
        [string]$ExpectedPublisher = ""
    )

    $parent = [IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($StagedPath))
    Assert-AmeBrokerFixedInstallTree -InstallDirectory $parent -RequireInstallDirectory | Out-Null
    $source = [IO.File]::Open($SourcePath, 'Open', 'Read', 'Read')
    try {
        if ($source.Length -le 0 -or $source.Length -gt 128MB) {
            throw "The broker payload exceeds the bounded installer size"
        }
        $hasher = [Security.Cryptography.SHA256]::Create()
        try {
            $expectedHash = ConvertTo-AmeBrokerUpperHex -Bytes ($hasher.ComputeHash($source))
        } finally {
            $hasher.Dispose()
        }
        $source.Position = 0
        $destination = [IO.File]::Open($StagedPath, 'CreateNew', 'Write', 'None')
        try {
            $source.CopyTo($destination)
            $destination.Flush($true)
        } finally {
            $destination.Dispose()
        }
        Set-AmeBrokerFileAcl -Path $StagedPath -IncludeServiceSid $IncludeServiceSid
        Assert-AmeBrokerPhysicalFile -ExpectedPath $StagedPath
        Assert-AmeBrokerExactAclFacts `
            -Facts (Get-AmeBrokerAclFacts -Path $StagedPath) `
            -Kind File `
            -IncludeServiceSid $IncludeServiceSid
        $actualHash = (Get-FileHash -LiteralPath $StagedPath -Algorithm SHA256).Hash
        if ($actualHash -cne $expectedHash) {
            throw "The protected broker staging copy differs from the held source payload"
        }
        Assert-AmeBrokerPayload -Path $StagedPath -ExpectedPublisher $ExpectedPublisher | Out-Null
    } finally {
        $source.Dispose()
    }
}

function ConvertTo-AmeBrokerUpperHex {
    param(
        [Parameter(Mandatory = $true)]
        [byte[]]$Bytes
    )

    return ([BitConverter]::ToString($Bytes)).Replace("-", "")
}

function Get-AmeBrokerProtocolFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $process = [Diagnostics.Process]::new()
    $process.StartInfo = [Diagnostics.ProcessStartInfo]@{
        FileName = [IO.Path]::GetFullPath($Path)
        Arguments = "--protocol-info"
        UseShellExecute = $false
        CreateNoWindow = $true
        WindowStyle = [Diagnostics.ProcessWindowStyle]::Hidden
        RedirectStandardOutput = $true
        RedirectStandardError = $true
    }
    try {
        if (-not $process.Start()) {
            throw "The broker protocol probe could not start"
        }
        $stdout = [char[]]::new(256)
        $stderr = [char[]]::new(256)
        $outputRead = $process.StandardOutput.ReadBlockAsync($stdout, 0, $stdout.Length)
        $errorRead = $process.StandardError.ReadBlockAsync($stderr, 0, $stderr.Length)
        if (-not $process.WaitForExit(5000)) {
            $process.Kill()
            $process.WaitForExit(1000) | Out-Null
            throw "The broker protocol probe exceeded its five-second deadline"
        }
        if (-not $outputRead.Wait(1000) -or -not $errorRead.Wait(1000) -or
            $outputRead.Result -ge $stdout.Length -or $errorRead.Result -ne 0) {
            throw "The broker protocol probe exceeded its output contract"
        }
        $text = [string]::new($stdout, 0, $outputRead.Result)
        $output = @(($text -replace '\r?\n\z', '') -split '\r?\n')
        return Assert-AmeBrokerProtocolOutputFacts -Output $output -ExitCode $process.ExitCode
    } finally {
        $process.Dispose()
    }
}

function Assert-AmeBrokerProtocolOutputFacts {
    param(
        [AllowEmptyCollection()]
        [object[]]$Output,
        [Parameter(Mandatory = $true)]
        [int]$ExitCode
    )

    if ($exitCode -ne 0 -or $output.Count -ne 1) {
        throw "The broker protocol probe did not produce one successful record"
    }
    $match = [regex]::Match(
        [string]$output[0],
        '^binary=cedarflake_ame_journal_broker protocol=(\d+) max_frame=(\d+)$',
        [System.Text.RegularExpressions.RegexOptions]::CultureInvariant
    )
    if (-not $match.Success) {
        throw "The broker protocol probe output is malformed"
    }
    $facts = [pscustomobject]@{
        Protocol = [int]$match.Groups[1].Value
        MaximumFrameBytes = [int]$match.Groups[2].Value
    }
    if (
        $facts.Protocol -ne $script:AmeBrokerProtocolVersion -or
        $facts.MaximumFrameBytes -ne $script:AmeBrokerMaximumFrameBytes
    ) {
        throw "The broker protocol probe is incompatible with this installer"
    }
    return $facts
}

function New-AmeBrokerIdentityManifest {
    param(
        [Parameter(Mandatory = $true)]
        [string]$BinaryPath,
        [AllowEmptyString()]
        [string]$ExpectedPublisher = ""
    )

    $resolvedPath = Assert-AmeBrokerPayload `
        -Path $BinaryPath `
        -ExpectedPublisher $ExpectedPublisher
    $protocol = Get-AmeBrokerProtocolFacts -Path $resolvedPath
    $sha256 = (Get-FileHash -LiteralPath $resolvedPath -Algorithm SHA256).Hash.ToUpperInvariant()
    $manifest = "AMEJBID3|protocol=$($protocol.Protocol)|max_frame=" +
        "$($protocol.MaximumFrameBytes)|sha256=$sha256"
    Assert-AmeBrokerIdentityManifestFacts -Manifest $manifest
    return $manifest
}

function Get-AmeBrokerIdentityManifestFacts {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Manifest
    )

    $v3 = [regex]::Match(
        $Manifest,
        '\AAMEJBID3\|protocol=5\|max_frame=1048576\|sha256=([0-9A-F]{64})\z'
    )
    if ($v3.Success) {
        return [pscustomobject]@{
            Version = 3
            Sha256 = $v3.Groups[1].Value
            ExpectedPublisher = ""
            SignerSha256 = $null
        }
    }
    $match = [regex]::Match(
        $Manifest,
        '\AAMEJBID2\|protocol=(5)\|max_frame=(1048576)\|sha256=([0-9A-F]{64})' +
            '\|signer_sha256=([0-9A-F]{64})\|publisher_utf16le=([0-9A-F]+)' +
            '\|client_path_utf16le=([0-9A-F]+)\|client_sha256=([0-9A-F]{64})' +
            '\|client_signer_sha256=([0-9A-F]{64})\z',
        [System.Text.RegularExpressions.RegexOptions]::CultureInvariant
    )
    if (
        -not $match.Success -or
        [int]$match.Groups[1].Value -ne $script:AmeBrokerProtocolVersion -or
        [int]$match.Groups[2].Value -ne $script:AmeBrokerMaximumFrameBytes -or
        $match.Groups[5].Value.Length % 4 -ne 0 -or
        $match.Groups[6].Value.Length % 4 -ne 0
    ) {
        throw "The broker service identity manifest is invalid or incompatible"
    }
    $clientPathHex = $match.Groups[6].Value
    $clientPathBytes = [byte[]]::new($clientPathHex.Length / 2)
    for ($index = 0; $index -lt $clientPathBytes.Length; $index += 1) {
        $clientPathBytes[$index] = [Convert]::ToByte(
            $clientPathHex.Substring($index * 2, 2),
            16
        )
    }
    $clientPath = [Text.Encoding]::Unicode.GetString($clientPathBytes)
    if (
        $clientPath.IndexOf([char]0) -ge 0 -or
        -not $clientPath.Equals(
            (Get-AmeApplicationInstalledBinaryPath),
            [System.StringComparison]::OrdinalIgnoreCase
        )
    ) {
        throw "The broker service identity manifest does not name the fixed Ame client"
    }
    $publisherHex = $match.Groups[5].Value
    $publisherBytes = [byte[]]::new($publisherHex.Length / 2)
    for ($index = 0; $index -lt $publisherBytes.Length; $index += 1) {
        $publisherBytes[$index] = [Convert]::ToByte($publisherHex.Substring($index * 2, 2), 16)
    }
    $publisher = [Text.UnicodeEncoding]::new($false, $false, $true).GetString($publisherBytes)
    if ([string]::IsNullOrWhiteSpace($publisher) -or $publisher.IndexOf([char]0) -ge 0) {
        throw "The legacy broker manifest lacks its publisher identity"
    }
    Assert-AmeBrokerPublisherPolicy -ExpectedPublisher $publisher
    return [pscustomobject]@{
        Version = 2
        Sha256 = $match.Groups[3].Value
        ExpectedPublisher = $publisher
        SignerSha256 = $match.Groups[4].Value
    }
}

function Assert-AmeBrokerIdentityManifestFacts {
    param([Parameter(Mandatory = $true)][string]$Manifest)

    Get-AmeBrokerIdentityManifestFacts -Manifest $Manifest | Out-Null
}

function Assert-AmeBrokerManifestBinary {
    param(
        [Parameter(Mandatory = $true)]
        [string]$BinaryPath,
        [Parameter(Mandatory = $true)]
        [string]$Manifest
    )

    $identity = Get-AmeBrokerIdentityManifestFacts -Manifest $Manifest
    Assert-AmeBrokerPayload `
        -Path $BinaryPath `
        -ExpectedPublisher $identity.ExpectedPublisher | Out-Null
    $sha256 = (Get-FileHash -LiteralPath $BinaryPath -Algorithm SHA256).Hash
    if ($sha256 -cne $identity.Sha256) {
        throw "The installed broker identity does not match the protected service manifest"
    }
    if ($identity.Version -eq 2) {
        $certificate = (Get-AuthenticodeSignature -LiteralPath $BinaryPath).SignerCertificate
        $hasher = [Security.Cryptography.SHA256]::Create()
        try {
            $signerHash = ConvertTo-AmeBrokerUpperHex -Bytes ($hasher.ComputeHash($certificate.RawData))
        } finally {
            $hasher.Dispose()
        }
        if ($signerHash -cne $identity.SignerSha256) {
            throw "The legacy broker certificate does not match its protected service manifest"
        }
    }
}

function Get-AmeBrokerRecoveryPublisher {
    param([Parameter(Mandatory = $true)][psobject]$State)

    if ([int]$State.Schema -eq 2) {
        return [string]$State.ExpectedPublisher
    }
    if ([int]$State.Schema -eq 3) {
        # New payload policy is checked before its immutable facts enter the transaction.
        # Recovery compares those facts; the previous payload keeps its own manifest policy.
        return ""
    }
    throw "The broker transaction schema is unsupported"
}

function Test-AmeBrokerTransactionServiceIdentity {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$State,
        [Parameter(Mandatory = $true)]
        [psobject]$BinaryFacts,
        [Parameter(Mandatory = $true)]
        [string]$Manifest
    )

    $isPrevious = Test-AmeBrokerBinaryPayloadFactsMatch `
        -Expected $State.ExpectedPreviousBinary -Actual $BinaryFacts
    $isNew = Test-AmeBrokerBinaryPayloadFactsMatch `
        -Expected $State.ExpectedNewBinary -Actual $BinaryFacts
    return ($isPrevious -and $Manifest -ceq [string]$State.PreviousIdentityManifest) -or
        ($isNew -and $Manifest -ceq [string]$State.NewIdentityManifest)
}
