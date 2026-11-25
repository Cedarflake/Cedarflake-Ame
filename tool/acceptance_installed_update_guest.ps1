$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ([Security.Principal.WindowsIdentity]::GetCurrent().Name.Split('\')[-1] -cne 'WDAGUtilityAccount') {
    throw 'Installed update acceptance requires the disposable Sandbox controller'
}
$inputRoot = 'C:\AmeInput'
$outputRoot = 'C:\AmeResults'
$utf8 = [Text.UTF8Encoding]::new($false)
function Publish-GuestJson([string]$Name, $Value) {
    $path = Join-Path $outputRoot $Name
    [IO.File]::WriteAllText(($path + '.tmp'), ($Value | ConvertTo-Json -Depth 8), $utf8)
    Move-Item -LiteralPath ($path + '.tmp') -Destination $path -Force
}
function Invoke-GuestInstaller([string]$Phase, [string[]]$Arguments) {
    Publish-GuestJson 'progress.json' @{ phase = $Phase }
    $child = Start-Process -FilePath (Join-Path $env:SystemRoot 'System32\msiexec.exe') `
        -ArgumentList ($Arguments + @('/qn', '/norestart', '/l*v', "C:\AmeResults\$Phase.log")) -WindowStyle Hidden -PassThru
    try {
        if (-not $child.WaitForExit(240000)) { throw "Guest installer did not retire: $Phase" }
        if ($child.ExitCode -notin @(0, 3010)) { throw "Guest installer returned $($child.ExitCode): $Phase" }
    } finally { $child.Dispose() }
}

$guest = $null
$administrator = $null
$finalResult = $null
$exitCode = 0
try {
    $plan = Get-Content -LiteralPath (Join-Path $inputRoot 'plan.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    foreach ($file in $plan.files) {
        $path = Join-Path $inputRoot $file.name
        if ((Get-Item -LiteralPath $path).Length -ne $file.length -or
            (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ine $file.sha256) { throw 'MSI fixture input differs' }
    }
    . (Join-Path $inputRoot 'user.ps1')
    $guest = New-AmeGuestUser -InputRoot $inputRoot -OutputRoot $outputRoot
    $rules = @((Get-Acl -LiteralPath 'C:\').GetAccessRules($true, $false, [Security.Principal.SecurityIdentifier]) |
        Where-Object { $_.IdentityReference.Value -eq 'S-1-5-11' -and $_.InheritanceFlags -eq 'None' -and $_.AccessControlType -eq 'Allow' })
    if ($rules.Count -eq 1 -and [int]$rules[0].FileSystemRights -eq 0x1301bf) {
        & (Join-Path $guest.InputRoot 'volume.ps1')
    }
    Publish-GuestJson 'progress.json' @{ phase = 'baseline-prerequisite' }
    $runtime = Start-Process -FilePath (Join-Path $guest.InputRoot 'vc_redist.exe') `
        -ArgumentList '/install /quiet /norestart /log C:\AmeResults\runtime.log' -WindowStyle Hidden -PassThru
    try {
        if (-not $runtime.WaitForExit(600000) -or $runtime.ExitCode -notin @(0, 3010)) { throw 'Guest baseline runtime failed' }
    } finally { $runtime.Dispose() }
    Invoke-GuestInstaller 'baseline-install' @('/i', (Join-Path $guest.InputRoot 'baseline.msi'))
    $directory = (Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Cedarflake\Ame\Installer').InstallFolder
    if ($directory.TrimEnd('\') -ine (Join-Path $env:ProgramFiles 'Cedarflake Ame\Application')) { throw 'Unexpected registered guest application path' }
    $application = Join-Path $directory 'cedarflake_ame.exe'
    if ((Get-Item -LiteralPath $application).VersionInfo.ProductVersion -cne '0.1.0-alpha.3+3') { throw 'Unexpected guest baseline version' }
    Copy-Item -LiteralPath (Join-Path $guest.InputRoot 'parent.exe') -Destination $application -Force
    Copy-Item -LiteralPath (Join-Path $guest.InputRoot 'helper.exe') -Destination (Join-Path $directory 'cedarflake_ame_update.exe') -Force

    Add-LocalGroupMember -SID 'S-1-5-32-544' -Member $guest.Account
    $probe = Start-Process -FilePath (Join-Path $guest.InputRoot 'parent.exe') -ArgumentList '--inspect-token 0' `
        -Credential $guest.Credential -LoadUserProfile -WorkingDirectory $guest.InputRoot -WindowStyle Hidden `
        -RedirectStandardOutput (Join-Path $guest.OutputRoot 'admin-token.json') -PassThru
    try {
        if (-not $probe.WaitForExit(15000) -or $probe.ExitCode -ne 0) { throw 'Fresh guest admin token inspection failed' }
    } finally { $probe.Dispose() }
    $token = Get-Content -LiteralPath (Join-Path $guest.OutputRoot 'admin-token.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    $policy = Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System' |
        Select-Object EnableLUA, ConsentPromptBehaviorAdmin, PromptOnSecureDesktop
    Publish-GuestJson 'elevation-policy.json' @{ policy = $policy; freshAdministratorToken = $token }
    if ($token.elevated -or -not $token.linked) {
        Remove-LocalGroupMember -SID 'S-1-5-32-544' -Member $guest.Account
        $administrator = 'AmeAdmin' + [guid]::NewGuid().ToString('N').Substring(0, 8)
        $password = [guid]::NewGuid().ToString('N') + 'aA!9'
        New-LocalUser -Name $administrator -Password (ConvertTo-SecureString $password -AsPlainText -Force) `
            -AccountNeverExpires -PasswordNeverExpires | Out-Null
        Add-LocalGroupMember -SID 'S-1-5-32-544' -Member $administrator
        Publish-GuestJson 'elevation-credential.json' @{ user = '.\' + $administrator; password = $password }
    }
    Start-AmeGuestWorker -Guest $guest -ScriptName 'worker.ps1'
    $deadline = [DateTime]::UtcNow.AddMinutes(12)
    while (-not $guest.Process.HasExited) {
        $retained = Test-Path -LiteralPath (Join-Path $outputRoot 'retain-for-user')
        if ($retained) {
            [IO.File]::WriteAllText((Join-Path $guest.OutputRoot 'retain-for-user'), 'pending explicit user action', $utf8)
            $deadline = [DateTime]::UtcNow.AddMinutes(12)
        }
        if ([DateTime]::UtcNow -ge $deadline) { throw 'Guest update worker is still waiting for completion or UI inspection' }
        $progress = Join-Path $guest.OutputRoot 'progress.json'
        if (Test-Path -LiteralPath $progress) {
            Publish-GuestJson 'progress.json' (Get-Content -LiteralPath $progress -Raw -Encoding UTF8 | ConvertFrom-Json)
        }
        if (Test-Path -LiteralPath (Join-Path $outputRoot 'ui-verified')) {
            Copy-Item -LiteralPath (Join-Path $outputRoot 'ui-verified') -Destination (Join-Path $guest.OutputRoot 'ui-verified') -Force
        }
        Start-Sleep -Milliseconds 200
    }
    $result = Read-AmeGuestWorkerResult -Guest $guest
    if ($result.status -cne 'passed') { throw $result.error }
    Invoke-GuestInstaller 'updated-uninstall' @('/x', (Join-Path $guest.InputRoot 'target.msi'))
    if (Get-Service CedarflakeAmeJournalBroker -ErrorAction SilentlyContinue) { throw 'Updated guest service remained after uninstall' }
    $result | Add-Member -NotePropertyName uninstalled -NotePropertyValue $true
    $finalResult = $result
} catch {
    $finalResult = @{ format = 'ame-installed-update-acceptance-v1'; status = 'failed'; error = $_.Exception.Message }
    $exitCode = 1
} finally {
    if ($null -ne $guest) { Remove-AmeGuestUser -Guest $guest }
    if ($null -ne $administrator) { Remove-LocalUser -Name $administrator }
}
Publish-GuestJson 'result.json' $finalResult
exit $exitCode
