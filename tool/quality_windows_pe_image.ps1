function Assert-AmeWindowsPeImage {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][ValidateSet('x64', 'arm64')][string]$Architecture
    )

    $machine = if ($Architecture -eq 'arm64') { 0xAA64 } else { 0x8664 }

    $stream = [System.IO.File]::OpenRead($Path)
    try {
        $reader = [System.IO.BinaryReader]::new($stream)
        try {
            if ($stream.Length -lt 90 -or $reader.ReadUInt16() -ne 0x5A4D) {
                throw "Unsigned runtime payload is not a complete PE image"
            }
            $stream.Position = 0x3C
            $offset = $reader.ReadUInt32()
            if ($offset -lt 64 -or $offset -gt $stream.Length - 26) {
                throw "Unsigned runtime payload has an invalid PE header offset"
            }
            $stream.Position = $offset
            if ($reader.ReadUInt32() -ne 0x4550 -or $reader.ReadUInt16() -ne $machine) {
                throw "Unsigned runtime payload must be a Windows $Architecture PE image: $Path"
            }
            $stream.Position = $offset + 20
            $optionalHeaderSize = $reader.ReadUInt16()
            if ($optionalHeaderSize -lt 2 -or $offset + 24 + $optionalHeaderSize -gt $stream.Length) {
                throw "Unsigned runtime payload has an incomplete optional PE header"
            }
            $stream.Position = $offset + 24
            if ($reader.ReadUInt16() -ne 0x20B) {
                throw "Unsigned runtime payload must use the PE32+ image format"
            }
        } finally {
            $reader.Dispose()
        }
    } finally {
        $stream.Dispose()
    }
}
