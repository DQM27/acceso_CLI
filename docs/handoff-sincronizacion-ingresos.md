# Handoff: menos consultas a la nube en el registro de ingresos

Para la sesión que continúa este trabajo (incluida la sesión local de la PC).
Leer completo antes de tocar nada. Fecha del estado: 19 de octubre de 2026.
Rama: `perf/ingreso-http2` (sin PR).

## Reglas del dueño (obligatorias)

- `AGENTS.md`: comunicarse en español; nunca el término prohibido para el
  puesto de control ("puesto de control", "portería" o "punto de acceso");
  commit bien documentado + push por cada cambio exitoso.
- **Todos los commits van sólo a nombre de `Daniel Quintana
  <daniel.bleach1@gmail.com>`**, como autor y committer. Sin `Co-Authored-By`,
  `Claude-Session` ni otra atribución a asistentes, en commits ni en PR.
- No crear PR sin que se pida. **Nada contra Supabase (staging ni producción)
  sin preguntar.**
- Principio rector: el núcleo Rust decide las reglas y los mensajes; escritorio
  y móvil sólo preguntan y muestran.

## ⚠️ Primero, en la PC

La sesión local tenía cambios **sin commitear** de una versión anterior de esta
idea: caché de 30 s, verificación en segundo plano, botón "Verificando…",
verificación de tres estados en el núcleo y 10 pruebas del modal. Esa versión
se descartó: lo que quedó en la rama es la versión simple (abajo). **No
commitear esos cambios encima.** Guardarlos aparte por si acaso y traer la
rama:

```
git stash push -u -m "version vieja B+A, descartada"
git pull origin perf/ingreso-http2
```

## La decisión

La verificación de un ingreso se partió en dos grupos:

| Lo sabe el equipo (sin red) | Sólo lo sabe la nube |
|---|---|
| Catálogo, PRAIND, acceso denegado, empresa | Si la persona está adentro **en otra unidad**, por cualquier vía |
| Adentro en este equipo o en el otro de la unidad (caché `*_remotos`) | |
| Adentro por otra vía en esta unidad | |
| Gafete ocupado en este equipo o en el otro (misma caché) | |

- La caché del otro equipo la mantiene al día **Realtime, que está en las dos
  apps** (`desktop/src/nubeRealtime.ts`, `mobile/.../NubeRealtime.kt`). El canal
  es privado por sitio: una unidad no ve a las otras, y por eso lo de otras
  unidades sólo lo sabe la nube.
- **Vista previa al elegir a la persona: sólo local.** **Registrar: primero todo
  lo local y después una sola verificación final en vivo**, en paralelo con el
  gafete por si la caché viene un instante atrasada. Lo que se escape (dos
  equipos en el mismo segundo, o sin red) lo atrapan los índices únicos de
  Supabase al sincronizar, como antes.
- Se descartó "reservar" a la persona en Supabase al elegirla: agrega
  peticiones, ataca un caso casi imposible y no funciona sin internet.

## Lo que se hizo (commits en orden)

| Commit | Qué |
|---|---|
| `56da3ff` | Escritorio: los formularios de contratista, proveedor y correo calientan las conexiones con la nube al abrirse (`precalentar_nube`). |
| `8bdb505` | `reqwest` con HTTP/2: las consultas en paralelo comparten una conexión. |
| `af29828` | **Paso 1.** Vista previa local y "local antes de la red" en las cuatro clases. |
| `6eda6f9` | **Paso 2.** Calentamiento en KOF (escritorio) y en los cuatro formularios del móvil. |
| `78c8d13` | **Paso 3.** Avisos en vivo con la fila para proveedor, correo y KOF (migración **sin aplicar**). |

### Paso 1 (`af29828`)

- **Contratista:** la vista previa es `AppCore::preparar_ingreso`, sin red, en
  las dos apps. "Adentro por otra vía en esta unidad" pasó a la preparación
  (`PreparacionIngreso::adentro_por_otra_via`, resuelto en `bloqueo()`). Se
  eliminaron `application::preparar_ingreso_verificado` y el método UniFFI
  `prepararIngresoVerificado`; Kotlin llama a `prepararIngreso`. Por ingreso:
  de 5 consultas en 2 esperas a 3 consultas en 1 espera.
- **Cada servicio separa validar de escribir:** `validar_entrada` (contratista),
  `validar_ingreso` (proveedor y correo), `validar_entrega` (KOF). El `AppCore`
  las expone de sólo lectura (`validar_ingreso`, `validar_ingreso_proveedor`,
  `validar_ingreso_correo`, `validar_entrega_gafete_provisional`).
