# Carnés PRAIND de contratistas: guardarlos y verlos

Estado: **plan, sin implementar** (2026-11-15). Lo ejecuta otra sesión; este
documento trae todo el contexto necesario para empezar sin la conversación
original.

## Qué se quiere

Cada contratista tiene un carné de inducción PRAIND (imagen PNG con foto,
nombre, cédula, empresa y fechas de inducción y vencimiento). Hoy el sistema
sólo guarda la fecha de vencimiento (`fecha_vencimiento_praind`). Se quiere:

1. Guardar la imagen del carné **en su resolución original**.
2. Una opción nueva en el escritorio, "Ver detalles" del contratista, que
   muestre sus datos y el carné.
3. Verlo también en el teléfono (es lo que más usan los oficiales en la
   portería), sin que la app se vuelva lenta.
4. Cargar el carné escaneando su QR, sin subir archivos a mano.
5. Poder restringir quién carga carnés (sólo el panel, o panel y equipos) con
   un interruptor, sin publicar una versión nueva de las apps.
6. Importar de una vez los carnés que el dueño ya tiene en una carpeta.

Prioridad declarada por el dueño: rápido, eficiente y sin vueltas.

## Lo que ya se averiguó (no repetir)

### De dónde sale el carné

- El QR del carné lleva a un enlace corto `https://prd.pw/<código>`.
- Ese enlace responde `307` hacia
  `https://safety-record.com/api-aleluya/v0-aleluya/trabajadores-aleluya/<código>/despliegue_publico_aleluya`.
