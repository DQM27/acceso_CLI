# Checklist QA — para ir tildando (2026-09-17)

> Versión resumida y accionable de
> `docs/auditorias/plan-qa-buenas-practicas-2026-09-17.md` — ese documento
> tiene el detalle técnico completo de cada punto (por qué importa, qué
> se hizo, cómo se verifica). Este archivo es solo para llevar el pulso de
> qué falta, tildando a medida que se resuelve. Actualizar los dos en
> paralelo, no solo uno.

## Ya implementado (rama `qa`, no mergeado a `main` todavía)

- [x] Versión instalada visible en el sidebar de escritorio
- [x] `cargo fmt --check` corriendo en los 3 jobs de CI (antes solo 1 de 3)
- [x] Logs activados en producción (antes solo en modo debug)
- [x] Fallo silencioso corregido en la sincronización automática (errores y panics que no dejaban rastro)
- [x] Código muerto del viejo sistema de respaldo eliminado + nota falsa en `pendientes.md` corregida
- [x] **Corrección 2026-09-17 (tarde):** el secreto de dispositivo en Android **sí está cifrado** (Android Keystore, `SecretoDispositivoStore.kt`, desde el commit `7aed199` del 2026-09-10) -- la entrada de más abajo, agregada hoy más temprano, estaba mal: busqué solo `androidx.security`/`EncryptedFile` y no encontré el mecanismo real, que usa el API de Keystore directo. Verificado de punta a punta: la clave ya no depende de `ANDROID_ID` (se genera dentro del propio Keystore), `ANDROID_ID` solo se usa para migrar el secreto legado en texto plano una única vez, y el archivo viejo se borra después. Conectado de verdad en `AplicacionViewModel.kt:52`, no es código muerto.
- [x] `CODEOWNERS` agregado (`.github/CODEOWNERS`, @DQM27) -- falta el interruptor en GitHub, ver más abajo
- [x] Chequeo de versión mínima entre dispositivos -- **escritorio completo**, con tests (`cargo test --lib`, 365 passed). Mobile queda pendiente, ver más abajo

## Decisiones de negocio pendientes (no es código, es elegir)

- [ ] **Correo para el diagnóstico crítico** — ¿creamos cuenta en Resend (gratis para este volumen) para que el zip de diagnóstico se mande solo? *(Vos dijiste "opción A" — falta el ok final para crear la cuenta)*
- [ ] **Ambiente de staging** — confirmar que armamos el 2do proyecto Supabase gratis para probar antes de un release real *(dijiste que te gusta la idea — falta confirmar que arrancamos)*
- [x] **Sentry (error-tracking automático) -- conectado en ambas plataformas (2026-09-17).** Cuenta creada por el usuario, dos proyectos (`control-acceso-desktop`, `control-acceso-mobile`) vía el MCP de Sentry. **Ojo con la cuenta:** quedó en trial del plan Business (13 días), sin tarjeta cargada -- al vencer baja sola al plan gratis real porque no hay forma de cobrar. No tocar el botón "Confirmar" de la pantalla de planes mientras tanto (activaría el plan pago). Ver detalle técnico en `plan-qa-buenas-practicas-2026-09-17.md`, punto 5.4.
- [ ] **Revisión de código obligatoria en GitHub** — el archivo `CODEOWNERS` ya está (`.github/CODEOWNERS`, @DQM27 como revisor por defecto). Falta el interruptor, que **no tengo forma de activar yo** (no hay herramienta para tocar configuración del repo en este entorno) — vos lo hacés en 1 minuto: GitHub → repo → **Settings → Branches → Add branch protection rule** → rama `main` → tildar **"Require a pull request before merging"** + **"Require review from Code Owners"** → Save.
  ⚠️ **Ojo con esto:** una vez activado, ni siquiera vos podés pushear directo a `main` (ni yo, cuando trabajo con tu cuenta) — todo cambio, sin excepción, tiene que pasar por PR + tu propia aprobación como Code Owner. Si eso te complica el flujo del día a día, hay una casilla "Do not allow bypassing the above settings" que podés dejar SIN marcar para que el dueño del repo pueda saltarse la regla en un apuro.
- [ ] **Runbook de base local dañada** — falta que confirmes qué tablas es aceptable perder si un sitio tiene que reconstruirse desde la nube (ver plan, punto 7) antes de poder diseñar la pantalla
- [ ] **Verificar que Sentry realmente notifica** — por defecto manda email al primer error nuevo de un tipo (no en cada repetición), pero no se confirmó de punta a punta (el MCP no expone la API de alert rules, quedó deprecada del lado de Sentry). Entrar a **Settings → Alerts** de cada proyecto (`control-acceso-desktop`/`control-acceso-mobile`) y a **Settings → Notifications** de la cuenta para confirmar que el canal de aviso (email u otro) está activo -- forzar un error de prueba es la forma más segura de confirmarlo antes de depender de esto en un sitio real

## Trabajo técnico pendiente (no necesita tu decisión, solo tiempo)

- [ ] **Chequeo de versión mínima en mobile** — falta que Kotlin le pase su versión real a Rust (nuevo método UniFFI + una llamada en `AplicacionViewModel.kt`). Chico y de bajo riesgo, pero toca Kotlin — decime si avanzo
- [ ] Configurar `VERSION_MINIMA_ACEPTADA` en Supabase cuando decidan la primera versión a exigir (`supabase secrets set VERSION_MINIMA_ACEPTADA=X.Y.Z`) -- sin esto seteado, el chequeo ya está listo pero no rechaza nada
- [ ] Diagnóstico exportable (.zip) — el botón + armar el archivo (espera el ok de Resend de arriba para la parte de "mandarlo solo")
- [x] **Instrumentar más puntos de fallo con logs, escritorio y mobile (2026-09-17).** Ver detalle técnico en `plan-qa-buenas-practicas-2026-09-17.md`, puntos 5.1/5.2/5.3.
- [x] **Logging del fallo fatal de arranque (2026-09-17).** Archivo propio en `%LOCALAPPDATA%\<identifier>\logs\fallo-fatal-arranque.log`, sin depender del plugin (que a esa altura no existe todavía) -- más `sentry::capture_message` (ya inicializado en ese punto). Ver punto 5.3.
- [ ] ~~Cifrado del secreto de dispositivo en Android~~ **Ya está hecho — ver corrección arriba.** Lo único real que falta: no hay test automatizado de `AndroidKeystoreSecretoDispositivoStore` (no se puede sin Robolectric, que el proyecto no tiene) — decidir si vale la pena sumarlo
- [ ] Variables de entorno para `web`/`web-visitas` (para poder apuntar a staging sin editar código) — depende de que el proyecto de staging ya exista

## Descartado a propósito (no reabrir sin una razón nueva)

- [x] ~~Backups tradicionales~~ — decisión de arquitectura ya tomada, la nube es el respaldo
- [x] ~~Pruebas de carga clásicas~~ — no es un servidor multi-usuario
- [x] ~~Feature flags de producto~~ — sin caso de uso real hoy
- [x] ~~Migraciones "down"/reversibles~~ — la atomicidad transaccional ya cubre el riesgo real
