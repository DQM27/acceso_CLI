# Registro de dispositivos — revisión y propuesta simplificada (2026-09-29)

> Revisión del flujo real de alta, autenticación, suspensión y baja de
> dispositivos (escritorio, móvil y panel web), contrastada contra
> `plan-sesion-unica-dispositivos.md` y contra prácticas actuales de la
> industria. Propone **reemplazar** los puntos 1–5 de ese plan por un
> modelo más simple que resuelve los mismos problemas con menos piezas.
> Nada de esto está implementado todavía.

## 1. Quién controla a quién

**El panel web es la única autoridad.** Escritorio y móvil nunca se dan de
alta solos: sólo *canjean* algo que el panel emitió. Ese principio ya existe
hoy y se mantiene; lo que cambia es *qué* emite el panel y *qué* guarda el
dispositivo.

```
Panel web (admin, Supabase Auth)          Supabase (Edge Functions + Postgres)         Escritorio / Móvil
────────────────────────────────          ────────────────────────────────────         ─────────────────
Crear dispositivo / Re-vincular  ───────▶  emite código de un solo uso (15 min)
Muestra código + QR                                                                    Persona escribe/escanea código
                                           canje atómico  ◀──────────────────────────  código + clave pública + metadata
                                           guarda clave pública, quema el código  ───▶ ok (dispositivo_id, sitio_id)
                                                                                       ...
                                           device-auth: verifica firma  ◀───────────── aserción firmada con clave privada
                                           JWT corto (1 h)  ─────────────────────────▶
Suspender / Revocar  ───────────────────▶  marca estado + broadcast "expulsar"  ──────▶ cierra canal y deja de operar
```

## 2. Cómo funciona hoy (verificado en el código)

| Pieza | Dónde | Qué hace |
| --- | --- | --- |
| Alta | `supabase/functions/admin-provision-device` | Crea la fila en `dispositivos`, genera un secreto (2×UUID), guarda sólo su SHA-256 y lo devuelve **una vez** al panel. |
| Panel | `web/src/pantallas/Dispositivos.tsx` | Muestra el secreto para copiar/pegar. Sitio se elige **por nombre** y se hace `upsert` por nombre. |
| Escritorio | `PrimerArranque.tsx` → `comandos/nube.rs::configurar_dispositivo_inicial` → `AppCore::configurar_dispositivo_inicial` | Pega el secreto, lo guarda con DPAPI (`nube/credenciales.rs`), llama `device-auth` con metadata de la PC y baja el catálogo. |
| Móvil | `PantallaPrimerArranque.kt` → `PrimerArranqueViewModel` → `Nucleo::configurar_dispositivo_inicial_con_secreto` | Igual, con el secreto cifrado por Android Keystore (`SecretoDispositivoStore.kt`). |
| Autenticación | `supabase/functions/device-auth` | Busca por `secret_hash`, rechaza revocado/suspendido/versión vieja, firma un JWT ES256 propio de **12 h** con `sitio_id`/`tipo`. |
| Caché | `src/nube/cache_token.rs` | Reusa el JWT hasta 15 min antes de vencer; el secreto crudo viaja en cada renovación. |
| Baja | `admin-revoke-device` / `admin-suspend-device` / `admin-delete-device` | Sólo marcan columnas; `device-auth` deja de emitir tokens nuevos. |

### Hallazgos

