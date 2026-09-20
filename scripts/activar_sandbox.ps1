# Activa, en la sesión actual de PowerShell, las variables necesarias para
# que desktop/mobile apunten al proyecto de staging de Supabase Y a una
# base de datos local separada -- para no ensuciar ni la nube de producción
# ni el archivo control_acceso.db real. Ver docs/recuperacion-sitio-staging.md.
#
# Uso: . .\scripts\activar_sandbox.ps1   (con el punto y espacio al inicio,
# si no las variables no sobreviven fuera de este script)

$env:CONTROL_ACCESO_SUPABASE_URL = "https://pmrytjktlyiuikxuuxpr.supabase.co"
$env:CONTROL_ACCESO_SUPABASE_APIKEY = "sb_publishable_29DwMvfyj8Jq--LBcqxtBA_pTwWrDH4"
$env:CONTROL_ACCESO_DB = Join-Path $env:LOCALAPPDATA "ControlAccesoSandbox\control_acceso.db"

# `dispositivo-nube.secret` (secreto de activación) y `db_key.dat` (clave
# SQLCipher) viven en %APPDATA%\ControlAcceso -- una carpeta DISTINTA de
# %LOCALAPPDATA%, sin variable de override propia (ver
# src/nube/credenciales.rs, ROAMING_APP_DATA_ENV). Sobreescribir APPDATA acá
# es la única forma de aislarla sin tocar código -- solo afecta esta misma
# terminal, no el resto de Windows.
$env:APPDATA = Join-Path $env:LOCALAPPDATA "ControlAccesoSandboxRoaming"

Write-Host "Sandbox activo en esta terminal:"
Write-Host "  Supabase:      $env:CONTROL_ACCESO_SUPABASE_URL"
Write-Host "  DB local:      $env:CONTROL_ACCESO_DB"
Write-Host "  Secreto/clave: $env:APPDATA\ControlAcceso"
Write-Host "Corre 'cargo run' / 'npm run tauri dev' desde esta misma terminal para que lo hereden."
Write-Host "OJO: mientras esta terminal esté activa, cualquier otro programa que lances desde ella (npm, etc.) también ve este APPDATA distinto."
