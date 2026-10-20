# Notas del proyecto

## Preferencias del usuario

- **NUNCA usar el término prohibido para el puesto de control** en conversación, código,
  comentarios ni documentación. Para referirse al puesto de control de acceso usar
  términos como "puesto de control", "portería" o "punto de acceso".
- Comunicarse siempre en español.
- Siempre que se complete un cambio exitoso, hacer un commit bien documentado y subirlo.
- **Todos los commits van sólo a nombre de `Daniel Quintana <daniel.bleach1@gmail.com>`**,
  como autor y como committer (`git config user.name`/`user.email` antes del primer
  commit). Sin líneas `Co-Authored-By`, `Claude-Session` ni ninguna otra atribución a
  asistentes en commits ni en pull requests.

## Supabase

- **Permisos explícitos** (desde `20261020130000_permisos_api_explicitos`): lo que crea
  una migración en `public` ya NO recibe permisos solo, y ninguna función nueva (de
  ningún esquema) es ejecutable por `public`. Cada migración da a mano, junto con su RLS:
  - tablas: `grant ... on public.x to authenticated` (lo que usen apps/panel) **y**
    `to service_role` (Edge Functions de administración; salta RLS pero no los GRANT);
  - funciones RPC: `grant execute ... to authenticated, service_role`; las de `private`
    que use la RLS, `to authenticated`;
  - Realtime necesita `select` para `authenticated` en la tabla publicada.
  - **Nunca** a `anon`: nadie entra sin sesión (los equipos usan su JWT con rol
    `authenticated`). Única excepción: la telemetría de staging (`supabase/scripts/`).
- Cada migración lleva su batería en `supabase/tests/`, probada en staging dentro de una
  transacción con `rollback`. Mergear a `main` aplica las migraciones en producción.
