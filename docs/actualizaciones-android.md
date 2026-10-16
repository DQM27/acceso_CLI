# Actualizaciones de la app Android

La app se distribuye fuera de cualquier tienda. Desde la versión que trae
`Actualizador.kt`, se actualiza sola desde el almacenamiento de descargas
(Cloudflare R2 detrás de `descargas.megabrisas.com`), sin depender de GitHub
Releases. Así sigue funcionando con el repositorio privado.

## Cómo funciona

1. Cada release (`release.yml`, job `build-android`) sube a R2:
   - el APK, en `android/Lattis-mobile-v<versión>-<12 primeros del SHA-256>.apk`.
     Es inmutable y se cachea para siempre;
   - el manifiesto `android/latest.json`, sin caché. Se sube **después** del
     APK.
2. La app consulta el manifiesto al pasar a primer plano, como mucho una vez
   por hora (`MainActivity.onStart`).
3. Si el `version_code` publicado es mayor que el instalado, aparece una franja
   arriba de todas las pantallas, incluido el login.
4. Al tocar **Actualizar**:
   - la primera vez, Android pide habilitar "Instalar apps desconocidas" para
     Lattis (sólo una vez por teléfono);
   - se baja el APK verificando su SHA-256, y antes de instalar se comprueba
     que sea el mismo paquete, la versión anunciada y la misma firma;
   - se instala con `PackageInstaller`. Desde Android 12, si se cumplen las
     condiciones del sistema, no pide confirmación; si igual la pide, se
     muestra el diálogo del sistema. Al terminar, Android cierra Lattis y hay
     que volver a abrirla.

Las verificaciones de la app existen para fallar antes y con un mensaje claro.
La garantía de fondo es de Android: no instala encima un APK firmado con otra
llave ni una versión menor.

Los builds de prueba (`debug`, `diagnostico`) tienen el actualizador apagado:
`URL_MANIFIESTO_ACTUALIZACION` vacía en `app/build.gradle.kts`.

### Formato del manifiesto

```json
{
  "esquema": 1,
  "version_code": 33,
  "version_name": "1.4.11",
  "url": "https://descargas.megabrisas.com/android/Lattis-mobile-v1.4.11-0123456789ab.apk",
  "sha256": "<64 caracteres hexadecimales en minúscula>",
  "tamano_bytes": 52428800,
  "version_minima_code": 0
}
```

La app rechaza el manifiesto si el esquema no es 1, si el APK no se sirve por
HTTPS **desde el mismo servidor que el manifiesto**, si el SHA-256 está mal
formado, si el tamaño pasa de 300 MB o si `version_minima_code` es mayor que
`version_code` (ver `ActualizacionCalculos.validar` y sus tests). Un campo
nuevo se puede agregar sin subir `esquema`; cambiar el significado de uno
existente sí lo sube, y la app vieja deja de ofrecer actualizaciones hasta que
se instale una nueva a mano.

## Actualización obligatoria

Por defecto la actualización es opcional ("Más tarde" la oculta hasta la
próxima revisión). Para obligarla, se pone en la variable del repositorio
`ANDROID_VERSION_MINIMA_CODE` el primer `versionCode` aceptable y se saca un
release. Por debajo de ese número la franja sale en rojo y no se puede
posponer.

**No bloquea la app**, a propósito: el oficial puede seguir registrando aunque
no tenga señal para bajar la actualización. La sincronización la corta el
núcleo por su cuenta cuando la nube responde que la versión ya no es
compatible (`VersionDesactualizada`).

## Configuración inicial (una sola vez)

1. **Cloudflare → R2:** activar R2 y crear el bucket (por ejemplo
   `lattis-descargas`).
2. **Bucket → Settings → Custom Domains:** conectar `descargas.megabrisas.com`.
   No usar el subdominio `r2.dev`: tiene límite de uso y Cloudflare indica que
   no es para producción. El dominio tiene que coincidir con el de
   `URL_MANIFIESTO_ACTUALIZACION` y con `URL_DESCARGAS` de `release.yml`.
3. **R2 → Administrar tokens de API:** crear un token con permiso *Object Read
   & Write* **sólo sobre ese bucket**.
4. **GitHub → Settings → Secrets and variables → Actions:**
   - secretos `R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`;
   - variable `R2_BUCKET` con el nombre del bucket;
   - variable `ANDROID_VERSION_MINIMA_CODE`, opcional (vacía = nunca obligatoria).

Mientras falte algo de esto, el release sigue saliendo y deja el aviso "R2 sin
configurar" en el log del paso.

5. **Primera instalación:** las versiones anteriores no tienen actualizador. La
   primera versión que lo trae se instala a mano en cada teléfono; de ahí en
   adelante se actualizan solas.

## Comprobar una publicación

```sh
curl -s https://descargas.megabrisas.com/android/latest.json
```

Tiene que mostrar el `version_code` recién publicado, y la dirección de `url`
tiene que descargar un archivo cuyo `sha256sum` coincida con el del manifiesto.

## Pendiente

- El escritorio todavía lee su `latest.json` de GitHub Releases
  (`desktop/src-tauri/tauri.conf.json`). Hay que moverlo a R2 antes de volver
  privado el repositorio.
- Probar en el Samsung que la actualización no pida confirmación. Según la
  documentación de Android, una app que se actualiza a sí misma ya cumple esa
  condición desde la primera vez.
