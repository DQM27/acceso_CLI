# Actualizaciones de la app Android

La app se distribuye fuera de cualquier tienda. Desde la versión que trae el
actualizador, se actualiza sola desde el almacenamiento de descargas
(Cloudflare R2 detrás de `lt.megabrisas.com`), sin depender de GitHub
Releases. Así sigue funcionando con el repositorio privado.

## Cómo funciona

1. Cada release del móvil (`release-movil.yml`, tag `movil-v<versionName>`)
   sube a R2:
   - el APK, en `android/Lattis-mobile-v<versión>-<12 primeros del SHA-256>.apk`.
     Es inmutable y se cachea para siempre;
   - el manifiesto `android/latest.json`, sin caché. Se sube **después** del
     APK.
2. La app consulta el manifiesto al pasar a primer plano, como mucho una vez
   por hora (`MainActivity.onStart`).
3. Si el `version_code` publicado es mayor que el instalado, aparece una tarjeta
   flotante, semitransparente, abajo de todas las pantallas (incluido el login),
   sin empujar el contenido. Tiene **Actualizar** y una **X**. La X la cierra
   ("Más tarde"): la versión queda pendiente como un puntito que late unos segundos
   sobre la inicial del operador (menú de la cuenta) y no vuelve en cada revisión
   de la misma versión. Una versión más nueva, o una obligatoria (sin X, en rojo),
   vuelve a mostrar la tarjeta.
4. Al tocar **Actualizar**:
   - la primera vez, Android pide habilitar "Instalar apps desconocidas" para
     Lattis (sólo una vez por teléfono);
   - se baja el APK verificando su SHA-256, y antes de instalar se comprueba
     que sea el mismo paquete, la versión anunciada y la misma firma;
   - se instala con `PackageInstaller`. Desde Android 12, si se cumplen las
     condiciones del sistema, no pide confirmación; si igual la pide, se
     muestra el diálogo del sistema. Al terminar, Android cierra Lattis y hay
     que volver a abrirla.

### Desde el menú de la cuenta

El ítem de actualización del menú (la inicial del operador, arriba a la derecha)
es un solo botón que cambia: **Buscar actualizaciones** cuando no hay una versión
confirmada y **Actualizar a X** cuando sí la hay. Al buscar, el menú se cierra, la
inicial muestra un círculo girando y la respuesta sale como aviso corto: "Estás al
día" o "No se pudo revisar" (si hay versión nueva no hay aviso: aparece la tarjeta
y el puntito). La búsqueda no espera la hora entre revisiones. Los builds de
prueba, sin dirección del manifiesto, no muestran el ítem. El menú también
muestra la versión instalada.

### Quién decide qué

Igual que en el resto de la app (ver `mobile/android/arquitectura.md`):

| Capa | Archivo | Responsabilidad |
|---|---|---|
| Núcleo (Rust) | `mobile/rust-core/src/actualizacion.rs` | `evaluar_manifiesto_actualizacion`: valida el manifiesto y decide si hay versión nueva y si es obligatoria. |
| Servicio Android | `ServicioActualizaciones.kt` | Sólo lo que es de Android: leer el manifiesto, descargar y verificar el APK, entregarlo a `PackageInstaller` y recibir el resultado. No decide nada. |
| ViewModel | `ActualizacionViewModel.kt` | Dueño del estado: cuándo consultar (una vez por hora), descarga, instalación y errores. |
| Pantalla | `AvisoActualizacion.kt` | Sólo dibuja la franja con el estado que recibe. |

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
  "url": "https://lt.megabrisas.com/android/Lattis-mobile-v1.4.11-0123456789ab.apk",
  "sha256": "<64 caracteres hexadecimales en minúscula>",
  "tamano_bytes": 52428800,
  "version_minima_code": 0
}
```

La app rechaza el manifiesto si el esquema no es 1, si el APK no se sirve por
HTTPS **desde el mismo servidor que el manifiesto**, si el SHA-256 está mal
formado, si el tamaño pasa de 300 MB o si `version_minima_code` es mayor que
`version_code` (ver `mobile/rust-core/src/actualizacion.rs` y sus tests). Un campo
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

## Cómo sacar una versión del móvil

El móvil se publica aparte del escritorio. Un tag `v*` (escritorio) ya no compila
ni publica el APK.

1. Subir `versionCode` y `versionName` en `mobile/android/app/build.gradle.kts` y
   fusionar.
2. `git tag movil-v<versionName>` y `git push origin movil-v<versionName>`
   (por ejemplo `movil-v1.4.14`).

`release-movil.yml` hace el resto: prueba, compila, firma, crea el Release de
GitHub `Lattis móvil v<versionName>` (nunca marcado como "Latest", para no pisar
el `latest.json` del escritorio) y publica el APK y el manifiesto en R2.

Protecciones:

- El job falla si el tag no coincide con el `versionName` del código.
- Una versión ya publicada no se sube otra vez: si el manifiesto de R2 ya tiene
  ese `versionCode`, el paso de R2 no hace nada. Así no quedan dos APK distintos
  con la misma versión. Para forzarlo, disparo manual (Actions → Release móvil →
  Run workflow) con `republicar`.
- Dos corridas no se ejecutan a la vez, para que no se pisen el manifiesto.

---

## Configuración inicial (una sola vez)

1. **Cloudflare → R2:** activar R2 y crear el bucket (por ejemplo
   `lattis-dist`).
2. **Bucket → Settings → Custom Domains:** conectar `lt.megabrisas.com`.
   No usar el subdominio `r2.dev`: tiene límite de uso y Cloudflare indica que
   no es para producción. El dominio tiene que coincidir con el de
   `URL_MANIFIESTO_ACTUALIZACION` y con `URL_DESCARGAS` de `release-movil.yml`.
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
curl -s https://lt.megabrisas.com/android/latest.json
```

Tiene que mostrar el `version_code` recién publicado, y la dirección de `url`
tiene que descargar un archivo cuyo `sha256sum` coincida con el del manifiesto.

## Pendiente

La lista completa, en orden, está en `docs/pendientes.md` ("Distribución fuera de
GitHub"). Lo que toca a este documento:

- Página de descargas en `lt.megabrisas.com`, con la versión vigente de
  Android y de Windows. Es el link para instalar en un equipo nuevo cuando los de
  GitHub Releases ya no sean públicos.
- El escritorio todavía lee su `latest.json` de GitHub Releases
  (`desktop/src-tauri/tauri.conf.json`). Hay que moverlo a R2 antes de volver
  privado el repositorio.
- Probar en el Samsung que la actualización no pida confirmación. Según la
  documentación de Android, una app que se actualiza a sí misma ya cumple esa
  condición desde la primera vez.
