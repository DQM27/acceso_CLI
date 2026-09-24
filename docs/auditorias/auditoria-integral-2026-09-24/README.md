# Auditoría integral de QA — núcleo, escritorio y móvil (2026-09-24)

> Commit auditado: `372d93d` (rama `claude/modal-modifications-1nb87e`, que ya
> incluye los cambios de modales de `feature/tokyo-night-mobile`).
> Alcance: núcleo Rust (`src/`), capa de nube y Supabase (`src/nube/`,
> `supabase/`), escritorio (`desktop/src-tauri` + `desktop/src`), móvil
> (`mobile/rust-core`, `mobile/android`, `mobile/ios`) y los workflows de CI que
> los afectan. **Fuera de alcance:** `web/` y `web-visitas/`. Rutas y
> visitas/citas (funcionalidad en pausa) **sí** se analizaron.

## Cómo se hizo

Seis auditorías en paralelo, cada una con su anexo detallado en esta carpeta:

| Anexo | Área | Hallazgos |
|---|---|---|
| [01-nucleo-rust.md](01-nucleo-rust.md) | Dominio, servicios, base de datos, migraciones, exportación, autenticación local | 21 (NR-xx) |
| [02-nube-supabase.md](02-nube-supabase.md) | Sincronización, RLS, funciones `SECURITY DEFINER`, edge functions, tokens | 28 (NS-xx) |
| [03-desktop-tauri.md](03-desktop-tauri.md) | Los 85 comandos Tauri, capabilities, CSP, clave DPAPI, PDF, updater, release | 23 (DT-xx) |
| [04-desktop-frontend.md](04-desktop-frontend.md) | React/TS, modales, carreras async, contratos TS↔Rust, accesibilidad | 23 (DF-xx) |
| [05-movil.md](05-movil.md) | OWASP MASVS, puente UniFFI, Keystore, OCR/MRZ, Compose, iOS | 26 (MV-xx) |
| [06-herramientas-codigo-muerto.md](06-herramientas-codigo-muerto.md) | clippy, tests, cobertura, `cargo audit`, `npm audit`, `cargo machete`, `unsafe`, CI | 14 (HT-xx) |

Cada hallazgo trae archivo:línea, escenario de impacto, referencia externa
(OWASP ASVS/MASVS, CWE, RustSec/GHSA, documentación oficial de Tauri, Supabase,
SQLite, Android) y si es **nuevo** o un **pendiente de auditorías previas** que
sigue abierto. Los hallazgos de severidad Alta se verificaron además a mano
contra el código (ver "Verificación manual").

### Totales

| Severidad | Núcleo | Nube | Tauri | Frontend | Móvil | Herram. | **Total** |
|---|---:|---:|---:|---:|---:|---:|---:|
| Crítica | 0 | 0 | 0 | 0 | 0 | 0 | **0** |
| Alta | 1 | 7 | 2 | 0 | 2 | 0 | **12** |
| Media | 10 | 11 | 8 | 9 | 7 | 3 | **48** |
| Baja | 9 | 8 | 10 | 9 | 15 | 7 | **58** |
| Info | 1 | 2 | 3 | 5 | 2 | 4 | **17** |

Hay solapes entre anexos (un mismo problema visto desde dos capas); abajo se
agrupan como un solo tema.

### Estado de las herramientas automáticas

| Chequeo | Resultado |
|---|---|
| `cargo clippy` (raíz, móvil, escritorio compilado para Windows) | Limpio, 0 warnings con la configuración del repo |
| `cargo test-plano` (raíz) | 766 ✓ · 0 ✗ · 1 ignorado (`nube_smoke`, requiere red) |
| `cargo test` `mobile/rust-core` | 21 ✓ |
| `vitest` (`desktop/`) | 284 ✓ en 45 archivos |
| `tsc --noEmit` / `npm run lint` | 0 errores / 56 advertencias |
| `cargo audit` (5 lockfiles) | 0 vulnerabilidades; 7 avisos unmaintained/unsound ya aceptados (HT-01) |
| `npm audit` (`desktop/`) | 0 vulnerabilidades |
| Cobertura de líneas del núcleo | 85,9 % (bajos: `application/nube.rs` 7,9 %, `application/gafetes.rs` 42 %) |
| `unwrap()` en código no-test | 0 en núcleo y móvil |
| `desktop/src-tauri` en Linux | **No compila** (rama `#[cfg(not(windows))]` rota, HT-05) |

