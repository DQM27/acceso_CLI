param(
    [string]$Source = (Join-Path $PSScriptRoot "..\assets\icon-master.png")
)

$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$sourcePath = [System.IO.Path]::GetFullPath($Source)
$icoPath = Join-Path $repoRoot "assets\icon.ico"
$msixAssetsPath = Join-Path $repoRoot "packaging\msix\Assets"

if (-not (Test-Path -LiteralPath $sourcePath -PathType Leaf)) {
    throw "No se encontro la imagen maestra: $sourcePath"
}

function New-RoundedIconBitmap {
    param(
        [Parameter(Mandatory)]
        [System.Drawing.Image]$Image,

        [Parameter(Mandatory)]
        [int]$Size
    )

    $bitmap = [System.Drawing.Bitmap]::new(
        $Size,
        $Size,
        [System.Drawing.Imaging.PixelFormat]::Format32bppArgb
    )
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $path = [System.Drawing.Drawing2D.GraphicsPath]::new()

    try {
        $graphics.Clear([System.Drawing.Color]::Transparent)
        $graphics.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceCopy
        $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
        $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
        $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias

        # La imagen generada ya dibuja un mosaico redondeado sobre negro. Este
        # recorte convierte sus esquinas negras en transparencia real para que
        # el icono se vea bien tanto en el tema claro como en el oscuro.
        $radius = [single]($Size * 0.115)
        $diameter = [single]($radius * 2)
        $edge = [single]$Size

        $path.AddArc(0, 0, $diameter, $diameter, 180, 90)
        $path.AddArc($edge - $diameter, 0, $diameter, $diameter, 270, 90)
        $path.AddArc(
            $edge - $diameter,
            $edge - $diameter,
            $diameter,
            $diameter,
            0,
            90
        )
        $path.AddArc(0, $edge - $diameter, $diameter, $diameter, 90, 90)
        $path.CloseFigure()

        $graphics.SetClip($path)
        $destination = [System.Drawing.Rectangle]::new(0, 0, $Size, $Size)
        $graphics.DrawImage(
            $Image,
            $destination,
            0,
            0,
            $Image.Width,
            $Image.Height,
            [System.Drawing.GraphicsUnit]::Pixel
        )
    }
    catch {
        $bitmap.Dispose()
        throw
    }
    finally {
        $path.Dispose()
        $graphics.Dispose()
    }

    return $bitmap
}

function Export-IconPng {
    param(
        [Parameter(Mandatory)]
        [System.Drawing.Image]$Image,

        [Parameter(Mandatory)]
        [int]$Size,

        [Parameter(Mandatory)]
        [string]$Path
    )

    $bitmap = New-RoundedIconBitmap -Image $Image -Size $Size
    try {
        $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
    }
    finally {
        $bitmap.Dispose()
    }
}

function Export-MultiResolutionIco {
    param(
        [Parameter(Mandatory)]
        [System.Drawing.Image]$Image,

        [Parameter(Mandatory)]
        [int[]]$Sizes,

        [Parameter(Mandatory)]
        [string]$Path
    )

    $frames = foreach ($size in $Sizes) {
        $bitmap = New-RoundedIconBitmap -Image $Image -Size $size
        $stream = [System.IO.MemoryStream]::new()

        try {
            $bitmap.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
            [PSCustomObject]@{
                Size = $size
                Bytes = $stream.ToArray()
            }
        }
        finally {
            $stream.Dispose()
            $bitmap.Dispose()
        }
    }

    $output = [System.IO.MemoryStream]::new()
    $writer = [System.IO.BinaryWriter]::new($output)

    try {
        $writer.Write([uint16]0) # Reservado.
        $writer.Write([uint16]1) # Tipo 1 = icono.
        $writer.Write([uint16]$frames.Count)

        $offset = 6 + (16 * $frames.Count)
        foreach ($frame in $frames) {
            $dimension = if ($frame.Size -eq 256) { 0 } else { $frame.Size }
            $writer.Write([byte]$dimension)
            $writer.Write([byte]$dimension)
            $writer.Write([byte]0) # Sin paleta indexada.
            $writer.Write([byte]0)
            $writer.Write([uint16]1)
            $writer.Write([uint16]32)
            $writer.Write([uint32]$frame.Bytes.Length)
            $writer.Write([uint32]$offset)
            $offset += $frame.Bytes.Length
        }

        foreach ($frame in $frames) {
            $writer.Write($frame.Bytes)
        }

        $writer.Flush()
        [System.IO.File]::WriteAllBytes($Path, $output.ToArray())
    }
    finally {
        $writer.Dispose()
        $output.Dispose()
    }
}

$sourceImage = [System.Drawing.Image]::FromFile($sourcePath)
try {
    if ($sourceImage.Width -ne $sourceImage.Height) {
        throw "La imagen maestra debe ser cuadrada; mide $($sourceImage.Width)x$($sourceImage.Height)."
    }

    $pngTargets = [ordered]@{
        "Square44x44Logo.png" = 44
        "StoreLogo.png" = 50
        "Square71x71Logo.png" = 71
        "Square150x150Logo.png" = 150
    }

    foreach ($target in $pngTargets.GetEnumerator()) {
        Export-IconPng `
            -Image $sourceImage `
            -Size $target.Value `
            -Path (Join-Path $msixAssetsPath $target.Key)
    }

    Export-MultiResolutionIco `
        -Image $sourceImage `
        -Sizes @(16, 32, 48, 64, 128, 256) `
        -Path $icoPath
}
finally {
    $sourceImage.Dispose()
}

Write-Host "Iconos generados desde $sourcePath"
