function ConvertTo-AmeBridgeCode {
    param(
        [Parameter(Mandatory = $true)][string]$Source,
        [Parameter(Mandatory = $true)][ValidateSet("Rust", "Dart")][string]$Language
    )

    # Preserve offsets while excluding comments and literals from structural evidence.
    $code = $Source.ToCharArray()
    for ($index = 0; $index -lt $code.Length; $index++) {
        $start = $index
        if ($index + 1 -lt $code.Length -and $code[$index] -eq '/' -and $code[$index + 1] -eq '/') {
            $end = $Source.IndexOf("`n", $index)
            if ($end -lt 0) { $end = $code.Length }
        } elseif ($index + 1 -lt $code.Length -and $code[$index] -eq '/' -and $code[$index + 1] -eq '*') {
            $depth = 1
            $end = $index + 2
            while ($depth -gt 0 -and $end -lt $code.Length - 1) {
                $pair = $Source.Substring($end, 2)
                if ($pair -eq "/*") { $depth++; $end += 2 }
                elseif ($pair -eq "*/") { $depth--; $end += 2 }
                else { $end++ }
            }
            if ($depth -ne 0) { throw "Unterminated bridge source comment" }
        } elseif ($code[$index] -eq '"' -or $code[$index] -eq "'") {
            $quote = [string]$code[$index]
            if ($Language -eq "Rust" -and $quote -eq "'") {
                $character = [regex]::Match(
                    $Source.Substring($index), "^'(?:[^'\\`r`n]|\\(?:u\{[0-9a-fA-F]+\}|x[0-9a-fA-F]{2}|.))'"
                )
                if (-not $character.Success) { continue }
                $end = $index + $character.Length
            } else {
                $delimiter = $quote
                $raw = $false
                if ($Language -eq "Rust") {
                    $prefix = $index - 1
                    while ($prefix -ge 0 -and $Source[$prefix] -eq '#') { $prefix-- }
                    $raw = $prefix -ge 0 -and $Source[$prefix] -eq 'r'
                    if ($raw) { $delimiter += '#' * ($index - $prefix - 1) }
                } else {
                    $raw = $index -gt 0 -and $Source[$index - 1] -eq 'r'
                    if ($index + 2 -lt $code.Length -and $Source.Substring($index, 3) -eq $quote * 3) {
                        $delimiter = $quote * 3
                    }
                }
                $end = $index + $(if ($delimiter.Length -eq 3 -and $Language -eq "Dart") { 3 } else { 1 })
                while ($true) {
                    if ($end -ge $code.Length) { throw "Unterminated bridge source literal" }
                    if (-not $raw -and $Source[$end] -eq '\') { $end += 2; continue }
                    if ($end + $delimiter.Length -le $code.Length -and
                        [string]::CompareOrdinal($Source, $end, $delimiter, 0, $delimiter.Length) -eq 0) {
                        $end += $delimiter.Length
                        break
                    }
                    $end++
                }
            }
        } else { continue }
        for ($masked = $start; $masked -lt $end; $masked++) {
            if ($code[$masked] -ne "`r" -and $code[$masked] -ne "`n") { $code[$masked] = ' ' }
        }
        $index = $end - 1
    }
    return -join $code
}

function Get-AmeBridgeClosingDelimiter {
    param([string]$Code, [int]$Start, [char]$Open, [char]$Close)

    if ($Code[$Start] -ne $Open) { throw "Bridge declaration has no opening delimiter" }
    $depth = 0
    for ($index = $Start; $index -lt $Code.Length; $index++) {
        if ($Code[$index] -eq $Open) { $depth++ }
        elseif ($Code[$index] -eq $Close) {
            $depth--
            if ($depth -eq 0) { return $index }
        }
    }
    throw "Unterminated bridge declaration"
}

function Get-AmeBridgeMethod {
    param(
        [string]$Code,
        [string]$Name,
        [ValidateSet("RustApi", "RustWire", "DartApi", "DartGenerated")][string]$Kind,
        [string]$Owner = ""
    )

    if ($Owner) {
        $ownerPattern = if ($Kind -eq "RustApi") {
            '\bimpl\s+{0}\s*\{{' -f [regex]::Escape($Owner)
        } else {
            '\bclass\s+{0}\b[^{{;]*\{{' -f [regex]::Escape($Owner)
        }
        $owners = [regex]::Matches($Code, $ownerPattern)
        if ($owners.Count -ne 1) { throw "Expected one bridge owner $Owner; found $($owners.Count)" }
        $start = $owners[0].Index + $owners[0].Length - 1
        $end = Get-AmeBridgeClosingDelimiter $Code $start '{' '}'
        $Code = $Code.Substring($start + 1, $end - $start - 1)
    }
    $prefix = switch ($Kind) {
        RustApi { '\bpub\s+(?:async\s+)?fn\s+' }
        RustWire { '(?m)^\s*fn\s+' }
        DartApi { '(?m)^\s*(?:Future\s*<[^;{}\r\n]+>|[\w?]+)\s+' }
        DartGenerated { '@override\s+(?:Future\s*<[^;{}\r\n]+>|[\w?]+)\s+' }
    }
    $matches = [regex]::Matches($Code, $prefix + [regex]::Escape($Name) + '\s*\(')
    if ($matches.Count -ne 1) { throw "Expected one $Kind method $Name; found $($matches.Count)" }
    $declaration = $matches[0]
    $parametersEnd = Get-AmeBridgeClosingDelimiter $Code ($declaration.Index + $declaration.Length - 1) '(' ')'
    $bodyStart = $parametersEnd + 1
    while ($bodyStart -lt $Code.Length -and [char]::IsWhiteSpace($Code[$bodyStart])) { $bodyStart++ }
    if ($Kind.StartsWith("Rust")) {
        while ($bodyStart -lt $Code.Length -and $Code[$bodyStart] -ne '{' -and $Code[$bodyStart] -ne ';') { $bodyStart++ }
    }
    $attributes = ""
    if ($Kind -eq "RustApi") {
        $before = $Code.Substring(0, $declaration.Index)
        $attributes = [regex]::Match($before, '(?s)(?:#\[[^\]]*\]\s*)+$').Value
    }
    if ($bodyStart -ge $Code.Length) { throw "Missing body for bridge method $Name" }
    if ($Code[$bodyStart] -eq '{') {
        $end = Get-AmeBridgeClosingDelimiter $Code $bodyStart '{' '}'
    } elseif ($Kind -eq "DartApi" -and $Code[$bodyStart] -eq ';') {
        $end = $bodyStart
    } elseif ($Kind -eq "DartApi" -and $Code.Substring($bodyStart).StartsWith("=>")) {
        $end = $Code.IndexOf(';', $bodyStart)
        if ($end -lt 0) { throw "Unterminated expression for bridge method $Name" }
    } else { throw "Unsupported body for bridge method $Name" }
    return [pscustomobject]@{
        Signature = $Code.Substring($declaration.Index, $parametersEnd - $declaration.Index + 1)
        Attributes = $attributes
        Body = $Code.Substring($bodyStart, $end - $bodyStart + 1)
    }
}