- Esa página es HTML público, sin sesión, de unos 2 KB. Trae:
  - el **nombre** de la persona, dentro del primer `<h2>`;
  - el programa de inducción, en un `<h4>` (por ejemplo "Programa De
    Contratistas Coca Cola Costa Rica Manufactura");
  - un único `<img alt="certificate_chirho" src="https://media-chirho.safety-record.com/...png">`
    con el carné original.
- El PNG de prueba mide 2338×3614 y pesa 1,24 MB. Es byte a byte el mismo
  archivo que el contratista recibe.
- La página **no** trae cédula, empresa ni fechas como texto: sólo están
  dentro de la imagen.
- El `<código>` es largo y opaco. No se deduce de la cédula ni del nombre, y
  el resto de la información del sitio pide usuario y contraseña.

**Límite firme:** no se recorren ni se adivinan códigos en `safety-record.com`.
Sólo se consulta el enlace de un QR que alguien presentó o de un carné que el
dueño ya tiene. La descarga masiva sale de la carpeta del dueño (fase 5), no
del sitio ajeno.

### Almacenamiento

- La organización de Supabase está en el plan **Free** (1 GB de archivos). Con
  originales de 1,24 MB entran unos 800 carnés: no alcanza. Por eso los
  archivos van a R2 y Supabase sólo guarda permisos y metadatos.
- En Cloudflare ya existe el bucket **`lattis-praind`** (creado el
  2026-10-19, región ENAM, vacío). Es privado: sin `r2.dev`, sin dominio
  conectado, sin CORS. Así debe quedar; no hay políticas que corregir.
- El otro bucket, `lattis-dist`, es el de descargas públicas
  (`lt.megabrisas.com`, 1,32 GB). No se usa para esto. Su dominio acepta TLS
  1.0; conviene subirlo a 1.2 en una tarea aparte.
- R2 cobra por el total de la cuenta, no por bucket. Los precios y el cupo
  gratis (10 GB según memoria de quien planeó) hay que confirmarlos en el
  panel de Cloudflare antes de la importación masiva.
- `wrangler` está instalado en `web/` con sesión iniciada y permisos de
  escritura sobre Workers y R2.

### Cómo es el proyecto hoy (lo que toca este plan)

- **Sesiones.** Los equipos (escritorio y móvil) usan un JWT ES256 firmado por
  `supabase/functions/_shared/dispositivos.ts` con rol `authenticated` y los
  claims `sitio_id`, `tipo` y `huella`; dura 1 hora. Postgres lo acepta y
  `private.dispositivo_vigente()` rechaza los equipos revocados. El panel web
  entra con Google y su correo debe estar en `administradores_panel`
  (`es_admin_global()` en RLS, `correoAdminAutorizado` en
  `supabase/functions/_shared/admin.ts`).
- **Contratistas.** Son un catálogo global (no por sitio). El modelo está en
  `src/models/contratista.rs`. La columna `fecha_vencimiento_praind` sirve de
  guía para ver por dónde viaja una columna: migraciones de Supabase,
  `src/database/schema/migraciones/` (la última es `migracion_63.sql`),
  `src/database/queries/contratistas.rs`,
  `src/database/repositories/contratista_repository.rs`,
  `src/nube/sincronizacion/catalogo.rs` y `cola.rs`,
  `mobile/rust-core/src/catalogo.rs` y `tipos.rs`,
  `supabase/functions/_shared/contratistas.ts` y `web/src/api/contratistas.ts`.
- **Escritorio.** Tauri + React. La pantalla es
  `desktop/src/pantallas/Contratistas.tsx`; hoy el doble clic abre
  `FormularioContratista`. Los comandos Rust están en
  `desktop/src-tauri/src/comandos/contratistas.rs`. El CSP
  (`desktop/src-tauri/tauri.conf.json`) sólo permite `img-src 'self'` y
  `connect-src` hacia Supabase. La base local está cifrada con SQLCipher.
- **Storage.** Ninguna app usa Supabase Storage ni R2 para datos; R2 sólo se
  usa desde los flujos de publicación de versiones.
- **Reglas del repositorio** (ver `AGENTS.md`): permisos explícitos en cada
  migración, nunca a `anon`; cada migración con su batería en
  `supabase/tests/`, probada en staging dentro de una transacción con
  `rollback`; mergear a `main` aplica las migraciones en producción; el
  despliegue del panel y de los Workers es manual y se pregunta antes.

## Diseño

```
equipo o panel ──(JWT de Supabase)──► Worker praind ──(binding)──► R2 lattis-praind
                                          │
                                          └──(mismo JWT)──► RPC en Supabase
                                                             permisos + metadatos
```

Tres decisiones sostienen el diseño:

1. **El Worker no guarda ningún secreto.** Lee y escribe R2 por *binding* (sin
   llaves de R2) y habla con Supabase usando el JWT de quien llama más la
   clave publicable. No lleva `service_role`.
2. **Los permisos viven en Postgres.** El Worker no interpreta el JWT: llama a
   una RPC con ese mismo token y Postgres valida firma, vigencia del equipo,
   si es administrador y el interruptor de carga. Una sola fuente de verdad,
   igual que el resto del sistema.
3. **Los archivos no cambian nunca.** La ruta del objeto incluye su SHA-256,
   así que se puede guardar en caché para siempre. Reemplazar un carné es
   escribir un objeto nuevo y cambiar la ruta en el contratista.

### Datos en Supabase (una migración)

- **Columna `contratistas.praind_foto text null`.** Guarda la ruta del objeto
  en R2: `contratistas/<contratista_id>/<sha256>.<ext>`. `null` significa que
  no hay carné. Viaja a los equipos por la sincronización de catálogo y el
  aviso en vivo que ya existen, así que cada equipo sabe sin consultar si hay
  carné que mostrar.
- **Tabla `fotos_praind`** (sólo en la nube, bitácora): `id`, `contratista_id`,
  `clave` (ruta en R2), `sha256`, `bytes`, `tipo_mime`, `enlace_origen` (el
  del QR, puede ser `null` en cargas manuales), `nombre_en_origen`,
  `cargado_por` (correo del administrador o id del equipo), `cargado_en`,
  `reemplazada_en`. Lectura sólo para administradores del panel.
- **Interruptor de carga.** Buscar primero si ya hay una tabla de ajustes
  globales; no se encontró ninguna con nombre `ajustes`, `configuracion` o
  `parametros`. Si no existe, crear `ajustes_praind` de una sola fila con
  `carga_desde_equipos boolean not null default false`. Lo cambian sólo los
  administradores del panel. Arranca en `false`: sólo el panel carga.
- **RPC `praind_permisos()`** → `{ puede_ver, puede_cargar }`.
  - `puede_ver`: equipo vigente o administrador del panel.
  - `puede_cargar`: administrador del panel, o equipo vigente con el
    interruptor encendido.
- **RPC `praind_registrar(contratista_id, clave, sha256, bytes, tipo_mime,
  enlace_origen, nombre_en_origen)`.** `security definer`; vuelve a comprobar
  `puede_cargar`, valida que `clave` tenga la forma esperada para ese
  contratista, inserta en `fotos_praind`, marca la anterior como reemplazada
  y actualiza `contratistas.praind_foto`. Devuelve la clave anterior para que
  el Worker borre ese objeto.
- **RPC `praind_quitar(contratista_id)`.** Sólo administradores del panel.
- Permisos: `grant execute ... to authenticated, service_role` en las tres
  RPC; `grant` de tabla según las reglas de `AGENTS.md`. Nada a `anon`.
- Confirmar que el disparador que avisa cambios de `contratistas` a los
  equipos se activa también con la columna nueva.

Batería en `supabase/tests/`: equipo vigente ve y no carga con el interruptor
apagado; carga con el interruptor encendido; equipo revocado no ve; `anon` no
ejecuta nada; administrador carga, reemplaza y quita; `praind_registrar`
rechaza una clave que no corresponde al contratista.

### Worker

- Carpeta nueva `worker-praind/` en la raíz, con el mismo estilo de
  `web/wrangler.jsonc`. Nombre `praind-brisas`, `workers_dev: false`,
  `preview_urls: false`, dominio propio `praind.megabrisas.com`.
- Binding R2 `FOTOS` → `lattis-praind`. Variables (no secretas): URL y clave
  publicable de Supabase, y la lista de orígenes permitidos para CORS (sólo el
  dominio del panel; las apps nativas no usan CORS).
- Un segundo entorno para staging: otro bucket (`lattis-praind-staging`) y el
  proyecto de staging de Supabase. Las pruebas nunca escriben en el bucket de
  producción.

| Ruta | Quién | Qué hace |
|---|---|---|
| `GET /foto/<clave>` | `puede_ver` | Entrega el original desde R2 |
| `POST /cargar-desde-qr` | `puede_cargar` | Recibe `{ contratista_id, enlace }`, baja el original y lo guarda |
| `PUT /cargar/<contratista_id>` | `puede_cargar` | Recibe el archivo directamente (respaldo e importación) |
| `DELETE /foto/<contratista_id>` | administrador | Quita el carné |

Reglas del Worker:

- **Autorización.** Toma el `Authorization: Bearer` y llama a
  `praind_permisos()`. Guarda la respuesta 60 segundos por huella del token
  (hash, nunca el token) para no pagar un viaje a Supabase por imagen. Las
  escrituras no usan esa caché: `praind_registrar` valida de nuevo.
- **`GET`.** Responde con `Cache-Control: private, max-age=31536000,
  immutable`, `ETag` con el SHA-256 y `X-Content-Type-Options: nosniff`.
  Nunca caché compartida ni pública.
- **Descarga desde el QR.** Es la parte delicada: el Worker pide una dirección
  que le dicta el cliente. Para que no sirva de puente hacia otros sitios:
  - sólo `https`, y sólo estos anfitriones: `prd.pw` y `safety-record.com`
    para la página, `media-chirho.safety-record.com` para la imagen;
  - redirecciones seguidas a mano, máximo 3, validando el anfitrión en cada
    salto;
  - la página no puede pasar de 64 KB ni la imagen de 10 MB;
  - el tipo se comprueba por los primeros bytes del archivo (PNG o JPEG), no
    por la cabecera;
  - se extraen el nombre del `<h2>` y el `src` del `<img>` con
    `HTMLRewriter`, sin expresiones regulares sobre el HTML completo.
- **Comprobación del nombre.** El Worker devuelve `nombre_en_origen` junto con
  el resultado. La app compara ese nombre con el del contratista (misma
  normalización que ya usa el proyecto para nombres) y, si no coinciden, pide
  confirmación antes de guardar. Para eso `POST /cargar-desde-qr` acepta
  `solo_previsualizar: true`, que devuelve nombre y tamaño sin guardar nada.
- **Orden al guardar.** Calcular SHA-256, escribir en R2, llamar a
  `praind_registrar`, borrar el objeto anterior. Si la RPC falla, borrar el
  objeto recién escrito para no dejar huérfanos.
- **Límite de uso.** Regla de *rate limiting* por IP en las rutas de carga.
- **Registros.** Nunca escribir tokens, nombres ni cédulas en los logs; sólo
  ids y códigos de resultado.
- Pruebas con `vitest` y `@cloudflare/vitest-pool-workers`: permisos, lista
  de anfitriones, redirección a un anfitrión no permitido, archivo que no es
  imagen, archivo demasiado grande, falla de la RPC sin dejar huérfanos.

### Escritorio

- **Datos.** Migración local 64 que agrega `praind_foto` a `contratistas`;
  sumar el campo al modelo, consultas, repositorio, sincronización y DTO.
- **Descarga en Rust.** Comando nuevo que baja la imagen con el token del
  equipo y entrega los bytes al frontend. El token no sale de Rust y el CSP no
  se abre a dominios externos; sólo hay que agregar `blob:` a `img-src`.
- **Caché.** En la primera fase, en memoria (los últimos 20 carnés). La base
  local está cifrada; guardar carnés sin cifrar en disco bajaría esa
  protección, así que la caché en disco queda para después y debe ir cifrada
  (ver "Decisiones abiertas").
- **Pantalla.** Acción "Ver detalles" en la pantalla Contratistas: panel con
  nombre, cédula, empresa, tipo de ingreso, vencimiento del PRAIND, estado de
  acceso y el carné, con ampliación al hacer clic. Sin carné: texto "Este
  contratista no tiene carné PRAIND cargado." Sin conexión y sin caché: "No se
  pudo cargar el carné. Revisá la conexión."
- **Doble clic.** Hoy abre la edición. Recomendación: que el doble clic abra
  los detalles y que ahí haya un botón "Editar", porque ver es más frecuente
  que editar. Es un cambio de costumbre; confirmarlo con el dueño.
- **Carga.** Si `praind_permisos()` devuelve `puede_cargar`, el panel de
  detalles muestra "Cargar carné" con dos entradas: pegar el enlace del QR o
  elegir un archivo de imagen (el escritorio lee el QR del archivo). Si no,
  la opción no aparece.
- Seguir las convenciones del frontend: lógica en `*.logica.ts` con su prueba,
  estilos de `diseno.css`/`controles.css`, textos en español.

### Panel web

- En la pantalla de contratistas: ver el carné, cargarlo pegando el enlace del
  QR o subiendo el archivo, y quitarlo.
- En ajustes: el interruptor "Permitir que los equipos carguen carnés".
- El panel llama al Worker con `fetch` y muestra la imagen como `blob:`.
  Revisar `web/public/_headers` (hay una prueba, `web/src/_headers.test.ts`)
  para permitir `connect-src https://praind.megabrisas.com` e `img-src blob:`.
- Actualizar el manual: `docs/manual/panel/contratistas.md`.

### Móvil

- **Ver.** En la ficha del contratista, un botón "Ver carné" que aparece sólo
  si `praind_foto` no es `null`. La imagen se baja al tocar el botón, nunca al
  abrir la ficha ni al sincronizar, para no gastar datos ni frenar la app.
  Caché en el almacenamiento privado de la app, limitada en tamaño.
- **Cargar.** Con el interruptor encendido: "Escanear QR del carné" con el
  lector que la app ya tiene; manda el enlace al Worker, que hace la descarga.
  El teléfono no sube el archivo, así que funciona bien con mala señal.
- Respetar la separación Kotlin/Rust descrita en
  `mobile/android/arquitectura.md`.

### Importación de la carpeta

Programa local en `scripts/`, que corre en la máquina del dueño con una sesión
de administrador del panel. Dos pasadas:

1. **Simulación.** Recorre la carpeta, lee el QR de cada imagen, consulta la
   página de ese enlace para obtener el nombre, busca al contratista y escribe
   un reporte CSV: archivo, nombre en origen, contratista encontrado, estado
   (`listo`, `sin QR`, `sin coincidencia`, `varias coincidencias`, `ya tiene
   carné`). No escribe nada.
2. **Carga.** Sube sólo las filas `listo` por `PUT /cargar/<contratista_id>`.
   Se puede repetir sin duplicar: mismo SHA-256, misma ruta.

Las filas que no calzan las resuelve una persona con el reporte en la mano.
Entre consultas a la página ajena, una pausa de un segundo: son carnés
propios, pero el sitio no es nuestro.

## Fases y entregas

Cada fase es un PR que se puede probar solo.

| Fase | Contenido | Se puede probar cuando |
|---|---|---|
| 1 | Migración, RPC y batería en staging | La batería pasa en staging con `rollback` |
| 2 | Worker con sus pruebas, desplegado contra staging | Se carga y se ve un carné con `curl` y un token de staging |
| 3 | Escritorio: detalles, vista y carga | Se ve el carné de un contratista en la app |
| 4 | Panel web: vista, carga, quitar e interruptor | Un administrador carga y bloquea la carga desde equipos |
| 5 | Importación de la carpeta | El reporte de simulación cuadra con la carpeta |
| 6 | Móvil: ver y escanear | El oficial ve el carné en el teléfono |

Orden obligatorio: 1 antes que 2, 2 antes que el resto. Las fases 3 a 6 son
independientes entre sí.

Antes de producción: crear el dominio `praind.megabrisas.com`, desplegar el
Worker de producción (manual, preguntando antes) y recién después mergear las
apps que lo usan.

## Riesgos

- **La página ajena cambia.** Si `safety-record.com` cambia su HTML, la carga
  por QR falla con un mensaje claro y queda la carga por archivo. Los carnés
  ya guardados no dependen de ese sitio.
- **Carné equivocado.** Mitigado con la comparación de nombre y la
  confirmación; la bitácora `fotos_praind` deja ver quién cargó qué.
- **Datos personales.** Cara, nombre y cédula. Bucket privado, sin caché
  compartida, sin datos personales en logs, y borrado del objeto cuando se
  quita el carné o se reemplaza.
- **Contratista eliminado.** Definir qué pasa con su carné (ver abajo).
- **Cupo de R2.** Revisar el tamaño total de la carpeta antes de importar.

## Decisiones abiertas (preguntar al dueño)

1. **Nombres de los archivos de la carpeta.** ¿Son imágenes o PDF? ¿El nombre
   del archivo trae la cédula o el nombre? Si trae la cédula, la importación
   puede emparejar sin consultar la página ajena.
2. **Doble clic en Contratistas:** ¿abre detalles (recomendado) o sigue
   abriendo la edición?
3. **Caché en disco del escritorio:** ¿hace falta ver carnés sin internet? Si
   sí, se agrega una caché cifrada en una fase posterior.
4. **Vencimiento.** Cuando el PRAIND vence y el contratista trae un carné
   nuevo, ¿se conserva el anterior como historial o se borra? El plan asume
   que se borra.
5. **Contratista eliminado o con acceso negado:** ¿se conserva su carné? El
   plan asume que se conserva mientras exista el contratista y se borra con
   él.
6. **Interruptor:** ¿uno global (lo que asume el plan) o por sitio?

## Fuera de alcance

- Leer la cédula o las fechas desde la imagen (OCR) para autocompletar.
- Versiones reducidas de la imagen: el dueño pidió originales.
- Recorrer `safety-record.com` para obtener carnés que nadie presentó.
