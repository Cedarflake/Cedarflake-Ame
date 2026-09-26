$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "integration_windows_accessibility_assemblies.ps1")

function Assert-AmeUiaAssemblyRefusal {
    param([scriptblock]$Action)

    $refused = $false
    try {
        & $Action
    } catch {
        $refused = $true
    }
    if (-not $refused) {
        throw "Windows UIA assembly guardrail accepted an invalid contract"
    }
}

$loadElapsed = [System.Diagnostics.Stopwatch]::StartNew()
Import-AmeWindowsUiaAssembly -Name "UIAutomationTypes"
$typesMilliseconds = $loadElapsed.ElapsedMilliseconds
Import-AmeWindowsUiaAssembly -Name "UIAutomationClient"
$totalMilliseconds = $loadElapsed.ElapsedMilliseconds
$actualAssembly = [System.Windows.Automation.AutomationElement].Assembly
Assert-AmeWindowsUiaAssemblyIdentity -Identity $actualAssembly.GetName() -Name "UIAutomationClient"

foreach ($invalidIdentity in @(
    "UIAutomationClient, Version=4.0.0.0, Culture=neutral, PublicKeyToken=null",
    "UIAutomationClient, Version=4.0.0.0, Culture=neutral, PublicKeyToken=b77a5c561934e089",
    "UIAutomationTypes, Version=4.0.0.0, Culture=neutral, PublicKeyToken=31bf3856ad364e35",
    "UIAutomationClient, Version=4.0.0.0, Culture=en-US, PublicKeyToken=31bf3856ad364e35"
)) {
    Assert-AmeUiaAssemblyRefusal {
        Assert-AmeWindowsUiaAssemblyIdentity `
            -Identity ([System.Reflection.AssemblyName]::new($invalidIdentity)) `
            -Name "UIAutomationClient"
    }
}
Assert-AmeUiaAssemblyRefusal {
    Import-AmeWindowsUiaAssembly -Name "System.Private.CoreLib"
}
Assert-AmeUiaAssemblyRefusal {
    Assert-AmeWindowsUiaAssemblyTypes -Assembly $actualAssembly `
        -RequiredTypes @("System.Windows.Automation.MissingRequiredType")
}
Assert-AmeUiaAssemblyRefusal {
    Assert-AmeWindowsUiaAssemblyTypes -Assembly $actualAssembly `
        -RequiredTypes @("system.windows.automation.automationelement")
}

$loaderSource = [System.IO.File]::ReadAllText(
    (Join-Path $PSScriptRoot "integration_windows_accessibility_assemblies.ps1")
)
$tokens = $null
$parseErrors = $null
$loaderAst = [System.Management.Automation.Language.Parser]::ParseInput(
    $loaderSource, [ref]$tokens, [ref]$parseErrors
)
if ($parseErrors.Count -ne 0) {
    throw "Windows UIA assembly loader does not parse"
}
$forbiddenCommands = @($loaderAst.FindAll({
    param($node)
    $node -is [System.Management.Automation.Language.CommandAst] -and
        $node.GetCommandName() -in @("Add-Type", "Import-Module", "Get-ChildItem")
}, $true))
if ($forbiddenCommands.Count -ne 0 -or $loaderSource -match '::LoadFrom\(') {
    throw "Windows UIA assembly loading regained a compiler, module, or path fallback"
}
Write-Host (
    "AME_WINDOWS_UIA_ASSEMBLIES host=$($PSVersionTable.PSVersion) " +
    "types_ms=$typesMilliseconds total_ms=$totalMilliseconds result=ok"
)
