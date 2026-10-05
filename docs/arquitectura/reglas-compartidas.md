# Reglas compartidas: una sola fuente para núcleo, panel web y Edge Functions

Fecha: 2026-10-04. Rama: `feat/reglas-compartidas`.

## Por qué

Hasta ahora, las reglas de negocio vivían en el núcleo Rust, que usan
escritorio y teléfono. El panel web no usa el núcleo: tenía sus propias
copias en TypeScript y en SQL (`panel_crear_contratista`), y esas copias se
desviaron:

- la función SQL seguía aceptando contratistas "POR CORREO", retirado del
  núcleo el 2026-10-03; el formulario lo ocultaba, pero la función se podía
  llamar directo;
- no tenía la regla de personal de ruta;
- sus mensajes eran distintos a los de las apps.

Es el problema clásico de tener la misma regla en varios lugares: se cambia
en uno y en los otros no.

## Dos tipos de reglas

| Tipo | Ejemplos | Dónde vive |
|---|---|---|
| **De criterio** (¿el dato es válido?) | formato de cédula, nombre sin números, tipo de ingreso elegible, cuándo se pide PRAIND, PRAIND vencido, personal de ruta | crate `reglas/`, compartido por todos |
| **De autoridad** (¿quién puede hacerlo?) | sólo un administrador crea contratistas, sólo un equipo vigente escribe, RLS | en el servidor: Postgres (RLS) y Edge Functions |
| **Que necesitan datos** | cédula repetida, la empresa existe, "¿ya está adentro?", gafete libre | donde están los datos: el núcleo con su base local, la nube con Postgres (índices únicos, triggers, funciones) |

Lo que corre en el navegador se puede saltar. Por eso el panel usa las
reglas sólo para **avisar** mientras se llena el formulario, y la decisión de
guardar la toma el **servidor**, que valida otra vez con el mismo código.

## Cómo está armado

```
reglas/                      crate `control_acceso_reglas` (Rust puro: sin base,
  src/cedula.rs              sin red, sin reloj; sólo chrono y thiserror)
  src/tipo_ingreso.rs
  src/contratista.rs         validar_contratista + reglas sueltas
  wasm/                      crate `control_acceso_reglas_wasm` (wasm-bindgen)
    build.rs                 huella de las fuentes
    src/lib.rs               API para JavaScript

núcleo (src/)                reexporta desde `reglas` (domain::cedula,
                             models::tipo_ingreso, domain::contratista);
                             ContratistaService usa validar_contratista

web/src/reglas/              panel: carga el paquete (index.ts) + test de huella
  wasm/                      GENERADO: reglas.js, reglas_bg.wasm, tipos

supabase/functions/
  _shared/reglas.ts          Deno: carga el paquete
  _shared/reglas/            GENERADO: reglas.js + .wasm en base64
  admin-crear-contratista/   alta de contratistas del panel
```

### El alta de un contratista, de punta a punta

| Plataforma | Valida con | Guarda en |
|---|---|---|
| Escritorio y teléfono | `validar_contratista` (Rust nativo) en `ContratistaService` | SQLite, y se sincroniza |
| Panel web: formulario | `validarContratista` (WebAssembly en el navegador) | No guarda; sólo avisa con el mensaje del núcleo |
| Panel web: botón Guardar | Edge Function `admin-crear-contratista`: revisa que sea administrador y valida con `validarContratista` (WebAssembly en Deno) | Postgres, con la cuenta de servicio |

La base conserva sólo lo que necesita ver todos los datos:
- el índice único `contratistas_cedula_normalizada_key` (cédula repetida,
  también cuando dos altas llegan a la vez);
- la RLS (un administrador no puede insertar directo);
- el trigger que avisa a los equipos.

### Velocidad

- **El panel no espera al WebAssembly para mostrarse.** Empieza a cargarlo al
  arrancar (64 KB comprimido) y sólo el formulario lo necesita. Si alguien lo
  abre antes de que llegue, muestra "Cargando…" un instante
  (`useEstadoReglas`).
- **Guardar hace 3 viajes seguidos a la base, no 4.** La función revisa la
  sesión y `administradores_panel` y, a la vez, busca la empresa; después
  guarda. Si quien llama no es administrador, la empresa encontrada se
  descarta. Medido en staging: unos 0,4 s por llamada una vez caliente y
  0,8 s la primera (arranque en frío de la función). La función SQL de antes
  era un solo viaje: es el costo de validar en el servidor con las reglas del
  núcleo.

## Cambiar una regla

1. Cambiarla en `reglas/src/` y ajustar sus tests (`cargo test` en `reglas/`).
2. Regenerar el paquete WebAssembly:
   ```sh
   rustup target add wasm32-unknown-unknown   # una vez
   # wasm-pack 0.13: https://github.com/rustwasm/wasm-pack/releases
   ./scripts/generar-reglas-wasm.sh
   ```
3. Commitear todo junto: fuentes y lo generado (`web/src/reglas/wasm/` y
   `supabase/functions/_shared/reglas/`).
4. Volver a desplegar la Edge Function que use la regla y el panel web.

Si se olvida el paso 2, **la CI falla**: `web/src/reglas/reglas.test.ts`
calcula la huella de las fuentes de `reglas/` y la compara con la que trae
el paquete commiteado (`huellaFuentes()`, calculada en `reglas/wasm/build.rs`).
El workflow del panel también corre cuando cambia `reglas/**`.

El paquete se genera sin `wasm-opt`, así que no depende de descargar
binaryen: con la misma versión de Rust, regenerar da los mismos bytes.

## Seguridad del navegador

`public/_headers` agrega `'wasm-unsafe-eval'` a `script-src`. Es la directiva
estándar que permite **compilar WebAssembly**; no habilita `eval` ni
`new Function` de JavaScript. El test de la política lo distingue y sigue
prohibiendo `'unsafe-eval'`. Sin esa directiva, el navegador bloquearía el
módulo.

## Desplegar

1. Desplegar la Edge Function `admin-crear-contratista`.
2. Publicar el panel web.
3. Aplicar la migración `20261004170000_alta_de_contratistas_por_edge_function`,
   que retira `panel_crear_contratista`.

En ese orden: si se aplica la migración antes, el panel viejo se queda sin
poder crear contratistas hasta que se publique el nuevo. Primero en staging;
a producción sólo después de probarlo ahí.

## Cambios de comportamiento

- **PRAIND sin acceso:** a quien no tiene acceso no se le pide PRAIND. Era el
  criterio del panel y ahora es el de todos. Al crear desde las apps la
  persona siempre tiene acceso, así que ahí no cambia nada.
- **Orden de los errores:** las reglas de criterio se revisan antes que la
  empresa. Si fallan las dos cosas, se avisa primero el problema de los datos.

## Siguientes etapas

- **Usuarios y dispositivos:** llevar a `reglas/` la parte de criterio (cédula,
  nombre, rol válido, etiqueta y tipo de equipo) y que sus Edge Functions
  (`admin-create-usuario`, `admin-provision-device`) la usen. La autoridad
  (quién puede) sigue en el servidor.
- **Escritorio:** su frontend React repite algunas reglas (`TIPOS_ELEGIBLES`,
  `estaYaAdentro`); puede usar el mismo paquete.
- **Verificaciones de visitas:** pasar las de `desktop/src-tauri/src/comandos/citas.rs`
  al núcleo (`application`), como las de contratistas, proveedores y correo.
