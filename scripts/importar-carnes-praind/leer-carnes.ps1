# Lee el texto de cada carne PRAIND de una carpeta con el OCR de Windows
# (espanol, sin instalar nada) y lo deja en un JSON para importar.mjs.
# Las imagenes que no son PNG ni JPEG (por ejemplo .webp) se convierten a PNG
# en -Convertidos, porque el Worker solo acepta esos dos formatos.
#
# Corre en Windows PowerShell 5.1 (powershell.exe): PowerShell 7 no carga
# las clases de Windows Runtime. Archivo en ASCII a proposito: 5.1 lee sin
# BOM como ANSI.
param(
    [Parameter(Mandatory = $true)][string]$Carpeta,
    [Parameter(Mandatory = $true)][string]$Salida,
    [Parameter(Mandatory = $true)][string]$Convertidos
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$null = [Windows.Storage.StorageFile, Windows.Storage, ContentType = WindowsRuntime]
$null = [Windows.Media.Ocr.OcrEngine, Windows.Foundation, ContentType = WindowsRuntime]
$null = [Windows.Graphics.Imaging.BitmapDecoder, Windows.Graphics, ContentType = WindowsRuntime]
$null = [Windows.Graphics.Imaging.BitmapEncoder, Windows.Graphics, ContentType = WindowsRuntime]
$null = [Windows.Globalization.Language, Windows.Globalization, ContentType = WindowsRuntime]

$asTaskGenerico = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
        $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0]
$asTaskAccion = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
        $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncAction' })[0]

function Esperar($operacion, [Type]$tipo) {
    $tarea = $asTaskGenerico.MakeGenericMethod($tipo).Invoke($null, @($operacion))
    $tarea.Wait(-1) | Out-Null
    $tarea.Result
}
function EsperarAccion($accion) {
    $tarea = $asTaskAccion.Invoke($null, @($accion))
    $tarea.Wait(-1) | Out-Null
}

function ConvertirAPng($bitmap, [string]$destino) {
    $archivo = [System.IO.File]::Create($destino)
    try {
        $flujo = [System.IO.WindowsRuntimeStreamExtensions]::AsRandomAccessStream($archivo)
        $codificador = Esperar ([Windows.Graphics.Imaging.BitmapEncoder]::CreateAsync([Windows.Graphics.Imaging.BitmapEncoder]::PngEncoderId, $flujo)) ([Windows.Graphics.Imaging.BitmapEncoder])
        $codificador.SetSoftwareBitmap($bitmap)
        EsperarAccion ($codificador.FlushAsync())
    }
    finally { $archivo.Dispose() }
}

$motor = [Windows.Media.Ocr.OcrEngine]::TryCreateFromLanguage([Windows.Globalization.Language]::new('es-MX'))
if (-not $motor) { throw 'Windows no tiene el OCR en espanol (es-MX) instalado.' }
New-Item -ItemType Directory -Force $Convertidos | Out-Null

$resultados = New-Object System.Collections.ArrayList
Get-ChildItem $Carpeta -File | Where-Object { $_.Extension -match '^\.(png|jpe?g|webp)$' } | ForEach-Object {
    $item = $_
    try {
        $archivo = Esperar ([Windows.Storage.StorageFile]::GetFileFromPathAsync($item.FullName)) ([Windows.Storage.StorageFile])
        $flujo = Esperar ($archivo.OpenAsync([Windows.Storage.FileAccessMode]::Read)) ([Windows.Storage.Streams.IRandomAccessStream])
        try {
            $decodificador = Esperar ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($flujo)) ([Windows.Graphics.Imaging.BitmapDecoder])
            $bitmap = Esperar ($decodificador.GetSoftwareBitmapAsync()) ([Windows.Graphics.Imaging.SoftwareBitmap])
            $texto = Esperar ($motor.RecognizeAsync($bitmap)) ([Windows.Media.Ocr.OcrResult])
            $imagen = $item.FullName
            if ($item.Extension -notmatch '^\.(png|jpe?g)$') {
                $imagen = Join-Path $Convertidos ($item.BaseName + '.png')
                ConvertirAPng $bitmap $imagen
            }
            [void]$resultados.Add([pscustomobject]@{
                    archivo    = $item.Name
                    imagen     = $imagen
                    modificado = $item.LastWriteTime.ToString('s')
                    lineas     = @($texto.Lines | ForEach-Object { $_.Text })
                })
        }
        finally { $flujo.Dispose() }
    }
    catch {
        [void]$resultados.Add([pscustomobject]@{ archivo = $item.Name; error = "$_" })
    }
}
# UTF-8 sin BOM (Out-File -Encoding utf8 de 5.1 pone BOM).
[System.IO.File]::WriteAllText($Salida, (ConvertTo-Json -InputObject $resultados.ToArray() -Depth 4), (New-Object System.Text.UTF8Encoding $false))
Write-Output "Leidos $($resultados.Count) archivos."