- Los cuatro `*_verificado` de `src/application/con_nube.rs` corren **todas**
  las reglas locales, más el gafete abierto en el otro equipo
  (`gafete_abierto_en_el_otro_equipo`, que usa `tenedores_del_gafete`), antes de
  consultar la nube. El candado del núcleo se toma una vez para lo local y se
  suelta antes de la red. La escritura vuelve a validar dentro de su
  transacción.
- Cambio de orden en un caso raro: si la persona está adentro por otra vía y
  además tiene el acceso denegado, se muestra el acceso denegado.

### Paso 2 (`6eda6f9`)

- Escritorio: `EntregarGafeteProvisionalModal` pide `precalentarNube`.
- Móvil: método UniFFI `Nucleo::precalentar_nube` (misma función del núcleo que
  el escritorio; vuelve enseguida, no hace nada sin sesión o sin vincular y
  nunca falla). Kotlin calienta al elegir a la persona (contratista), al elegir
  al encargado (KOF) y al aparecer el formulario (proveedor y correo).

### Paso 3 (`78c8d13`)

- Migración `supabase/migrations/20261018130000_cambio_nube_lleva_la_fila_de_las_cuatro_clases.sql`:
  la misma `private.emitir_cambio_nube_sitio()` de antes, con `registro`
  también en `ingresos_proveedor`, `ingresos_correo` y
  `prestamos_gafete_provisional`. Compatible hacia atrás.
- Batería `supabase/tests/cambio_nube_lleva_la_fila.sql` (se revierte sola).
- Núcleo: los módulos de sincronización de las tres tablas exponen
  `guardar_*_remoto` y `aplicar_cierre_*_propio` / `aplicar_devolucion_*`,
  compartidas por la sincronización y por `nube::en_vivo`, que ahora despacha
  por tabla (`TablaDeAbiertos`). En el escritorio el aviso deja la caché al día
  y devuelve `false` para que el historial (sólo copia de la nube) se
  sincronice; en el móvil el aviso basta.
- Anotada como pendiente en `docs/despliegue-produccion.md` (fila 7 y 2.12).

## Qué está verificado y qué no

**Verificado en la nube (Linux):**
- Núcleo: 752 pruebas, y 1010 con `nube`.
- `mobile/rust-core`: 151 pruebas.
- Frontend: 431 pruebas y lint.
- clippy y fmt de núcleo y móvil.
- Escritorio compilado con el target `x86_64-pc-windows-gnu`.
- Bindings de UniFFI regenerados.
- Pruebas nuevas con un firmante que revienta si alguien intenta ir a la nube.
  Prueban que lo local rechaza sin gastar consultas, y se comprobó que fallan si
  se quita la revisión.

**Sin verificar (hacerlo en la PC o en el CI):**
1. **Kotlin y el APK:** el SDK de Android no se pudo bajar (`dl.google.com`
   bloqueado en ese entorno). Correr `./gradlew testDebugUnitTest` y probar en el
   teléfono.
2. **La app de escritorio de verdad en Windows** (`npm run tauri dev`): elegir
   persona, registrar, KOF, proveedor y correo.
3. **La migración del paso 3 no está aplicada en ninguna base**, y la batería
   SQL no se corrió.

## Lo que sigue

1. **Staging:** aplicar la migración del paso 3, correr la batería y probar con
   dos equipos: registrar un proveedor en uno y ver que el otro rechaza ese
   gafete al instante. **Ojo:** la integración de Supabase con GitHub aplica las
   migraciones en producción al hacer merge a `main`; por eso va primero a
   staging.
2. **Juntar con `main`:** `main` no tiene nada que le falte a esta rama. Sus 27
   commits son los mismos cambios, que entraron por los PR #179, #180, #181,
   #189, #190 y #191 con otros hashes. Un merge normal choca en 6 archivos sólo
   porque esta rama siguió cambiándolos. La propuesta es
   `git merge -s ours origin/main`: no cambia ningún archivo y deja el PR con
   sólo los commits nuevos. Confirmar antes que `main` no haya avanzado:
   `git diff origin/main f0d3f29 --stat` debe mostrar sólo
   `docs/handoff-lote-hexagonal-ui-telemetria.md`.
3. PR a `main` cuando el dueño lo pida.

Ideas que quedaron fuera, a propósito:
- Juntar la verificación final en **una sola función de Supabase** (persona en
  cualquier unidad y vía + gafete): 1 petición en vez de 3, con la misma espera.
  Lleva migración.
- Guardar en vivo también el **historial** de proveedor, correo y KOF en el
  escritorio, así no hace falta la sincronización de la tabla después del
  aviso.
- `PreparacionIngreso::activo_en_otro_sitio` quedó siempre en `None`. Se dejó
  porque viaja a las dos apps; quitarlo cambia los bindings.
- Citas: el dueño dijo que no se toca.
