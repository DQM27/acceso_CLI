# Auditoría de Realtime en staging — 28 de septiembre de 2026

Rama auditada: `claude/nucleo-n1-n3` (contiene `realtime-con-datos`, último
commit `b949494d`). Entorno: proyecto Supabase `control-acceso-staging`
(`pmrytjktlyiuikxuuxpr`). Todo lo del lado de Supabase se hizo con consultas
de solo lectura; no se modificó ningún dato ni esquema.

**Veredicto:** el camino de ingresos de contratistas ya está al máximo
posible con Supabase Realtime: el aviso llega con la fila y el equipo la
guarda sin volver a consultar la nube. El resto de las tablas sincroniza
solo su parte. Queda un hueco funcional (citas canceladas) y la migración
todavía no está en producción.

## 1. Qué se verificó y resultado

| Punto | Resultado |
|---|---|
| `private.emitir_cambio_nube_sitio()` en staging igual a la migración `20260929075806_cambio_nube_lleva_la_fila_de_ingresos.sql` | Idéntica (registrada como `20260927211032` en staging) |
| La misma función en `control-acceso-nube` | **Versión vieja**, sin `id` ni `registro` |
| Triggers `*_emitir_cambio_nube` en staging | 17, todos habilitados (los 14 de producción más `viajes_ruta`, `documentos_ruta` y `salida_ruta_documentos`) |
| Políticas de `realtime.messages` | Las 3 esperadas (broadcast leer, presencia leer, presencia publicar), todas por `sitio:` + `sitio_id` del JWT |
| Tráfico real de avisos `ingresos` desde la migración (27/09 21:10 → 23:28) | 20 de 20 con la fila completa (10 INSERT, 10 UPDATE) |
| Tamaño del aviso con fila | ~1,1 KB (máx. 1.179 B); antes ~270 B. Sin impacto práctico |
| Avisos duplicados (misma tabla + id en el mismo segundo) | 0 de 101 |
| Avisos sin `dispositivo_id` | 18, todos pares INSERT/DELETE de pruebas hechas directo en la base (sin JWT de dispositivo). Los equipos los procesan igual, que es lo correcto |
| Restos de laboratorio (`_lab_*`) | Solo quedan mensajes viejos en `realtime.messages`; no hay tablas, funciones ni triggers de laboratorio |

### Coincidencia de la fila con el núcleo

Claves que trae `registro` en los avisos reales: `contratista_cedula`,
`contratista_id`, `contratista_nombre`, `created_at`, `dispositivo_entrada_id`,
`dispositivo_salida_id`, `empresa_activa_snapshot`, `empresa_nombre`,
`gafete_numero`, `hora_entrada`, `hora_salida`, `id`, `medio_ingreso`,
`motivo_resultado`, `placa`, `reglas_version`, `resultado_acceso`, `sitio_id`,
`tipo_ingreso`, `updated_at`, `usuario_entrada_nombre`,
`usuario_salida_nombre`, `version`.

Cubren todos los campos de `FilaIngresoRemoto`
(`src/nube/sincronizacion/ingresos.rs`) y de `EstadoIngreso`
(`src/nube/en_vivo.rs`). Los campos extra se ignoran.

## 2. Revisión del código de la rama

- **Alcance por tabla** (`src/nube/alcance.rs`): cada aviso corre solo las
  etapas de su tabla; una tabla desconocida cae en sincronización completa.
  Correcto.
- **Aplicación en vivo** (`src/nube/en_vivo.rs`): usa las mismas funciones
  que la sincronización. `guardar_ingreso_remoto` descarta lo que ya vive en
  `registro_ingresos`, así que un aviso de un ingreso propio modificado desde
  otro lado no lo duplica en Activos. Un aviso repetido no duplica
  (`INSERT OR REPLACE`). Correcto.
- **Eco propio**: escritorio (`desktop/src/nubeRealtime.ts:177`) y Android
  (`NubeRealtime.kt:113`) descartan sus propios avisos.
- **Red de seguridad**: la sincronización por tabla corre igual detrás del
  aviso aplicado, y el pulso de 2 minutos sigue activo.

## 3. Hallazgos

### R-1 (alta) — Cancelar una cita no avisa en vivo

`web-visitas/src/api.ts:111` cancela con `update citas set estado =
'CANCELADA'`. Ni `citas` ni `cita_visitantes` tienen trigger de aviso, en
staging ni en producción: solo `cita_sitios`. El núcleo ya reconoce la tabla
`citas` (`alcance.rs`), pero nunca le llega el aviso. La portería puede ver
una visita cancelada como válida hasta 2 minutos (pulso periódico).

Arreglo propuesto: trigger en `citas` que emita `cambio_nube` a cada
`sitio:` de sus filas en `cita_sitios` (`citas` no tiene `sitio_id` propio,
así que no puede reutilizar `emitir_cambio_nube_sitio` tal cual).

### R-2 (alta, operativa) — La migración del aviso con datos no está en producción

Sin riesgo de rotura: la app nueva con la función vieja cae en la
sincronización por tabla. Pero la mejora principal no rinde en la portería
hasta aplicar `20260929075806_cambio_nube_lleva_la_fila_de_ingresos.sql` en
`control-acceso-nube`.

### R-3 (media) — Solo `ingresos` trae la fila

`ingresos_proveedor`, `movimientos_visita`, `prestamos_gafete_provisional` y
`gafetes` siguen necesitando una consulta a la nube por aviso (ya acotada a
su tabla). Siguiente mejora natural: ampliar `registro` y `en_vivo.rs` a
`ingresos_proveedor` y `movimientos_visita`, que son las otras pantallas de
"quién está adentro".

### R-4 (media) — Staging tiene esquema que no está en esta rama

Staging se armó por lotes (`lote_00` … `lote_04`), así que su historial de
migraciones no coincide archivo por archivo con `supabase/migrations`.
Además tiene objetos sin migración en esta rama:

- `visitas`, `visita_invitados`, `visita_movimientos` (publicadas para el
  panel) y sus RPC: vienen de `claude/rediseno-web-visitas`.
- `viajes_ruta`, `documentos_ruta`, `salida_ruta_documentos`, con trigger de
  aviso: vienen de `feat/rutas-documento-tramo-viaje`.
- `personas_vetadas` (creada y revertida el 28/09): de
  `claude/veto-por-persona`.

No afecta el tiempo real de esta rama, pero una prueba en staging no
representa exactamente lo que tendría producción con estas migraciones.

### R-5 (baja) — Documentación desactualizada

`docs/arquitectura/arquitectura-supabase.md` §4.2 y §4.4 hablan de 6
triggers; hoy son 14 en producción y 17 en staging, y el payload ya lleva
`id` y `registro`.

## 4. Pendiente de esta auditoría

- Registros del servicio Realtime (errores de conexión o autorización) y
  asesor de seguridad de staging: no se pudieron consultar en esta sesión.
- Latencia de punta a punta con dos equipos reales del mismo sitio
  (registrar en uno y medir cuánto tarda en aparecer en el otro). La base
  solo prueba que el aviso sale bien formado, no cuánto tarda en llegar.