`cargo audit` **no** detecta la vulnerabilidad del updater (DT-03): el aviso
está publicado en GHSA pero todavía no en RustSec.

---

## Verificación manual de los hallazgos altos

Confirmados leyendo el código, además de lo que reportaron los agentes:

| ID | Confirmación |
|---|---|
| NR-01 / MV-03 | `mobile/rust-core/src/lib.rs:1189` abre con `AppCore::abrir_con_reloj` → `open_database` → `abrir_conexion(path, None)` (`src/database/connection.rs:96`). Ninguna clave. |
| MV-01 / DF-03 | `lib.rs:2759` cachea la contraseña (temporal incluida) tras el login online; el login offline devuelve `debe_cambiar_password: false` fijo (`lib.rs:1377`). |
| NS-01 | `destino_lote` acepta `("salida_ruta", _)` (`sincronizacion.rs:150`), `construir_cuerpo` no tiene esa rama y cae en `unreachable!` (`:175`); el lote se usa con `grupo.len() > 1` (`:242`). |
| NS-02 | `admin-delete-device` sólo hace `update({ oculto_en_panel: true })`; `device-auth` filtra por `revoked_at` y `suspended_at`, no por `oculto_en_panel`. |
| NS-07 | `supabase/functions/device-auth/index.ts:41`: `TOKEN_TTL_SECONDS = 12 * 60 * 60`. |
| DT-01 | 75 de los comandos `#[tauri::command]` son `fn` síncronas; la documentación de Tauri v2 indica que corren en el hilo principal. |
| DT-02 | `comandos/exportacion.rs:30-37`: `guardar_csv` hace `std::fs::write(&destino, …)` con la ruta que manda el webview. |
| DT-03 | `desktop/src-tauri/Cargo.lock`: `tauri-plugin-updater 2.11.0`; `capabilities/default.json` concede `updater:default`. [CVE-2026-95624](https://www.strix.ai/cve/CVE-2026-95624), corregida en 2.12.0. |

---

## Temas principales (agrupados y priorizados)

### 1. Datos personales sin cifrar en Android — Alta
**NR-01, MV-03, MV-24.** La base del teléfono (cédulas, nombres, ingresos y los
hashes Argon2 de las contraseñas cacheadas) se guarda **en claro**, aunque se
compile con SQLCipher/SQLite3MC. La documentación (`docs/credenciales.md`,
`application/mod.rs:119-122`, `release.yml:162-168`) afirma lo contrario. Además,
`allowBackup="false"` no impide la transferencia entre teléfonos en Android 12+
(Smart Switch y similares) porque falta `dataExtractionRules`.
**Acción:** derivar una clave de 32 bytes protegida por el Keystore (el mismo
mecanismo que ya usa `SecretoDispositivoStore`), abrir con
`abrir_con_reloj_cifrado`, migrar la base existente con `sqlcipher_export`, y
agregar `dataExtractionRules` que excluyan la base.

### 2. Se puede esquivar el cambio obligatorio de contraseña — Alta
**MV-01, DF-03, MV-02, NS-13.** El login online cachea el hash de la contraseña
**temporal** antes de que se cambie; el siguiente login offline entra con
`debe_cambiar_password=false`. En escritorio el backend deja la sesión abierta
y sólo la interfaz exige el cambio. En móvil, después de cambiar la contraseña
la nueva es rechazada offline por 24 h y la temporal sigue sirviendo. Además
la bandera vive en `user_metadata`, que el propio usuario puede modificar.
**Acción:** no cachear mientras `debe_cambiar_password` sea verdadero; guardar la
bandera localmente y exigirla en el backend; actualizar la caché al cambiar la
contraseña; mover la bandera a `app_metadata`.

### 3. Sincronización: caídas y pérdida de datos — Alta
- **NS-01:** pánico con dos o más salidas de ruta pendientes; se detiene todo lo
  que viene después en ese ciclo (bajas de usuarios, expulsión de sesión).
- **NS-03:** un cierre que llega antes que su apertura se marca `enviado` sin
  afectar filas → ingresos "fantasma" abiertos para siempre en la nube
  (ingresos, visitas, proveedores, KOF, salidas de ruta).
- **NS-06:** el upsert por clave natural reescribe la PK `id` cuando dos
  dispositivos tienen UUID distintos para la misma persona → ingresos en
  `fallido` permanente.
- **NS-09:** marca de agua compartida entre tablas y sin traslape → bajas de
  acceso que se pierden en un dispositivo.
- **NS-11:** los cierres remotos usan el reloj sin corregir.
- **DF-08:** `fallosPermanentesNube` nunca se muestra, así que nadie se entera.

**Acción:** agregar la rama `salida_ruta` (o quitarla de `destino_lote`) con una
prueba de lote; exigir `Prefer: return=representation` y reintentar si el
PATCH afecta 0 filas; `on_conflict` sobre la clave natural sin tocar `id`;
marcas de agua por tabla con traslape; exponer los fallos permanentes en la UI.

### 4. Revocar un dispositivo no lo revoca — Alta
**NS-02, NS-07, NS-12, NS-24.** "Eliminar" un dispositivo con historial sólo lo
oculta del panel; su secreto sigue autenticando. Revocar o suspender tarda
hasta 12 h (el TTL subió de 1 h a 12 h después de que la auditoría del 10-09
pidiera 5-15 min) porque ninguna política RLS revisa `revoked_at`/`suspended_at`.
**Acción:** que "eliminar" también fije `revoked_at`; revisar el estado del
dispositivo en las políticas (o una función `dispositivo_vigente()`), o bajar el
TTL con renovación silenciosa.

### 5. Aislamiento entre sitios y cuentas en Supabase — Alta
- **NS-04 (pendiente A-01):** cualquier dispositivo, incluso `visor`, puede
  escribir `contratistas.activo` y `empresas` de todos los sitios.
- **NS-05 (condicionada):** el login une la persona sólo por el correo
  sintético `cedula@brisas.local`, sin comparar `auth_user_id`. El `config.toml`
  versionado tiene el registro público abierto y sin confirmación de correo.
  **Verificar hoy en el dashboard de producción que el registro público esté
  desactivado.**
- **NS-14, NS-15, NS-23:** autorización administrativa sólo por `auth.email()`,
  OTP sólo en la interfaz, `plegar_texto` sin `search_path` fijo.

### 6. Escritorio: congelamientos, archivos y updater — Alta/Media
- **DT-01 (Alta):** 75 comandos síncronos corren en el hilo de la UI:
  exportación a Excel (≈33 s con 100 000 filas), Argon2 y llamadas de red.
  Los comentarios de `estado.rs:169` e `historial.rs:289` dicen lo contrario.
  **Acción:** `#[tauri::command(async)]` o `async fn` + `spawn_blocking`.
- **DT-02 / DF-19 (Alta):** los comandos de exportación escriben en cualquier
  ruta que envíe el webview. **Acción:** que el diálogo "Guardar como" lo abra
  el backend, o validar extensión y carpeta, y nunca permitir `db_key.dat`.
- **DT-03 (Media):** actualizar `tauri-plugin-updater` a 2.12.0.
- **DT-04, DT-05:** sesión de Supabase no ligada al usuario local (cambio de
  contraseña sobre la cuenta equivocada en una carrera); sesión local sin
  vencimiento.
- **DT-06:** la reconstrucción de una base dañada ignora `-wal`/`-shm`.
- **DT-07:** Sentry recibe un evento cada 2 minutos sin sesión.
- **DT-10:** instaladores de Windows sin firma Authenticode.

### 7. Login local sin freno — Media
**NR-06, DT-08.** Sin límite de intentos, sin registro de fallos y con mensajes
distintos que permiten saber qué cédulas existen o están inactivas.
**Acción:** retraso exponencial por cédula, mensaje único, evento de auditoría.

### 8. Reglas de negocio que sólo aplica la interfaz — Media
**DF-06, DF-07, DF-14, NS-08, MV-04, NR-08, NR-09, NR-10.** Gafete ocupado en
otro dispositivo no se revisa para proveedores ni KOF; la columna "Acceso" se
oculta al Operador sólo visualmente; el tope de 200 gafetes por rango sólo está
en el cliente; el chequeo "activo en otro sitio" nunca encuentra nada porque la
RLS lo impide; cédulas y placas sólo pasan por `trim`, así que un contratista
denegado puede volver a darse de alta cambiando el formato.

### 9. Migraciones y apertura de base — Media
**NR-02, NR-03, NR-04.** `PRAGMA foreign_key_check` corre después del `COMMIT`
en 7 migraciones; una base más nueva o una migración rechazada se reporta como
"corrupción" y abre el diálogo "reconstruir desde la nube", que borra la
auditoría local; nada verifica en tiempo de ejecución que el motor cifre.

### 10. Acoplamiento y duplicación — Media/Baja
- **NS-17 / MV-12 / DT-19:** la orquestación de la sincronización está copiada
  en 4 lugares (núcleo, escritorio y dos veces en móvil), con deriva real que ya
  produjo un bug documentado. Moverla a una sola función en `application/nube.rs`.
- **NS-18:** `sincronizacion.rs` (8 140 líneas) mezcla transporte, cola,
  recepción, consultas y construcción de cuerpos.
- **NR-18:** ciclo `domain`↔`models` y la regla PRAIND duplicada en SQL.
- **MV-09:** mutaciones del núcleo dentro de Composables.
- **DF-21:** `App.tsx` como orquestador grande.

### 11. Frontend de escritorio y modales nuevos — Media
**DF-01, DF-02, DF-04, DF-05.** La placa de los ingresos remotos nunca llega (el
DTO Rust no la envía); carreras de respuestas fuera de orden en
`NuevoIngresoModal` y `GestionGafeteModal` que pueden registrar el ingreso a otra
persona; `Modal` sin `role="dialog"` ni foco atrapado, Escape en `window` cierra
todos los modales apilados y los atajos siguen activos con un modal abierto;
CSV de AG Grid sin neutralizar fórmulas.

### 12. Móvil: robustez y OCR — Media
**MV-05, MV-06, MV-07, MV-13.** Cierre de la app si falla el Keystore al elegir
un contratista; en escaneo continuo la salida se registra sola por OCR del gafete
sin confirmación ni dígito verificador (un dígito mal leído dos veces cierra el
ingreso de otra persona); cuatro conversiones de imagen por frame; panics de
Rust que cruzan UniFFI tumban la app.

### 13. CI y pruebas — Media
**HT-09, HT-10, NS-26, MV-17, MV-21, DF-09.** Los tests del núcleo nunca corren
con SQLite3MC (el motor de producción) y los de `mobile/rust-core` no corren en
CI; sin pruebas de RLS ni de edge functions; los secretos del keystore Android
están en el `env` de todo el job sin `environment` protegido; herramientas del
release sin versión fija; la configuración de zizmor suprime sus 61 hallazgos.

---

## Código muerto (inventario consolidado)

| Elemento | Ubicación | Acción sugerida |
|---|---|---|
| Módulo `lenguaje_comandos` completo (≈1 430 líneas + 38 tests) y `AppCore::buscar_auditoria` | `src/lenguaje_comandos/` | Eliminar (HT-02) |
| `fijar_password_inicial` — fija la contraseña de cualquier cuenta sólo con la cédula | núcleo / nube | **Eliminar primero**: es peligroso si alguien lo reconecta (NR-07, NS-25) |
| `resetear_password_root`, `cambiar_mi_password_con_hash` y otras 9 funciones `pub` sin consumidor; 13 usadas sólo por tests | núcleo | Eliminar o bajar a `pub(crate)` (HT-03, NR-07) |
| `verificar_token_offline`, `obtener_jwks`, varios métodos de `application/nube.rs` | `src/nube/`, `src/application/nube.rs` | Eliminar (NS-25) |
| 12 métodos UniFFI sin uso, incluido `guardar_secreto_dispositivo` (formato débil derivado de `ANDROID_ID`) | `mobile/rust-core/src/lib.rs` | Eliminar (MV-11) |
| Comando `buscar_encargados_ruta_provisional` (registrado, nunca invocado) y `cambiar_mi_password` | `desktop/src-tauri` | Eliminar (DT-21, DF-17) |
| 14 exports y 10 tipos TS sin uso (knip) | `desktop/src` | Eliminar (DF-17) |
| Dependencia `serde_json` | `desktop/src-tauri/Cargo.toml` | Quitar (HT-06) |
| `build.rs`/`winresource`, perfil `release-native`, feature `dev-auth`, README de la TUI, `packaging/msix` (empaqueta un binario inexistente) | raíz | Eliminar o actualizar (HT-04, NR-17) |
| Workflows `db-cipher-e2e.yml` y `db-cipher-lab.yml` (apuntan a `experiments/`, que no existe) | `.github/workflows/` | Eliminar o arreglar (HT-07) |
| Ramas `#[cfg(not(windows))]` rotas | `desktop/src-tauri` | Arreglar o borrar (HT-05) |
| Base iOS (un solo commit, sin conexión con el núcleo, sin CI) | `mobile/ios/` | Decidir: mantener o retirar (MV-25) |
| Variante muerta de la guardia de reloj | núcleo | Eliminar (NR-12) |

## Rutas y visitas (en pausa)

Aunque estén en pausa, **sí afectan** a lo que está en producción:
- **NS-01** (salidas de ruta) puede detener la sincronización de todo lo demás.
  Es lo primero que hay que corregir.
- **NS-03** también afecta a visitas y salidas de ruta.
- **NS-10:** las citas no propagan bajas; un visitante quitado sigue autorizado
  localmente.
- **NR-09:** altas por rango de rutas sin tope.
- **DF-23:** las pantallas en pausa repiten los patrones de carreras de DF-10.

Si no se van a retomar pronto, conviene desactivar el encolado de `salida_ruta`
y `cita` detrás de una bandera en lugar de dejar el código vivo a medias.

## Documentación desactualizada
**HT-14, MV-24, NR-17.** `auditoria-calidad-2026-09.md` dice "cero `unsafe`" y
"sin código muerto" (ya no es cierto); `docs/pendientes.md` lista como
pendientes upgrades ya aplicados (`jsonwebtoken` 11, `argon2` 0.6); varios
comentarios de seguridad describen un cifrado móvil que no existe.

---

## Plan de acción sugerido

**Inmediato (esta semana)**
1. NS-01: pánico de `salida_ruta` (cambio de pocas líneas + prueba).
2. NS-05: confirmar en el dashboard de Supabase que el registro público está desactivado.
3. NS-02: "Eliminar" dispositivo también revoca.
4. DT-03: `tauri-plugin-updater` → 2.12.0.
5. MV-01 / DF-03: no cachear la contraseña temporal y exigir el cambio en el backend.
6. Eliminar `fijar_password_inicial` y las demás APIs de contraseña sin uso.

**Corto plazo (2-4 semanas)**
7. NR-01 / MV-03: cifrar la base Android y agregar `dataExtractionRules`.
8. NS-03, NS-06, NS-09: correcciones de integridad de la sincronización.
9. NS-04, NS-07: RLS de escritura por sitio y revalidación del dispositivo.
10. DT-01, DT-02: comandos fuera del hilo de UI y rutas de exportación controladas.
11. NR-06 / DT-08: freno al login local.
12. HT-10: tests del núcleo con SQLite3MC y tests de `mobile/rust-core` en CI.

**Medio plazo**
13. Unificar la orquestación de la sincronización (NS-17) y dividir `sincronizacion.rs` (NS-18).
14. Accesibilidad y carreras en modales (DF-02, DF-04).
15. Limpieza de código muerto según el inventario.
16. Pruebas de RLS y edge functions en CI (NS-26).

## Aspectos bien resueltos

- SQL siempre parametrizado; no se encontró inyección SQL.
- Exportación XLSX y HTML del PDF correctamente escapados.
- Argon2id para contraseñas; transacciones `IMMEDIATE`; triggers de historial inmutable.
- Todos los comandos Tauri exigen sesión y el núcleo revalida el usuario activo al escribir.
- Clave de la base de escritorio protegida con DPAPI y nunca regenerada sobre una base existente.
- Tokens sólo en memoria; Realtime con clave publicable y token de corta duración.
- CSP y capabilities de Tauri estrictas; ningún punto de entrada XSS en el frontend.
- Secreto del dispositivo Android con AES-GCM y clave del Keystore; `FLAG_SECURE` y R8 activos.
- Las imágenes de cédula nunca se escriben a disco; ningún ViewModel guarda `Context`.
- Los 10 bloques `unsafe` propios están justificados y documentados.
- No hay claves `service_role` en el árbol ni en el historial de git.
