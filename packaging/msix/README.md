# Empaquetado MSIX

Genera el paquete `.msix` para subir a Microsoft Store (identidad de producto:
"Control de Acceso Brisas", editor DQM27). Se hace en la PC con el Windows SDK
instalado — no en la máquina de compilación habitual sin privilegios de admin.

## Requisitos (una sola vez, con admin)

Instalar el **Windows SDK** (trae `MakeAppx.exe` y `signtool.exe`):
https://developer.microsoft.com/windows/downloads/windows-sdk/

Después de instalarlo, agregar al PATH la carpeta de herramientas, típicamente:
`C:\Program Files (x86)\Windows Kits\10\bin\<versión>\x64`

## Pasos para armar el paquete

1. Compilar el release normal (no `--release-native`, que no es portable):
   ```powershell
   cargo build --release
   ```

2. Copiar el ejecutable recién compilado a esta carpeta, junto al manifiesto:
   ```powershell
   Copy-Item target\release\control_acceso.exe packaging\msix\control_acceso.exe
   ```

3. Empaquetar:
   ```powershell
   MakeAppx.exe pack /d packaging\msix /p ControlAccesoBrisas.msix
   ```

   Esto produce `ControlAccesoBrisas.msix` a partir de todo lo que hay en
   `packaging\msix\` (el manifiesto, `Assets\`, y el `.exe` copiado en el
   paso anterior).

## Subir a Partner Center

**No firmar el `.msix` con ningún certificado propio.** Con firma
administrada por la Store (Store-managed signing), Microsoft firma el
paquete con su propio certificado recién al aprobarlo — si lo firmás vos
antes, puede chocar con el nombre de editor declarado en Partner Center y
la validación falla. Subir el archivo tal cual sale de `MakeAppx.exe`.

En Partner Center → Control de Acceso Brisas → "Iniciar envío", subir este
`.msix` en la sección de paquetes.

## Probarlo localmente antes de subir (opcional)

Para instalar el paquete sin pasar por la Store y confirmar que abre bien,
hace falta firmarlo con un certificado de prueba autofirmado e instalarlo
como confiable en esa misma PC (con admin, solo en esa PC de pruebas — esto
es aparte de la firma final, que la pone la Store):

```powershell
New-SelfSignedCertificate -Type Custom -Subject "CN=84E326A0-985C-4221-9089-EF72F40C735C" `
  -KeyUsage DigitalSignature -FriendlyName "ControlAccesoBrisas test" `
  -CertStoreLocation "Cert:\CurrentUser\My" `
  -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}")

signtool sign /fd SHA256 /a /f <ruta-al-pfx-exportado> ControlAccesoBrisas.msix
```

El `Subject` del certificado de prueba tiene que coincidir exactamente con
`Package/Identity/Publisher` del manifiesto (`CN=84E326A0-985C-4221-9089-EF72F40C735C`),
si no, Windows rechaza el paquete al instalarlo.
