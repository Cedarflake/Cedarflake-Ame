$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Read-AmeInstallerRows {
    param(
        [Parameter(Mandatory = $true)]$Database,
        [Parameter(Mandatory = $true)][string]$Table,
        [Parameter(Mandatory = $true)][string[]]$Columns
    )

    foreach ($name in @($Table) + $Columns) {
        if ($name -cnotmatch '^[A-Za-z_][A-Za-z_0-9]*$') { throw "Invalid MSI query identifier" }
    }
    $select = @($Columns | ForEach-Object { '`' + $_ + '`' }) -join ', '
    $view = $Database.OpenView(('SELECT {0} FROM `{1}`' -f $select, $Table), [object[]]@())
    try {
        $view.Execute()
        $count = 0
        while ($null -ne ($record = $view.Fetch())) {
            try {
                if (++$count -gt 4096) { throw "MSI table exceeds its row bound" }
                $row = [ordered]@{}
                for ($index = 0; $index -lt $Columns.Count; $index++) {
                    $value = $record.GetString($index + 1)
                    if ($value.Length -gt 32768) { throw "MSI field exceeds its text bound" }
                    $row[$Columns[$index]] = $value
                }
                [pscustomobject]$row
            } finally { $record.Dispose() }
        }
    } finally { $view.Dispose() }
}

function Read-AmeInstallerDatabase {
    param(
        [Parameter(Mandatory = $true)][string]$PackagePath,
        [Parameter(Mandatory = $true)][string]$DtfAssemblyPath
    )

    Add-Type -Path $DtfAssemblyPath
    $database = [WixToolset.Dtf.WindowsInstaller.Database]::new(
        $PackagePath, [WixToolset.Dtf.WindowsInstaller.DatabaseOpenMode]::ReadOnly)
    try {
        $tables = @(Read-AmeInstallerRows $database '_Tables' @('Name') | ForEach-Object { $_.Name })
        $definitions = [ordered]@{
            Property = 'Property,Value'
            Directory = 'Directory,Directory_Parent,DefaultDir'
            Component = 'Component,ComponentId,Directory_,Attributes,Condition,KeyPath'
            Feature = 'Feature,Feature_Parent,Level,Attributes,Directory_'
            FeatureComponents = 'Feature_,Component_'
            File = 'File,Component_,FileName,FileSize,Attributes,Sequence'
            ServiceInstall = 'ServiceInstall,Name,ServiceType,StartType,ErrorControl,StartName,Password,Arguments,Component_,Description'
            ServiceControl = 'ServiceControl,Name,Event,Arguments,Wait,Component_'
            MsiServiceConfig = 'MsiServiceConfig,Name,Event,ConfigType,Argument,Component_'
            MsiLockPermissionsEx = 'MsiLockPermissionsEx,LockObject,Table,SDDLText,Condition'
            CustomAction = 'Action,Type,Source,Target'
            InstallExecuteSequence = 'Action,Condition,Sequence'
            InstallUISequence = 'Action,Condition,Sequence'
            LaunchCondition = 'Condition,Description'
            Upgrade = 'UpgradeCode,VersionMin,VersionMax,Attributes,Remove,ActionProperty'
            Media = 'DiskId,LastSequence,Cabinet'
            RemoveFile = 'FileKey,Component_,FileName,DirProperty,InstallMode'
            Shortcut = 'Shortcut,Directory_,Name,Component_,Target,Arguments,WkDir'
            Registry = 'Registry,Root,Key,Name,Value,Component_'
            Dialog = 'Dialog,Control_First,Control_Default,Control_Cancel'
            Control = 'Dialog_,Control,Type,Attributes,Property,Text,Control_Next'
            CheckBox = 'Property,Value'
            ControlEvent = 'Dialog_,Control_,Event,Argument,Condition,Ordering'
        }
        $model = [ordered]@{ Tables = $tables; Template = $database.SummaryInfo.Template }
        foreach ($name in $definitions.Keys) {
            $model[$name] = @()
            if ($tables -ccontains $name) {
                $model[$name] = @(Read-AmeInstallerRows $database $name $definitions[$name].Split(','))
            }
        }
        $view = $database.OpenView("SELECT ``Data`` FROM ``Binary`` WHERE ``Name`` = 'AmeInstallerAdmission'", [object[]]@())
        try {
            $view.Execute()
            $record = $view.Fetch()
            if ($null -eq $record) { throw "MSI is missing its native admission module" }
            try {
                $stream = $record.GetStream(1)
                $sha256 = [Security.Cryptography.SHA256]::Create()
                try {
                    if ($stream.Length -le 0 -or $stream.Length -gt 4MB) { throw "MSI native admission module exceeds its bound" }
                    $model.AdmissionHash = -join @($sha256.ComputeHash($stream) | ForEach-Object { $_.ToString('X2') })
                } finally { $sha256.Dispose(); $stream.Dispose() }
            } finally { $record.Dispose() }
        } finally { $view.Dispose() }
        return [pscustomobject]$model
    } finally { $database.Dispose() }
}
