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

## Decisiones de negocio pendientes (no es código, es elegir)

- [ ] **Correo para el diagnóstico crítico** — ¿creamos cuenta en Resend (gratis para este volumen) para que el zip de diagnóstico se mande solo? *(Vos dijiste "opción A" — falta el ok final para crear la cuenta)*
- [ ] **Ambiente de staging** — confirmar que armamos el 2do proyecto Supabase gratis para probar antes de un release real *(dijiste que te gusta la idea — falta confirmar que arrancamos)*
- [ ] **Sentry (error-tracking automático)** — ¿lo sumamos o nos alcanza con el archivo de log local por ahora? Nivel gratis alcanza para este volumen, pero es una cuenta externa más
- [ ] **Revisión de código obligatoria en GitHub** — ¿activamos "no dejar mergear sin aprobación" en `main`? Es un checkbox en Settings del repo, no código — pero lo tenés que activar vos o decirme que lo haga
- [ ] **Runbook de base local dañada** — falta que confirmes qué tablas es aceptable perder si un sitio tiene que reconstruirse desde la nube (ver plan, punto 7) antes de poder diseñar la pantalla

## Trabajo técnico pendiente (no necesita tu decisión, solo tiempo)

- [ ] Chequeo de versión mínima entre dispositivos (diseño ya está claro, ver plan punto 9)
- [ ] Diagnóstico exportable (.zip) — el botón + armar el archivo (espera el ok de Resend de arriba para la parte de "mandarlo solo")
- [ ] Instrumentar más puntos de fallo con logs (comandos Tauri, cola de sincronización offline — plan punto 5.1/5.2)
- [ ] Logging del fallo fatal de arranque (base dañada, candado de instancia) — hoy corre antes de que el log exista (plan punto 5.3)
- [ ] Cifrado del secreto de dispositivo en Android — **verificado de nuevo hoy 2026-09-17: sigue sin hacer**, `ANDROID_ID` en texto plano tal cual documentaba la auditoría de septiembre
- [ ] Variables de entorno para `web`/`web-visitas` (para poder apuntar a staging sin editar código) — depende de que el proyecto de staging ya exista

## Descartado a propósito (no reabrir sin una razón nueva)

- [x] ~~Backups tradicionales~~ — decisión de arquitectura ya tomada, la nube es el respaldo
- [x] ~~Pruebas de carga clásicas~~ — no es un servidor multi-usuario
- [x] ~~Feature flags de producto~~ — sin caso de uso real hoy
- [x] ~~Migraciones "down"/reversibles~~ — la atomicidad transaccional ya cubre el riesgo real
