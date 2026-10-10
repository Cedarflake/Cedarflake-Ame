function Assert-AmeWindowsEngineResults {
    param([Parameter(Mandatory = $true)][string]$JUnitXml)

    $settings = [Xml.XmlReaderSettings]::new()
    $settings.DtdProcessing = [Xml.DtdProcessing]::Prohibit
    $settings.XmlResolver = $null
    $settings.MaxCharactersInDocument = 1MB
    $text = [IO.StringReader]::new($JUnitXml)
    $reader = $null
    $document = [Xml.XmlDocument]::new()
    $document.XmlResolver = $null
    try {
        $reader = [Xml.XmlReader]::Create($text, $settings)
        $document.Load($reader)
    } finally {
        if ($null -ne $reader) { $reader.Dispose() }
        $text.Dispose()
    }
    $suite = $document.DocumentElement
    if ($null -eq $suite -or $suite.get_Name() -cne "testsuite" -or
        $suite.GetAttribute("tests") -cne "2") {
        throw "Engine lifecycle results must contain exactly two executed cases"
    }
    foreach ($attribute in @("failures", "disabled", "skipped")) {
        if ($suite.GetAttribute($attribute) -cne "0") {
            throw "Engine lifecycle results contain unsuccessful cases: $attribute"
        }
    }
    if ($suite.GetAttribute("errors") -cnotin @("", "0")) { throw "Engine lifecycle results contain errors" }
    $cases = @($document.SelectNodes("/testsuite/testcase"))
    if ($cases.Count -ne 2) { throw "Engine lifecycle case roster is incomplete or duplicated" }
    foreach ($phase in @("explicit_destroy", "scope_exit")) {
        $name = "engine_lifecycle_$phase"
        $matches = @($cases | Where-Object { $_.GetAttribute("name") -ceq $name })
        if ($matches.Count -ne 1) { throw "Engine lifecycle case must execute exactly once: $name" }
        $case = $matches[0]
        if ($case.GetAttribute("status") -cne "run" -or
            $case.SelectNodes("failure|error|skipped").Count -ne 0) {
            throw "Engine lifecycle case did not pass execution: $name"
        }
        $outputs = @($case.SelectNodes("system-out"))
        $marker = "PASS: $name engine_started=1 child_destroy_notifications=1 scope_completed=1 " +
            "hwnd_retired=1 access_violations=0 plugins_registered=0 com_released_after_scope=1"
        if ($outputs.Count -ne 1 -or
            [regex]::Matches($outputs[0].InnerText, '(?m)^PASS:[^\r\n]*\r?$').Count -ne 1 -or
            [regex]::Matches($outputs[0].InnerText, '(?m)^' + [regex]::Escape($marker) + '\r?$').Count -ne 1) {
            throw "Engine lifecycle case lacks exact native completion evidence: $name"
        }
    }
}
