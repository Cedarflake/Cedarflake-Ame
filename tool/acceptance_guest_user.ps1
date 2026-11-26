$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ([Security.Principal.WindowsIdentity]::GetCurrent().Name.Split('\')[-1] -cne 'WDAGUtilityAccount') {
    throw 'Guest account orchestration requires the disposable Sandbox controller'
}

function New-AmeGuestUser {
    param([Parameter(Mandatory)][string]$InputRoot, [Parameter(Mandatory)][string]$OutputRoot)

    if ($InputRoot -cne 'C:\AmeInput' -or $OutputRoot -cne 'C:\AmeResults') { throw 'Unexpected mapped guest directories' }
    $account = 'AmeFixture' + [guid]::NewGuid().ToString('N').Substring(0, 8)
    $securePassword = ConvertTo-SecureString ([guid]::NewGuid().ToString('N') + 'aA!9') -AsPlainText -Force
    $guestInput = 'C:\AmeFixtureInput'
    $guestOutput = 'C:\AmeFixtureResults'
    foreach ($directory in @($guestInput, $guestOutput)) {
        if (Test-Path -LiteralPath $directory) { throw 'Guest fixture directory must be fresh' }
        [void][IO.Directory]::CreateDirectory($directory)
        $acl = Get-Acl -LiteralPath $directory
        $users = [Security.Principal.SecurityIdentifier]::new('S-1-5-32-545')
        $permission = if ($directory -ceq $guestInput) { 'ReadAndExecute' } else { 'Modify' }
        $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new(
            $users, $permission, 'ContainerInherit,ObjectInherit', 'None', 'Allow'))
        Set-Acl -LiteralPath $directory -AclObject $acl
    }
    Get-ChildItem -LiteralPath $InputRoot -File | Copy-Item -Destination $guestInput
    New-LocalUser -Name $account -Password $securePassword -AccountNeverExpires -PasswordNeverExpires | Out-Null
    try {
        Add-LocalGroupMember -SID 'S-1-5-32-545' -Member $account
        . (Join-Path $InputRoot 'desktop.ps1')
        $sid = (Get-LocalUser -Name $account).SID.Value
        $desktopAccess = [ordered]@{ sid = $sid
            stationBefore = [AmeGuestDesktop]::Inspect($false)
            desktopBefore = [AmeGuestDesktop]::Inspect($true)
            stationAfter = [AmeGuestDesktop]::Admit($false, $sid)
            desktopAfter = [AmeGuestDesktop]::Admit($true, $sid) }
        [IO.File]::WriteAllText((Join-Path $OutputRoot 'desktop-access.json'),
            ($desktopAccess | ConvertTo-Json), [Text.UTF8Encoding]::new($false))
        Start-Service -Name seclogon
        return [pscustomobject]@{ Account = $account; Process = $null; InputRoot = $guestInput
            OutputRoot = $guestOutput; ReportRoot = $OutputRoot
            Credential = [Management.Automation.PSCredential]::new('.\' + $account, $securePassword) }
    } catch {
        Remove-LocalUser -Name $account
        throw
    }
}

function Start-AmeGuestWorker {
    param([Parameter(Mandatory)][object]$Guest, [ValidatePattern('^[a-z-]+\.ps1$')][string]$ScriptName = 'guest.ps1')

    if ($null -ne $Guest.Process) { throw 'The fixture worker already started' }
    $Guest.Process = Start-Process -FilePath (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') `
        -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File C:\AmeFixtureInput\$ScriptName -Worker -InputRoot C:\AmeFixtureInput -OutputRoot C:\AmeFixtureResults" `
        -Credential $Guest.Credential -LoadUserProfile -WorkingDirectory $Guest.InputRoot -WindowStyle Hidden -PassThru
}

function Read-AmeGuestWorkerResult {
    param([Parameter(Mandatory)][object]$Guest)

    if (-not $Guest.Process.HasExited) { throw 'The fixture worker has not retired' }
    $path = Join-Path $Guest.OutputRoot 'worker-result.json'
    if (-not (Test-Path -LiteralPath $path)) { throw "Guest worker exited without a result: $($Guest.Process.ExitCode)" }
    $result = Get-Content -LiteralPath $path -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($result.status -cne 'passed') {
        try {
            $processes = @(Get-Process | Where-Object ProcessName -Like 'cedarflake_ame*' | ForEach-Object {
                @{ id = $_.Id; name = $_.ProcessName; path = $_.Path
                    modules = @($_.Modules | Select-Object -ExpandProperty ModuleName)
                    threads = @($_.Threads | ForEach-Object {
                        @{ id = $_.Id; state = [string]$_.ThreadState
                            wait = $(if ($_.ThreadState -eq 'Wait') { [string]$_.WaitReason } else { '' }) }
                    }) }
            })
            [IO.File]::WriteAllText((Join-Path $Guest.ReportRoot 'failure-processes.json'),
                (@{ processes = $processes; windows = [AmeGuestDesktop]::DescribeWindows() } | ConvertTo-Json -Depth 6),
                [Text.UTF8Encoding]::new($false))
        } catch {
            [IO.File]::WriteAllText((Join-Path $Guest.ReportRoot 'failure-diagnostic-error.txt'),
                $_.Exception.Message, [Text.UTF8Encoding]::new($false))
        }
    }
    return $result
}

function Remove-AmeGuestUser {
    param([Parameter(Mandatory)][object]$Guest)

    try {
        if ($null -eq $Guest.Process -or $Guest.Process.HasExited) { Remove-LocalUser -Name $Guest.Account }
    } finally {
        if ($null -ne $Guest.Process) { $Guest.Process.Dispose() }
    }
}
