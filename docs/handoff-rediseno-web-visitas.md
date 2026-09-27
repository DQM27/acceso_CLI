# Handoff: rediseño de la web de anfitriones (sesión dedicada)

Instrucciones para **una sesión que trabaja sólo en esto**, en paralelo
con otra sesión que trabaja en el núcleo Rust. Leer completo antes de
tocar nada.

## Documentos de referencia (obligatorio leer)

1. `AGENTS.md`: español; nunca el término prohibido para el puesto de
   control ("puesto de control", "portería" o "punto de acceso"); commit
   documentado + push por cada cambio exitoso.
2. `docs/auditorias/rediseno-visitas-2026-09-27.md`: el diseño aprobado.
   Esta sesión implementa **3.1 (sólo la parte de Supabase), 3.2, 3.8 y
   las fases V2 y V4**. Las decisiones de la tabla "Decisiones tomadas"
   están cerradas: no reabrirlas.

> Estos documentos viven en la rama `claude/auditoria-movil-arquitectura`.
> Si todavía no está fusionada a `main`, traerlos con
> `git checkout origin/claude/auditoria-movil-arquitectura -- docs/`.

## Rama

Crear `claude/rediseno-web-visitas` desde `main`. Nada de esta sesión va
a otra rama.

## Límites con la otra sesión (para no pisarse)

| Esta sesión **sí** toca | Esta sesión **no** toca |
|---|---|
| `web-visitas/**` | `src/**` (núcleo Rust) |
| `supabase/migrations/**` (sólo migraciones **nuevas** de visitas) | `mobile/**`, `desktop/**` |
| `supabase/functions/**` (sólo funciones **nuevas** de visitas) | `web/**` (panel administrativo) |
| `docs/` de visitas | Migraciones o funciones existentes |

La otra sesión trabaja en el núcleo (N1-N3 de
`docs/auditorias/auditoria-nucleo-rust-2026-09-27.md`), que **no** cambia
el esquema de visitas. Si algo de esta sesión obligara a tocar `src/`,
`desktop/` o `mobile/`, **parar y avisar al dueño**: no se hace aquí.

## Esquema: libre, pero sin borrar lo viejo todavía

Visitas **no está en uso en producción** (es un feature en desarrollo):
no hay datos reales que migrar ni usuarios que proteger. Por eso **no
hace falta puente ni migración de datos**: el modelo nuevo se diseña
limpio, tal cual la sección 3.1 del diseño.

Única restricción: **no borrar ni alterar** las tablas viejas (`citas`,
`cita_sitios`, `cita_visitantes`, `movimientos_visita`, `anfitriones`)
ni la RPC `crear_cita_anfitrion`. El núcleo Rust las consulta en **cada
sincronización** (`recibir_citas_del_sitio`,
`recibir_historial_visitas_del_sitio`); si desaparecen, la
sincronización de escritorio y móvil falla completa en staging y bloquea
a la otra sesión. Se eliminan más adelante, cuando se haga V1/V3 (núcleo
y puesto de control de visitas).

Consecuencia: hasta V1/V3 la web nueva **no se conecta con el check-in
del escritorio**. Es aceptable: el estado en vivo ("Llegó 9:12") se
construye sobre las tablas nuevas (`visita_movimientos`) y se prueba
insertando movimientos de prueba en staging.

Nombres de tablas nuevas sin chocar con las viejas: `visitantes`,
`visitas`, `visita_invitados`, `visita_movimientos`, `anfitrion_sitios`,
`restricciones_visitante`, `aceptaciones`, `requisitos_sitio`; para
anfitriones, columnas **nuevas** en `anfitriones` (p. ej. `id`,
`activo`) sin romper las existentes, o `anfitriones_v2`.

## Qué construir

### V2 (Supabase, sólo staging)

- Migraciones de las tablas nuevas con RLS por igualdad:
  anfitrión sólo ve/escribe lo suyo (`anfitrion` = `auth.email()`),
  dispositivos del sitio leen visitas de su `sitio_id`,
  `es_admin_global()` ve todo.
- RPC atómicas e idempotentes (id generado por el cliente, como
  `crear_cita_anfitrion` hoy): `crear_visitas`, `editar_visita`
  (sólo si ningún invitado entró), `cancelar_visita`,
  `duplicar_visita`, `responder_solicitud` (aprobar/rechazar walk-in).
- Vista `mis_visitas` (con `security_invoker = true`) con estado por
  persona.
- Función para "visitantes anteriores del anfitrión" (búsqueda por
  nombre/documento, sólo entre los que ese anfitrión ya invitó).
- Edge Function de correo: aviso de llegada y de solicitud sin cita, con
  enlace firmado de un solo uso para aprobar/rechazar. **[Si falta el
  proveedor de correo o su clave, dejar la función lista y avisar al
  dueño; no inventar credenciales.]**
- Pruebas de RLS: una cuenta que no es anfitrión ve cero filas; un
  anfitrión no ve visitas de otro.

### V4 (web)

Implementar exactamente la sección **3.8** del diseño:

- Tres pantallas: **Mis visitas** (inicio), **Agendar visita** (una sola
  página), **Detalle de visita**.
- Quitar Bootstrap y Sass; usar los tokens de `design/brisas.json` con
  acento rojo propio. Controles nativos (fecha, hora, casillas).
- Tono *usted*, sin voseo ni jerga técnica.
- Mobile-first, tarjetas en lugar de tablas, estados con texto + ícono.
- Actualización en vivo de estados con Supabase Realtime (sólo invalida
  y vuelve a pedir la vista, no recarga todo).
- Mantener: login actual (Google / enlace mágico) y la CSP estricta de
  `web-visitas/public/_headers`.

## Verificación antes de cada push

```bash
cd web-visitas
npm ci
npm run lint
npm run build
npm test
npm run test:e2e   # Chromium preinstalado; no ejecutar "playwright install"
```

Además:
- Migraciones aplicadas y probadas **sólo en staging**
  (`control-acceso-staging`). **Nunca** producción
  (`control-acceso-nube`): esa la aplica el dueño.
- Probar en staging que la llegada de un invitado (movimiento de prueba
  insertado en `visita_movimientos`) se ve en la web en vivo.
- Comprobar que la sincronización de escritorio contra staging sigue
  funcionando (las tablas viejas siguen intactas).
- Limpiar en staging todo dato de prueba creado.
- Criterios de aceptación de 3.8 (agendar 2 personas para mañana en
  menos de 60 s sin ayuda).

## Al terminar

- No crear PR salvo que el dueño lo pida.
- Dejar en `docs/` un resumen corto: qué quedó, qué migraciones hay que
  aplicar en producción y en qué orden, y qué queda pendiente para V1/V3
  (conectar el puesto de control a las tablas nuevas y borrar las viejas).
