# Auditoría adversarial de seguridad — Web de Visitas

**Fecha:** 2026-09-10  
**Sistema:** `acceso_CLI` / `web-visitas` / Supabase / Cloudflare  
**Rama de auditoría:** `audit/seguridad-adversarial-web-visitas-2026-09-10`  
**Estado:** **NO APTO todavía para el benchmark adversarial final**  
**Motivo principal:** fallo crítico de autorización por alcance global en RLS para identidades autenticadas con `sitio_id`.

> Este documento es una auditoría defensiva. Durante la revisión del entorno vivo sólo se realizaron consultas de lectura y una prueba de aislamiento en una transacción `READ ONLY`. No se efectuaron escrituras de explotación, cambios de datos, revocaciones ni pruebas de carga contra producción.

---

## 1. Veredicto ejecutivo

La arquitectura tiene varias defensas buenas y ya corrigió problemas importantes del reporte anterior: CSP estricta, ausencia de sourcemaps, validación cliente + servidor, RPC atómica/idempotente para crear citas, RLS activa en las tablas de visitas, límites de tamaño en la RPC, timeouts de base de datos y funciones administrativas que vuelven a validar identidad antes de usar privilegios elevados.

Sin embargo, **no recomiendo someter todavía producción al benchmark definitivo**. La razón es un problema de autorización más importante que un XSS o un header faltante:

- Las políticas de `usuarios`, `contratistas` y `empresas` tratan como “global” a cualquier sesión `authenticated` cuyo JWT tenga un `sitio_id`.
- La política de `usuarios` permite `SELECT` y `UPDATE` global bajo esa condición.
- `authenticated` conserva permisos amplios de tabla/columnas, incluidos campos sensibles.
- Las políticas no excluyen explícitamente a un dispositivo `tipo = 'visor'` de esas mutaciones.
- Se confirmó **lectura cross-site** con una identidad ficticia, sin correo administrativo: pudo ver **6 usuarios globales** aunque el sitio usado en la prueba tenía **0 usuarios propios**.
- No se ejecutó una escritura de prueba en producción; el impacto de escritura se deriva de los `GRANT`, las columnas y las condiciones RLS observadas.

En un sistema de control de acceso físico, esto afecta no sólo confidencialidad sino **integridad de decisiones de acceso**.

**Prioridad de remediación: P0 — antes de cualquier ejercicio ofensivo formal.**

---

## 2. Alcance

Se revisaron frontend/repositorio, Supabase vivo, perímetro Cloudflare y cadena de suministro/despliegue.

La revisión de Cloudflare quedó limitada a configuración versionada y arquitectura observable; no se certificaron desde dashboard WAF, Rate Limiting Rules, Bot Management, Access policies, DNSSEC, Logpush, TLS de zona ni alcance de API tokens.

---

## 3. Controles positivos observados

- CSP fuerte: `default-src 'none'`, scripts sólo desde `'self'`, sin `unsafe-inline` ni `unsafe-eval`, `object-src 'none'`, `frame-ancestors 'none'`, `base-uri 'none'`.
- HSTS, `nosniff`, `Referrer-Policy: no-referrer`, `Permissions-Policy` restrictiva y COOP.
- Sourcemaps de producción desactivados.
- Validación cliente + servidor de citas.
- RPC `crear_cita_anfitrion` con límites, idempotencia y atomicidad.
- Aislamiento correcto observado en tablas específicas de visitas para identidad ficticia no anfitriona.
- Edge Functions administrativas vuelven a validar bearer y pertenencia administrativa antes de usar privilegios elevados.
- Timeouts configurados para `anon`, `authenticated` y `authenticator`.
- CI con permisos de lectura, `npm ci --ignore-scripts`, pruebas, build y Playwright E2E.

---

## 4. Hallazgos priorizados

| ID | Severidad | Hallazgo | Estado |
|---|---|---|---|
| A-01 | **P0 / Crítico** | RLS global cross-site en `usuarios`, `contratistas` y `empresas` para JWT autenticado con `sitio_id` | **Confirmado** |
| A-02 | **P1 / Alto** | `GRANT` y privilegios por defecto demasiado amplios en `public` | **Confirmado** |
| A-03 | **P1 / Alto** | El WAF del dominio no protege el endpoint directo `*.supabase.co` | **Arquitectónico** |
| A-04 | **P1 / Alto** | `device-auth` público sin rate limit de aplicación visible y con CORS `*` | **Confirmado en código** |
| A-05 | **P1 / Alto** | Coste innecesario de RLS/índices bajo carga adversarial | **Confirmado por Advisor** |
| A-06 | **P2 / Medio** | Mutaciones sensibles expuestas todavía como DML directo | **Confirmado** |
| A-07 | **P2 / Medio** | Edge Functions con CORS amplio y algunos errores internos propagados | **Confirmado** |
| A-08 | **P2 / Medio** | `Cache-Control: no-store` global desperdicia CDN para assets versionados | **Confirmado** |
| A-09 | **P2 / Medio** | Rulesets/branch protection no pudieron certificarse completamente | **Parcialmente verificado** |
| A-10 | **P3 / Bajo** | `pg_net` en `public` y Leaked Password Protection desactivado | **Advisor** |
| A-11 | **P3 / Bajo** | Parche global de `CSSStyleSheet.prototype.insertRule` por FullCalendar | **Deuda técnica** |

