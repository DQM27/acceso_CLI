# Telemetría de diagnóstico del escritorio

Es la contraparte de la del teléfono (`mobile/android/docs/telemetria-diagnostico.md`).
Usa la misma tabla `telemetria_diagnostico` del proyecto de **staging** y los
mismos nombres de evento y de campo, así una consulta sirve para las dos
plataformas. Las filas del escritorio se reconocen por `version_app`, que
empieza con `desktop-` (por ejemplo `desktop-1.8.15+abc1234`, con el hash corto del commit).

## Cuándo mide

Un solo código para **todos** los builds, sin flags ni variantes. Cada evento
pertenece a un grupo (`control_acceso::telemetria::grupo_de_evento`) y sólo se
mide si la orden vigente de este equipo lo permite:

- `base` (arranque, cierre, caídas, errores) va **siempre**;
- `rendimiento` va **por defecto**;
- `fluidez`, `sincronizacion`, `recursos`, `escaneo` y `depuracion` los enciende
  el administrador **por equipo** desde el panel, con vencimiento.

La orden viaja panel → nube (`telemetria_ordenes`) → equipo. El equipo la recibe
por el aviso en vivo `telemetria_config` o en la sincronización, la guarda en su
SQLite (`config_telemetria`) y el núcleo la tiene en memoria; al abrir la app se
carga la guardada, así que rige aunque no haya red. Al vencer, el equipo vuelve
a `base` y `rendimiento` sin consultar a nadie. Diseño completo en
`docs/features-futuras/plan-telemetria-y-registro-de-errores.md`.

- El frontend pregunta al backend qué grupos rigen (`telemetria_grupos`, una
  llamada local, sin red) al abrir y cada 30 s, y sólo arma lo que rige. El
  backend vuelve a filtrar por grupo y descarta los tipos que no están en el
  catálogo.
- Sólo se manda **telemetría** a staging, nunca datos de operación. La base y la
  nube de la app siguen siendo las de producción. La dirección de staging está
  fija en `src-tauri/src/telemetria.rs`; producción nunca recibe telemetría. La
  llave es la publicable y en esa tabla sólo puede insertar (ver
  `supabase/scripts/telemetria_diagnostico_staging.sql`).
- Ya no hay build de diagnóstico, ni la feature `telemetria`, ni
  `telemetria_reglas` / `reglas_telemetria_vigentes`.

## Privacidad

Solo números y nombres técnicos:
- nombres de comandos, pantallas y secciones;
- tipos de error (`TypeError`, nunca el mensaje);
- tiempos, memoria y CPU.

Nunca se envían cédulas, nombres, placas, argumentos de comandos, contenido de
formularios ni mensajes de error. El backend también filtra lo que manda el
frontend: el `tipo` solo puede tener minúsculas, dígitos y `_`, y cada evento
pesa como máximo 16 KB.

## Eventos (`tipo`)

> **Hora de los eventos** (`ocurrido_en`): sale del reloj confiable del núcleo
> (hora del servidor + contador de arranque), la misma con que se sellan los
> registros, no del reloj del Windows. Así los tiempos de todos los equipos se
> comparan entre sí y con la bitácora. Antes de abrir el núcleo (los primeros
> eventos del arranque) se usa la del sistema.

