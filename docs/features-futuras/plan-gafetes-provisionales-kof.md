# Módulo "Gafetes provisionales KOF"

> Plan aprobado, sin ejecutar todavía — mismo criterio que otros documentos
> de esta carpeta (`plan-control-proveedores.md`,
> `plan-control-rutas.md`).

## Contexto

Problema distinto y mucho más chico que proveedores: un colaborador
**interno** de KOF (no contratista) a veces olvida o pierde su gafete
permanente, pero por norma no puede circular sin identificación. Como es
personal interno, no le corresponde un gafete de contratista — existe una
categoría física aparte, "gafete provisional KOF", con su propia
numeración, que se le presta mientras está en el sitio y se le retira al
salir.

Flujo (dictado por el usuario, textual): la persona dice "se me olvidó el
gafete", el guardia le pide su cédula física (**sólo para cotejar el
nombre a simple vista, el sistema no guarda ni valida ningún número de
cédula** — el cliente no autorizó capturar ese dato) y su número de
empleado. El guardia busca por nombre en el sistema, aparece la persona, y
verifica que el número de empleado que muestra el sistema coincide con el
que la persona dijo de palabra. Se le asigna un número de gafete
provisional y se entrega. Al salir, se retira — sin ninguna otra
validación.

## Hallazgo clave: el catálogo de personal ya existe, se reutiliza tal cual

El usuario fue explícito: "ya tenemos un catálogo de nombres y números de
empleado de los KOF, reutilicemos esa tabla" — es **`encargados_ruta`**,
la tabla ya creada e importada (~1000+ filas) para el módulo de rutas
(`src/database/schema.rs:2565-2572`):

```sql
CREATE TABLE encargados_ruta (
    id INTEGER PRIMARY KEY,
    codigo_empleado TEXT NOT NULL UNIQUE,
    nombre TEXT NOT NULL,
    cedula TEXT,
    activo INTEGER NOT NULL DEFAULT 1 CHECK (activo IN (0, 1)),
    uuid TEXT NOT NULL
) STRICT;
```

Ya existe además la búsqueda por nombre o código lista para reusar sin
tocarla: `Nucleo::buscar_encargados_ruta(texto: String) ->
Vec<EncargadoRuta>` (`mobile/rust-core/src/lib.rs:1346`), la misma que usa
hoy `PasoEncargado` en `PantallaRutas.kt` para el módulo de rutas. Este
módulo nuevo es, en buena parte, **una segunda pantalla de consumo sobre
un catálogo que ya existe** — no hace falta ni una tabla de personas
nueva, ni un importador, ni una pantalla de administración del catálogo
(esa ya es responsabilidad del módulo de rutas).

