$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Get-AmeMsixAssetDefinitions {
    foreach ($asset in @(
        @{ name = 'StoreLogo'; size = 50 },
        @{ name = 'Square44x44Logo'; size = 44 },
        @{ name = 'Square150x150Logo'; size = 150 }
    )) {
        @{ name = "$($asset.name).png"; size = $asset.size }
        foreach ($scale in @(100, 125, 150, 200, 400)) {
            @{ name = "$($asset.name).scale-$scale.png"; size = [int][Math]::Ceiling($asset.size * $scale / 100) }
        }
    }
    foreach ($size in @(16, 20, 24, 30, 32, 36, 40, 48, 60, 64, 72, 80, 96, 256)) {
        foreach ($theme in @('', '_altform-unplated', '_altform-lightunplated')) {
            @{ name = "Square44x44Logo.targetsize-$size$theme.png"; size = $size }
        }
    }
}

function New-AmeMsixAssets {
    param([Parameter(Mandatory)][string]$Directory)
    Add-Type -AssemblyName System.Drawing
    [void][IO.Directory]::CreateDirectory($Directory)
    $icon = [Drawing.Icon]::new((Join-Path $PSScriptRoot '../windows/runner/resources/app_icon.ico'), 256, 256)
    try {
        $source = $icon.ToBitmap()
        try {
            foreach ($asset in Get-AmeMsixAssetDefinitions) {
                $bitmap = [Drawing.Bitmap]::new($asset.size, $asset.size)
                $graphics = [Drawing.Graphics]::FromImage($bitmap)
                try {
                    $graphics.Clear([Drawing.Color]::Transparent)
                    $graphics.InterpolationMode = [Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
                    $graphics.DrawImage($source, 0, 0, $asset.size, $asset.size)
                    $bitmap.Save((Join-Path $Directory $asset.name), [Drawing.Imaging.ImageFormat]::Png)
                } finally { $graphics.Dispose(); $bitmap.Dispose() }
            }
        } finally { $source.Dispose() }
    } finally { $icon.Dispose() }
}

function New-AmeMsixResourceIndex {
    param([Parameter(Mandatory)][string]$Root, [Parameter(Mandatory)][string]$ConfigurationPath)
    $makePri = Get-AmeMsixSdkTool 'makepri.exe'
    Invoke-AmeChecked $makePri @('createconfig', '/cf', $ConfigurationPath, '/dq', 'en-US') |
        ForEach-Object { $_ -replace "`0", '' } | Out-Host
    $configuration = [xml](Get-Content -LiteralPath $ConfigurationPath -Raw -Encoding UTF8)
    [void]$configuration.resources.RemoveChild($configuration.resources.packaging)
    $settings = [Xml.XmlWriterSettings]::new()
    $settings.Encoding = [Text.UTF8Encoding]::new($false)
    $writer = [Xml.XmlWriter]::Create($ConfigurationPath, $settings)
    try { $configuration.Save($writer) } finally { $writer.Dispose() }
    Invoke-AmeChecked $makePri @('new', '/pr', $Root, '/cf', $ConfigurationPath, '/of', (Join-Path $Root 'resources.pri')) |
        ForEach-Object { $_ -replace "`0", '' } | Out-Host
}