| Tipo | Origen | Qué dice |
|---|---|---|
| `sesion_inicio` | Rust | SO, arquitectura, núcleos, RAM total |
| `orden_telemetria` | Rust, al arrancar y cada vez que cambia la orden | Confirmación de que la orden del panel llegó: `version` (`0` = sin orden) y `grupos` vigentes. Grupo `base` |
| `salida_anterior` | Rust | Cómo terminó la vez anterior: `CIERRE_NORMAL` o `SIN_CIERRE_NORMAL` (cuelgue, crash, "Finalizar tarea", apagón) |
| `arranque` | Rust | `ms_hasta_ventana` desde que arrancó el proceso; `por`: `frontend` o `red_de_seguridad` (el splash se cerró por tiempo) |
| `muestra_sistema` | Rust, cada 2 min | Memoria del proceso (en uso, privada y pico), CPU (`cpu_un_nucleo_pct`, `cpu_dispositivo_pct`), RAM libre del equipo y tamaño de la cola. No incluye los procesos de WebView2 |
| `sincronizacion_auto` | Rust | Cada pulso automático: `ms`, si fue el de `arranque` y el `resultado` |
| `reloj` | Rust, tras cada pulso automático | Reloj confiable (hora del servidor + contador de arranque): `confiable`, `margen_ms`, `diferencia_equipo_ms` (reloj de Windows menos la hora que usa la app; un salto grande sin conexión = alguien movió la hora) y `ancla_hace_ms` |
| `sesion_unidad` | Rust, en cada sincronización con sesión | Qué respondió la nube a la sesión única (`vigente`, `desplazada`, `sin_usuario` o `error`) y `transcurrido_ms` desde el ingreso. Sin cédula ni nombre |
| `panic` | Rust | Dónde ocurrió (`archivo:línea`) y en qué hilo, sin el mensaje. Se guarda en disco y sale en el próximo arranque |
| `cierre` | Rust | Cierre normal: minutos que estuvo abierta |
| `llamada_nucleo` | Frontend, cada 30 s | Por comando de Tauri, con el IPC incluido: llamadas, errores, p50/p90/máx y total en ms. Si hubo errores, `errores_por_tipo` (lista fija, ver `docs/telemetria-contrato.md`) |
| `transicion_ingreso` | Rust, en cada `preparar_ingreso` | Lo que tarda elegir a un contratista hasta tener el formulario del gafete, del lado del backend (sin el IPC, que sale en `llamada_nucleo`): `resultado` (`formulario`, `bloqueada` o `error`), `ms_cola` (esperando un hilo del pool), `ms_espera_candado` (esperando `state.core()`), `ms_nucleo` (`ms_total` sin la espera del candado: reglas locales y consulta a la nube) y `ms_total`. Mismos nombres que el del teléfono |
| `realtime` | Frontend, cada 30 s | Canal de avisos en vivo, con los mismos campos que el teléfono: conexión, avisos por tabla, ecos propios, aplicados y latencia corregida por el desfase del reloj |
| `pantalla` | Frontend | Pantalla o sección mostrada, la anterior y `ms_hasta_primer_frame` |
| `ui_bloqueos` | Frontend, cada 30 s | Tareas largas (> 50 ms) que trabaron la ventana: cantidad, total, p50, máximo y cuántas pasaron de 200 ms |
| `error_js` | Frontend, cada 30 s | Errores de JavaScript por origen y tipo (`error:TypeError`, `promesa:...`, `render:...`) |
| `muestra_webview` | Frontend, cada 2 min | Memoria del motor de JavaScript y cantidad de nodos del DOM, con la pantalla actual |
| `telemetria_descartada` | Rust | Filas que se descartaron porque la cola llegó a su tope (5 000) |

## Envío

- Cola en memoria. Un hilo propio la envía en lotes de 200 cada minuto.
- Si un lote no se puede enviar, la cola se guarda en
  `telemetria_pendiente.jsonl`, junto a la base local, y se reintenta al
  siguiente arranque.
- Al cerrar la app se guarda la cola y se intenta enviar durante 3 s como
  máximo.
- En un panic solo se guarda en disco. El cliente HTTP bloqueante no puede
  usarse desde un hilo de tokio.
- `telemetria_dispositivo` guarda un UUID aleatorio por instalación. No es
  el identificador de la máquina ni el del dispositivo en la nube.

## Consultas útiles

Comandos más lentos del escritorio:

```sql
select datos->>'nombre', sum((datos->>'llamadas')::int) llamadas,
       max((datos->>'max_ms')::numeric) max_ms, sum((datos->>'errores')::int) errores
from public.telemetria_diagnostico
where tipo = 'llamada_nucleo' and version_app like 'desktop-%'
group by 1 order by 3 desc;
```

Avisos en vivo, teléfono contra PC, en la misma prueba:

```sql
select ocurrido_en, left(version_app, 7) plataforma, datos->'avisos_por_tabla',
       datos->>'latencia_ms_p50', datos->>'latencia_ms_max', datos->>'desfase_reloj_ms'
from public.telemetria_diagnostico
where tipo = 'realtime' order by ocurrido_en desc;
```

Cierres anormales y panics:

```sql
select ocurrido_en, tipo, datos from public.telemetria_diagnostico
where version_app like 'desktop-%' and tipo in ('salida_anterior', 'panic')
order by ocurrido_en desc;
```