Nota: la columna `cedula` de `encargados_ruta` es nullable y no
confiable/poblada para todos — coincide con lo que dijo el usuario ("el
sistema no tiene número de cédula" para este propósito). La verificación
de identidad en este flujo es 100% humana (el guardia mirando la cédula
física), el sistema nunca la captura ni la valida.

## Diseño del núcleo

### Esquema

1. **`gafetes`**: agregar `'PROVISIONAL_KOF'` al `CHECK` de `tipo` (cuarto
   valor, junto a `CONTRATISTA`/`VISITA`/`PROVEEDOR`) y una columna
   `encargado_portador_id INTEGER REFERENCES encargados_ruta(id) ON DELETE
   RESTRICT`. Extender el `CHECK` par-exclusivo a cuatro casos (mismo
   criterio que el agregado para `PROVEEDOR` en `plan-control-proveedores.md`
   — conviene hacer ambas extensiones de `gafetes` en la misma migración ya
   que tocan el mismo `CHECK`).
2. **`gafetes_incidentes`**: agregar `encargado_portador_id` con el mismo
   criterio que las demás columnas de portador, por consistencia (gafete
   provisional perdido/resuelto).
3. **`prestamos_gafete_provisional`** — tabla nueva, ciclo entrega/
   devolución, mismo espíritu que `movimientos_visita`/
   `registro_ingresos_proveedor`:
   - `encargado_id INTEGER NOT NULL REFERENCES encargados_ruta(id) ON
     DELETE RESTRICT` + snapshot `encargado_nombre TEXT NOT NULL`,
     `encargado_codigo_empleado TEXT NOT NULL` (el dato que el guardia
     corrobora verbalmente).
   - `gafete_numero INTEGER NOT NULL`.
   - `fecha_hora_entrega TEXT NOT NULL`, `usuario_entrega_id INTEGER NOT
     NULL REFERENCES usuarios(id)`, `usuario_entrega_nombre TEXT NOT
     NULL`.
   - Trío nullable todo-o-nada para la devolución:
     `fecha_hora_devolucion`, `usuario_devolucion_id REFERENCES
     usuarios(id)`, `usuario_devolucion_nombre` — mismo `CHECK` que ya usan
     `registro_ingresos`/`salidas_ruta`/`registro_ingresos_proveedor`.
   - `CHECK` cronológico, `uuid TEXT` único para sync.
   - Índice único parcial `WHERE fecha_hora_devolucion IS NULL` sobre
     `encargado_id` — no puede tener dos préstamos abiertos a la vez.
   - Triggers de inmutabilidad, mismo patrón que el resto (no se borra,
     entrega inmutable, devolución una sola vez, fechas UTC).
   - Sin ningún campo de "resultado"/validación — a propósito, el usuario
     fue explícito en que no hay más verificación que la humana.
4. Si sincroniza a la nube, sumar `'prestamo_gafete_provisional'` al
   `CHECK` de `cola_salida.entidad`.

### Dominio y servicios

- `PortadorGafete::ProvisionalKof(i64)` (id de `encargados_ruta` — a
  diferencia de `Proveedor(i64)` de `plan-control-proveedores.md`, acá sí
  es un id de catálogo real, porque la persona sí es reutilizable).
- Nuevo `gafete_provisional_service.rs`: `entregar_gafete_provisional`,
  `registrar_devolucion_gafete_provisional`, `listar_prestamos_activos` —
  mucho más simple que `ingreso_proveedor_service.rs`, sin reglas de
  negocio más allá de "gafete no ocupado en el sitio" (reusa la misma
  validación que ya existe para contratistas/proveedores).

### Capa UniFFI

Sumar a `Nucleo`: `entregar_gafete_provisional`,
`registrar_devolucion_gafete_provisional`,
`listar_gafetes_provisionales_activos`. **No hace falta una función de
búsqueda nueva** — reusa `buscar_encargados_ruta` tal cual.

## UI mobile

Mucho más liviano que proveedores — no es un wizard de pasos, es un
formulario simple (screen o diálogo): buscador de encargado reusando el
mismo `ExposedDropdownMenuBox` que ya usa `PasoEncargado` en
`PantallaRutas.kt`, mostrando de forma prominente el `codigo_empleado` del
resultado elegido (para que el guardia lo coteje contra lo que la persona
dijo de palabra), campo de número de gafete, botón "Entregar". **Sin
cámara/OCR** — no hay ningún documento que escanear en este flujo.

Cierre (devolución): se integra como una cuarta categoría en la lista
unificada de "Activos" (junto a contratistas, visitas y proveedores) —
tap + confirmación + `registrarDevolucionGafeteProvisional`, mismo patrón
que las demás.

## UI desktop

Formulario simple equivalente (buscar encargado, número de gafete,
entregar) + paridad en "Activos" + historial. **Sin pantalla de
administración de catálogo** — `encargados_ruta` ya se administra desde
el módulo de rutas, este módulo sólo lo consulta.

## Verificación

- `cargo test`: el `CHECK` par-exclusivo de `gafetes` con las cuatro
  variantes de portador (contratista/visita/proveedor/provisional-kof) es
  el punto más propenso a errores — cubrir explícitamente los cuatro
  casos y sus combinaciones inválidas.
- `cargo test` en `mobile/rust-core` para los métodos nuevos.
- Build de Android vía `workflow_dispatch`.
- Prueba manual: buscar un encargado real de los ~1000 ya importados,
  entregar un gafete provisional, confirmar que no deja entregar dos a la
  vez a la misma persona, devolución desde Activos, y que gafetes de
  contratista/visita/proveedor sigan sin verse afectados.