| # | Severidad | Hallazgo | Evidencia |
| --- | --- | --- | --- |
| H1 | Alta | **El secreto es una credencial al portador reutilizable sin límite.** Quien lo copie (portapapeles, captura de pantalla del panel, chat donde se envió) puede activar N equipos que el sistema ve como el mismo dispositivo. Es el problema original del plan de sesión única. | `device-auth` sólo compara `secret_hash`. |
| H2 | Alta | **Revocar o suspender no corta el acceso hasta 12 h.** Ninguna política RLS consulta `revoked_at`/`suspended_at`; el JWT ya emitido sigue escribiendo `ingresos`, `contratistas`, etc. hasta que vence. | `grep revoked_at supabase/migrations` sólo aparece en el esquema inicial y la migración de suspensión; `TOKEN_TTL_SECONDS = 12 h`. |
| H3 | Media | **La metadata de hardware se sobrescribe en silencio.** Si un clon se activa con el mismo secreto, pisa `identificador_hardware`/`nombre_dispositivo` del equipo legítimo: se pierde justo la evidencia que el plan quería conservar. | `device-auth`, bloque `actualizacion`. |
| H4 | Media | **Re-vincular un equipo exige vaciar su base local.** `configurar_dispositivo_inicial` falla con `YaConfigurado` si hay usuarios; no hay forma de cambiar la credencial de un equipo ya configurado sin perder la bandeja de salida no sincronizada. | `application/nube.rs::configurar_dispositivo_inicial`, `docs/decisiones-tecnicas.md` (retiro de `guardarSecretoDispositivo`). |
| H5 | Media | **El sitio se elige por nombre con `upsert`.** Un error de tipeo en el panel crea un sitio nuevo en vez de fallar. | `admin-provision-device`, `upsert({ nombre })`. |
| H6 | Baja | **`correoAdminAutorizado` está copiado en 8 Edge Functions.** Un cambio de criterio de autorización hay que replicarlo a mano. | `grep "async function correoAdminAutorizado" supabase/functions`. |
| H7 | Baja | **La actualización de `last_seen_at`/metadata es "fire and forget"** tras devolver la respuesta; el runtime de Edge Functions puede cortarla. Debería usar `EdgeRuntime.waitUntil`. | `device-auth`, `.then(() => {})`. |
| H8 | Baja | **Un secreto largo es mala experiencia en móvil**: se copia desde el panel y se pega a mano en el celular (normalmente enviándolo por chat, lo que agrava H1). | `Dispositivos.tsx`, `copiarSecreto`. |

## 3. Por qué el plan actual se siente complejo

`plan-sesion-unica-dispositivos.md` intenta cerrar H1 **manteniendo un
secreto al portador** y parchándolo con piezas encadenadas:

1. tabla de "secretos usados" para detectar reuso,
2. rotación del secreto en cada autenticación (tipo *refresh token*),
3. registro forense + alerta por correo ante reuso,
4. desempate offline por fecha de alta + expulsión automática,
5. (en `pendientes.md`) verificación por correo en la primera activación.

Cada pieza existe porque la anterior deja un hueco: la rotación hace falta
porque el secreto se puede copiar; el desempate hace falta porque con
rotación y equipos offline dos copias pueden divergir; la alerta hace falta
porque el reuso sólo se *detecta*, no se *impide*. Además, la rotación tiene
un modo de falla propio y serio para un equipo de portería: si el servidor
rota el secreto y la respuesta no llega (corte de red, app cerrada antes de
persistir), **el equipo legítimo queda bloqueado** y hay que re-vincularlo
—lo que hoy implica vaciar su base (H4).

La raíz es una sola: **la credencial viaja y se puede copiar.** Si se quita
eso, la mayoría de las piezas sobran.

## 4. Propuesta

Tres cambios, cada uno independiente y entregable por separado.

### 4.1 Código de vinculación de un solo uso (reemplaza al secreto pegado)

- El panel ya no entrega un secreto permanente, sino un **código de
  vinculación**: 10 caracteres legibles (alfabeto sin `0/O/1/I`, ej.
  `K7QM-R4XT-2P`) más su **QR**, con vigencia corta (15 min por defecto).
- Nueva tabla `codigos_vinculacion` (`dispositivo_id`, `codigo_hash`,
  `expira_en`, `usado_en`, `creado_por`). El canje es **atómico** en un
  solo `UPDATE … WHERE usado_en IS NULL AND expira_en > now() RETURNING …`:
  dos equipos no pueden canjear el mismo código ni en carrera.
- Canje fallido (código usado, vencido o inexistente) → fila en
  `intentos_vinculacion` con IP (vista por el servidor), metadata enviada y
  motivo, visible en el panel. Esto sustituye al "registro forense" del plan
  con una tabla simple, sin alertas por correo como requisito de V1.
- **Re-vincular** es un botón del panel sobre un dispositivo existente:
  emite un código nuevo para el *mismo* `dispositivo_id` (conserva historial
  y FKs) e invalida la credencial anterior. En el cliente, la pantalla de
  vinculación debe poder abrirse también con base ya configurada (resuelve
  H4) siempre que la bandeja de salida se conserve.
- En el panel, el sitio se elige **por `id`** desde el desplegable, nunca
  por nombre libre (resuelve H5).

Es el mismo patrón que usan Android Enterprise e Intune para equipos
dedicados/kiosco: un *enrollment token* corto, revocable y en QR, emitido
por la consola central.

### 4.2 Credencial = par de claves generado en el propio equipo

