# Tarea aparte: verificación cruzada más rápida

Estado: **pendiente** (anotada 2026-10-31, después de la fase 1 de
observabilidad). Incluye también la limpieza de las 24 advertencias
`security definer` del linter (ver `plan-observabilidad-en-produccion.md`,
"Extensiones y avisos del linter").

## Dónde se va el tiempo hoy

Telemetría del Celular Brisas, últimos 7 días (`llamada_nucleo`, móvil):

| Operación | Llamadas | p50 | Máximo |
|---|---:|---:|---:|
| `preparar_ingreso` (elegir a la persona, local) | 420 | 4 ms | 35 ms |
| `registrar_ingreso` (confirmar; incluye `verificar_entrada` en la nube) | 374 | **276 ms** | **2.009 ms** |
| `registrar_ingreso_proveedor` | 42 | 274 ms | 2.084 ms |
| `registrar_ingreso_correo` | 17 | 314 ms | 484 ms |
| `cerrar_ingreso_remoto` | 24 | 281 ms | 1.283 ms |

Dentro de Postgres la consulta "¿está adentro en alguna unidad?" tarda
**3,4 ms** (`explain analyze` en producción, 30 personas adentro). El resto es
red: Costa Rica → `us-east-1` (Virginia) en 4G. No hay región de Supabase más
cercana.

Descartado: hacerlo en una Edge Function. Agrega un salto (equipo → función →
base), arranque en frío (~470-580 ms medidos), necesitaría `service_role` y
duplicaría las reglas en TypeScript.

## Qué probar, en orden

1. **Verificar por adelantado.** La cédula se conoce al elegir a la persona
   (`preparar_ingreso`), segundos antes de confirmar. Lanzar ahí, en segundo
   plano, la parte de la persona de `verificar_entrada` (misma vía, otra vía),
   y la del gafete apenas se elija el número. Al confirmar se usa la respuesta
   si tiene menos de N segundos (propuesta: 10 s); si no, se pregunta como hoy.
   La nube sigue siendo la última palabra: sus índices únicos rechazan un
   doble ingreso al subirlo (409, conflicto), así que la ventana de N segundos
   no abre un hueco nuevo. Meta: confirmar en lo que tarda lo local (~5 ms)
   en el caso normal.
2. **Los picos de 1-2 s.** Cruzar con la telemetría si coinciden con conexión
   fría, renovación del token o radio dormida; el plan de red del móvil
   (`plan-red-movil-por-el-nucleo.md`) ya reduce los despertares, y el token
   se puede renovar antes de vencer en segundo plano.
3. **La consulta en la base.** `private.personas_adentro()` recorre todos los
   ingresos abiertos de tres tablas y filtra. Con índices parciales por cédula
   normalizada (`where hora_salida is null`) sería una búsqueda por índice
   (<1 ms). Prioridad baja hoy (3 ms contra ~270 ms de red); importa si crece
   el volumen.
4. **Las 24 advertencias `security definer`:** envoltorio `security invoker`
   en `public` que llama a la función real en `private`, sin cambiar nombres
   ni parámetros (las apps y el panel no se tocan), con batería que compruebe
   que cada guardia sigue rechazando a quien no corresponde.

## Cómo se mide

`llamada_nucleo` de `registrar_ingreso*` (p50, p90, máximo) antes y después,
por versión, con el resumen diario de la fase 1.
