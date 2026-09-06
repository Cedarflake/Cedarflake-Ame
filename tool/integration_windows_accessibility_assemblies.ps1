function Assert-AmeWindowsUiaAssemblyIdentity {
    param(
        [Parameter(Mandatory = $true)] [System.Reflection.AssemblyName]$Identity,
        [Parameter(Mandatory = $true)]
        [ValidateSet("UIAutomationClient", "UIAutomationTypes")] [string]$Name
    )

    $token = [System.BitConverter]::ToString($Identity.GetPublicKeyToken())
    if (
        $Identity.Name -cne $Name -or $token -cne "31-BF-38-56-AD-36-4E-35" -or
        -not [string]::IsNullOrEmpty($Identity.CultureName)
    ) {
        throw "Windows UIA assembly identity does not match the fixed system contract"
    }
}

function Assert-AmeWindowsUiaAssemblyTypes {
    param(
        [Parameter(Mandatory = $true)] [System.Reflection.Assembly]$Assembly,
        [Parameter(Mandatory = $true)] [string[]]$RequiredTypes
    )

    foreach ($name in $RequiredTypes) {
        $type = $Assembly.GetType($name, $true, $false)
        if (-not [object]::ReferenceEquals($type.Assembly, $Assembly)) {
            throw "Windows UIA required type is not owned by its verified assembly"
        }
    }
}

function Import-AmeWindowsUiaAssembly {
    param(
        [Parameter(Mandatory = $true)]
        [ValidateSet("UIAutomationClient", "UIAutomationTypes")] [string]$Name
    )

    # Assembly.Load uses the runtime binder, without Add-Type's command initialization
    # or its fallback to a same-named DLL in the current working directory.
    $identity = [System.Reflection.AssemblyName]::new(
        "$Name, Version=4.0.0.0, Culture=neutral, PublicKeyToken=31bf3856ad364e35"
    )
    $assembly = [System.Reflection.Assembly]::Load($identity)
    Assert-AmeWindowsUiaAssemblyIdentity -Identity $assembly.GetName() -Name $Name
    $requiredTypes = if ($Name -ceq "UIAutomationClient") {
        @(
            "System.Windows.Automation.AutomationElement",
            "System.Windows.Automation.CacheRequest",
            "System.Windows.Automation.PropertyCondition",
            "System.Windows.Automation.TreeWalker"
        )
    } else {
        @("System.Windows.Automation.ControlType", "System.Windows.Automation.TreeScope")
    }
    Assert-AmeWindowsUiaAssemblyTypes -Assembly $assembly -RequiredTypes $requiredTypes
}
