# Reglas de negocio duplicadas o faltantes en escritorio

Rama `claude/dominio-en-nucleo`: las reglas que decidía Kotlin se movieron
al núcleo Rust (dominio + servicios). La app móvil ahora sólo pregunta y
muestra. **El escritorio no se tocó** (pedido del dueño: "si la desktop
hace lo mismo no tocar, sólo dejar una nota"). Esta nota lista lo que el
escritorio decide por su cuenta, para un refactor aparte y en orden.

## 1. Contratista (crear y editar) — RESUELTO en `claude/dominio-escritorio`

Todas las reglas viven en `ContratistaService::armar` y valen igual para
escritorio y móvil: cédula sólo dígitos, nombre sólo letras en
MAYÚSCULAS, personal de ruta sólo PRAIND/IN HOUSE, PRAIND obligatorio y
vigente, alta siempre con acceso. Al editar, "vigente" y "personal de
ruta" sólo se revisan si cambió la fecha, el tipo o la casilla.

El formulario de escritorio pregunta qué mostrar a
`reglas_formulario_contratista` (Tauri) y ya no tiene `requierePraind()`
ni los esquemas de cédula/nombre en TS.

## 2. Ingreso de proveedor

| Regla | Dónde vive ahora | Escritorio hoy |
|---|---|---|
| Cédula con ingreso abierto en el sitio (este equipo **o el otro dispositivo**) | `AppCore::proveedor_con_ingreso_activo_en_sitio` | Sólo lo local (`IngresoProveedorService` → `IngresoActivo`); no mira la caché `ingresos_proveedor_remotos` |
| Activo en otro sitio + gafete en uso en el otro dispositivo, antes de escribir | Móvil: `Nucleo::registrar_ingreso_proveedor_con_secreto` (una llamada) | **Orquestado en el comando Tauri**: `desktop/src-tauri/src/comandos/proveedores.rs` (`proveedor_activo_en_otro_sitio`, `gafete_proveedor_libre_en_otro_dispositivo`) |

Qué hacer: llevar la orquestación de `comandos/proveedores.rs` a una
función del núcleo (`src/application/proveedores.rs`) que usen ambos, con
`proveedor_con_ingreso_activo_en_sitio` como primer chequeo. Ojo: el
escritorio frena si la consulta del gafete falla (a propósito); el móvil
hace lo mismo al usar `gafete_de_proveedor_ocupado_en_sitio_con_secreto`.

## 3. Entrega de gafete provisional KOF

| Regla | Dónde vive ahora | Escritorio hoy |
|---|---|---|
| Gafete prestado en el otro dispositivo del sitio, antes de escribir | Móvil: `Nucleo::entregar_gafete_provisional_con_secreto` (una llamada) | **Orquestado en el comando Tauri**: `desktop/src-tauri/src/comandos/gafetes_provisionales.rs` (chequeo de nube + `entregar_gafete_provisional`) |

Qué hacer: mismo movimiento que en proveedores, una función del núcleo
que chequea y escribe, usada por ambos.

## 4. Placa y gafete al registrar el ingreso de un contratista

| Regla | Dónde vive ahora | Escritorio hoy |
|---|---|---|
| La placa sólo existe en vehículo; con "Caminando" se descarta, sin espacios en los bordes | `domain::registro_ingreso::placa_segun_medio` (lo aplica `Nucleo::registrar_ingreso`) | **Duplicada en TS**: `desktop/src/pantallas/NuevoIngresoModal.tsx` (`confirmarIngreso` + `validarPlaca`) |
| Placa obligatoria en vehículo, gafete obligatorio según el contratista | `RegistroIngresoService` (`PlacaRequerida`, `GafeteRequerido`), ya existía | **Pre-validada en TS** con su propio texto antes de llamar al núcleo |

Qué hacer: que el modal mande lo tipeado y muestre el mensaje del núcleo
(`mensaje_ingreso`); para eso el comando Tauri debe aplicar
`placa_segun_medio` igual que el puente móvil.

## Otros restos de lógica en Kotlin detectados (fuera de esta rama)

- `mensajeVencimientoPraind` (`PantallaConfirmarIngreso.kt`): arma "vence
  en N días (fecha)" con `LocalDate.now()`. Es espejo de
  `desktop/src/api/ingresos.ts`; debería salir del núcleo junto con
  `PreparacionIngreso` (mismo reloj que el resto de las reglas).