---

## 5. A-01 — autorización cross-site

Definir una matriz explícita:

| Actor | Lectura | Escritura |
|---|---|---|
| `anon` | ninguna | ninguna |
| anfitrión web | sólo sus citas | sólo RPC de sus citas |
| `visor` | sólo sitio asignado y mínimo necesario | **ninguna** |
| `pc` | sólo su sitio | operaciones aprobadas de su sitio |
| `mobile` | sólo su sitio | operaciones aprobadas de su sitio |
| `admin_global` | global | rutas administrativas auditadas |

Para campos críticos (`rol`, `activo`, `sitio_id`, PRAIND, `tipo_ingreso`, etc.), preferir RPC específicas y revocar `UPDATE` directo.

---

## 6. A-02 — privilegios por defecto

Reducir `GRANT` y defaults en staging, concediendo sólo lo indispensable. Considerar un esquema `api` explícitamente expuesto y dejar helpers/tablas internos fuera de la superficie publicada.

---

## 7. A-03 — bypass de Cloudflare por Supabase directo

Si el frontend habla directamente con Supabase, asumir que el atacante conoce la publishable key. La seguridad debe descansar en RLS/GRANT/RPC correctos, no en que el tráfico pase por Cloudflare.

Alternativas válidas:

- mantener Data API directa con RLS impecable, `GRANT` mínimos, RPC y rate limits;
- o mover operaciones sensibles detrás de gateway/Worker/Edge Function propia.

---

## 8. A-04 — `device-auth`

Controles positivos: secreto de alta entropía, SHA-256 almacenado, comparación constante, validación dispositivo/sitio/revocación/suspensión y JWT ES256.

Mitigaciones pendientes:

- rate limit por IP + dispositivo + identidad;
- límite pequeño de body;
- validación estructural estricta;
- restringir CORS cuando aplique;
- TTL menor (5–15 min) o revalidación viva del dispositivo;
- considerar `iss`, `aud`, `jti`/versión;
- errores genéricos al cliente.

---

## 9. A-05 — RLS/índices

Performance Advisor reportó:

- 12 políticas RLS con reevaluación por fila;
- 3 FK sin índice;
- múltiples políticas permisivas redundantes;
- un índice duplicado en `gafetes`.

Mitigar usando `(select auth.uid())` / `(select auth.jwt())` cuando corresponda, indexando FK relevantes y consolidando políticas.

---

## 10. Mutaciones, Edge Functions y caching

- mover cancelación de cita a RPC idempotente y auditada;
- CORS por allowlist;
- errores públicos genéricos + `correlation_id`;
- límites de body y rate limits;
- `index.html` con `no-store`, assets versionados con `public, max-age=31536000, immutable`.

---

## 11. GitHub / supply chain

Antes de producción/benchmark:

- proteger `main`;
- exigir PR y checks;
- revisión obligatoria;
- bloquear force-push/delete;
- Secret Scanning + Push Protection;
- Dependabot;
- CodeQL/SAST;
- análisis de dependencias;
- SBOM de release;
- considerar pinning de Actions por SHA.

---

## 12. Orden de corrección

### 0–24 horas

1. Corregir RLS de `usuarios`, `contratistas`, `empresas`.
2. `visor` estrictamente read-only.
3. Revocar UPDATE directo de privilegios/seguridad.
4. Tests RLS con dos sitios.
5. Verificar cero lectura/escritura cross-site.
6. Rotar cualquier secreto expuesto fuera de gestor seguro.

### 24–72 horas

1. Reducir `GRANT` y defaults.
2. Rate limit `device-auth`.
3. Reducir TTL/revalidar revocación.
4. Corregir warnings RLS initplan.
5. Añadir índices FK.
6. Consolidar políticas duplicadas.
7. Cancelación por RPC.
8. CORS allowlist + errores genéricos.

### Antes del benchmark adversarial

1. Certificar Cloudflare.
2. Branch protection/rulesets.
3. Secret scanning + SAST + dependency scanning.
4. Carga sólo en staging.
5. Tests multi-tenant automatizados.
6. Runbook de incidente/rollback.
7. Métricas y alertas.

---

## 13. Criterio de salida

El sistema estará listo cuando haya 0 P0 abiertos, `visor` sea demostrablemente read-only, el aislamiento A/B esté automatizado, las mutaciones críticas sean site-scoped/admin, el endpoint directo de Supabase conserve las mismas garantías, los `GRANT` sean mínimos y el rate limiting esté probado.

## 14. Conclusión

No conviene gastar primero tiempo en cosmética WAF mientras siga abierto A-01. Orden correcto:

```text
RLS/GRANT correctos
        ↓
mutaciones estrechas y auditadas
        ↓
rate limiting / resiliencia Supabase
        ↓
Cloudflare / WAF / bots
        ↓
CI / supply chain
        ↓
benchmark adversarial
```