- Durante el canje, el equipo **genera un par de claves EC P-256** y envía
  sólo la **clave pública**; el servidor la guarda en
  `dispositivos.clave_publica_jwk`. La clave privada nunca sale del equipo:
  - **Android:** Android Keystore (TEE/StrongBox), marcada no exportable. Ya
    existe experiencia con Keystore en `SecretoDispositivoStore.kt`.
  - **iOS:** Secure Enclave (soporta P-256 de forma nativa).
  - **Windows:** fase 1 con clave de software protegida con DPAPI (mismo
    nivel que el secreto actual); fase 2 opcional con el proveedor TPM de
    CNG (*Microsoft Platform Crypto Provider*).
- `device-auth` v2 recibe una **aserción firmada** (JWT corto: `iss =
  dispositivo_id`, `aud = device-auth`, `iat`, `exp ≤ 60 s`, `jti`),
  verifica la firma con la clave pública guardada y emite el JWT de sesión
  igual que hoy. Es el patrón *private_key_jwt* (RFC 7523) usado para
  autenticar clientes OAuth sin secretos compartidos.
- En el núcleo Rust, la firma se abstrae detrás de un trait (ej.
  `FirmanteDispositivo`) que cada plataforma implementa (Kotlin vía callback
  de UniFFI, Windows en Rust). El núcleo no conoce el almacén de claves.

**Qué es compartido y qué es por plataforma.** Escritorio y móvil siguen el
mismo flujo y usan las mismas funciones del núcleo; sólo cambia dónde vive
la clave privada:

| Responsabilidad | Dónde | Compartido |
| --- | --- | --- |
| Armar la aserción (header + claims, `jti`, `exp`, base64url, JWS final) | Núcleo Rust (`nube::`) | Sí |
| Canjear el código de vinculación y enviar la clave pública | Núcleo Rust | Sí |
| Pedir el token a `device-auth`, caché y reintentos | Núcleo Rust (`cache_token.rs`, ya existente) | Sí |
| Generar el par de claves y devolver la clave pública (JWK) | `FirmanteDispositivo::clave_publica` | No: una implementación por plataforma |
| Firmar bytes con la clave privada | `FirmanteDispositivo::firmar` | No: una implementación por plataforma |

```rust
/// Implementado por cada plataforma; el núcleo sólo le pide firmar.
pub trait FirmanteDispositivo: Send + Sync {
    /// Genera el par si no existe y devuelve la clave pública (JWK P-256).
    fn clave_publica(&self) -> Result<String, ErrorFirmante>;
    /// Firma ES256 (r||s, 64 bytes) sobre `datos` con la clave privada.
    fn firmar(&self, datos: &[u8]) -> Result<Vec<u8>, ErrorFirmante>;
}
```

- **Escritorio:** implementación en Rust dentro del núcleo (crate `p256`,
  clave protegida con DPAPI igual que el secreto actual en
  `nube/credenciales.rs`; más adelante, TPM por CNG).
- **Android:** implementación en Kotlin sobre Android Keystore, expuesta al
  núcleo como *callback interface* de UniFFI. Sería el primer callback del
  proyecto (hoy `mobile/rust-core` sólo expone funciones hacia Kotlin).
- **iOS:** misma interfaz, implementada en Swift sobre Secure Enclave.

Se descarta generar la clave en Rust también en móvil y guardarla cifrada
con Keystore: simplificaría el puente, pero la clave privada pasaría por la
memoria de la app y sería exportable, perdiendo la garantía principal de
esta propuesta. La firma de la aserción no puede usar `jsonwebtoken` (ya
dependencia del núcleo) porque esa crate necesita la clave privada en
memoria; el núcleo arma `header.payload` y delega sólo la firma.

**Qué elimina esto del plan actual:**

| Pieza del plan | ¿Sigue haciendo falta? | Motivo |
| --- | --- | --- |
| Tabla de secretos usados | No | El código se quema en el canje atómico. |
| Rotación del secreto en cada uso | No | No hay secreto que robar en tránsito; la clave privada no es exportable. Se evita además el bloqueo del equipo legítimo por respuesta perdida. |
| Desempate offline por fecha + expulsión | No | Dos equipos no pueden compartir identidad: el canje es único y la clave no se copia. |
| Verificación por correo al activar | No (opcional) | El código lo genera un admin autenticado, dura minutos y normalmente está presente. |
| Registro forense de reuso | Simplificado | Queda como `intentos_vinculacion`. |
| Presencia en tiempo real | Se mantiene | Ya funciona; no cambia. |

La rotación de *refresh tokens* que recomienda RFC 9700 es la alternativa
para clientes que **no** pueden restringir la credencial al emisor; la misma
norma acepta como equivalente que la credencial esté ligada al remitente
(*sender-constrained*), que es exactamente lo que da un par de claves.

