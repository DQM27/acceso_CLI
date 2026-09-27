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

## 2. Ingreso de proveedor — RESUELTO en `claude/dominio-escritorio`

`application::registrar_ingreso_proveedor_verificado` (`src/application/con_nube.rs`)
decide el orden y qué falla frena: cédula abierta en el sitio (este equipo
o el otro dispositivo), activa en otro sitio (mejor esfuerzo), gafete en
uso en el otro dispositivo (si no se puede verificar, frena) y recién ahí
escribe. Toma el candado del núcleo sólo para lo local, nunca durante la
red. La usan el comando Tauri `registrar_ingreso_proveedor` y
`Nucleo::registrar_ingreso_proveedor_con_secreto`; el texto sale de
`mensajes::mensaje_ingreso_proveedor_verificado`.

Pendiente de limpieza (requiere regenerar bindings y probar Android):
`Nucleo::proveedor_activo_en_otro_sitio_con_secreto` y
`Nucleo::gafete_de_proveedor_ocupado_en_sitio_con_secreto` ya no los usa
nadie.

## 3. Entrega de gafete provisional KOF — RESUELTO en `claude/dominio-escritorio`

`application::entregar_gafete_provisional_verificado` (`con_nube.rs`):
si el gafete está prestado en el otro dispositivo del sitio, o no se
puede verificar, no se entrega; recién ahí escribe. La usan el comando
Tauri `entregar_gafete_provisional` y
`Nucleo::entregar_gafete_provisional_con_secreto`; el texto sale de
`mensajes::mensaje_entrega_gafete_provisional_verificada`.

Pendiente de limpieza: `Nucleo::gafete_provisional_ocupado_en_sitio_con_secreto`
ya no lo usa nadie.

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