### 4.3 Revocación que corta de verdad

- **Expulsión inmediata:** al suspender/revocar, la Edge Function emite un
  broadcast `dispositivo_revocado` en `sitio:{id}`; el cliente, que ya
  escucha ese canal, borra el token cacheado y pasa a pantalla de bloqueo.
- **Red de seguridad sin Realtime:** bajar `TOKEN_TTL_SECONDS` de 12 h a
  1 h. El motivo original del aumento (tokens vencidos a mitad de
  sincronización) ya está cubierto por el reintento automático de
  `token_dispositivo_vencido` y por el margen de 15 min de `cache_token.rs`;
  renovar es barato y ocurre sólo bajo demanda.
- **Escrituras:** agregar `private.dispositivo_vigente()` (lookup por PK de
  `auth.jwt()->>'sub'` sobre `revoked_at`/`suspended_at`) a las políticas de
  `INSERT`/`UPDATE` de las tablas espejo. Lecturas sin cambio para no
  encarecer Realtime.

## 5. Plan por fases

| Fase | Contenido | Resuelve | Esfuerzo aprox. |
| --- | --- | --- | --- |
| **F0 — Correcciones rápidas** | `_shared/autorizacion_admin.ts` (H6); sitio por id (H5); no sobrescribir `identificador_hardware` si ya existe y registrar discrepancia (H3); `EdgeRuntime.waitUntil` (H7); broadcast de expulsión + TTL 1 h + `dispositivo_vigente()` en escrituras (H2). | H2, H3, H5, H6, H7 | 2–3 días |
| **F1 — Códigos de vinculación** | Tabla + `admin-crear-codigo-vinculacion` + `device-vincular`; panel con QR, cuenta regresiva, estados *Pendiente / Vinculado / Suspendido / Revocado* y botón *Re-vincular*; escáner QR en Android (`mlkit:barcode-scanning` ya es dependencia); escritorio escribe el código. En esta fase el canje todavía devuelve un secreto (compatibilidad), pero de un solo uso. | H1 (activación), H4, H8 | 4–5 días |
| **F2 — Par de claves** | Trait `FirmanteDispositivo` en el núcleo; implementaciones Android/iOS/Windows; `device-auth` v2 con aserción firmada; migración transparente (ver §6). | H1 completo | 1–1,5 semanas |
| **F3 — Opcional** | Clave en TPM en Windows; *key attestation* en Android para rechazar emuladores o equipos rooteados. | Endurecimiento | según necesidad |

## 6. Migración sin re-vincular equipos en campo

1. `device-auth` acepta ambos caminos: aserción firmada (nuevo) o secreto
   (legado, sólo para filas con `clave_publica_jwk IS NULL`).
2. Un cliente actualizado que todavía se autentica por secreto genera su par
   de claves y llama una sola vez a `device-registrar-clave` con su JWT
   vigente; el servidor guarda la clave pública y pone `secret_hash = NULL`.
3. El panel muestra qué equipos siguen en modo legado. Cuando no quede
   ninguno (o tras una fecha límite, usando `VERSION_MINIMA_ACEPTADA`), se
   elimina el camino por secreto.

## 7. Fuera de alcance

- **Sesión única por persona/sitio** (sección 7 del plan anterior y su
  entrada en `pendientes.md`): es identidad de *personas*, no de equipos. Se
  mantiene como tema aparte.
- **Clave SQLCipher por dispositivo en Vault** (`pendientes.md`): encaja
  bien después de F2 —la Edge Function que entregue la clave puede exigir
  la misma aserción firmada— pero no bloquea esta propuesta.

## 8. Referencias

- RFC 9700, *Best Current Practice for OAuth 2.0 Security* (rotación vs.
  credenciales ligadas al remitente): https://datatracker.ietf.org/doc/rfc9700/
- RFC 7523, *JWT Profile for OAuth 2.0 Client Authentication* (`private_key_jwt`):
  https://datatracker.ietf.org/doc/rfc7523/
- Android Keystore (claves no exportables): https://developer.android.com/privacy-and-security/keystore
- Android key attestation: https://developer.android.com/privacy-and-security/security-key-attestation
- Android Management API, tokens de enrolamiento con QR: https://developers.google.com/android/management/provision-device
- Intune, enrolamiento de equipos dedicados/kiosco: https://learn.microsoft.com/en-us/intune/intune-service/enrollment/android-kiosk-enroll
