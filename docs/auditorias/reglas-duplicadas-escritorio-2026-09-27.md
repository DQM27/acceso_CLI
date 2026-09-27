# Reglas de negocio duplicadas o faltantes en escritorio

Rama `claude/dominio-en-nucleo`: las reglas que decidía Kotlin se movieron
al núcleo Rust (dominio + servicios). La app móvil ahora sólo pregunta y
muestra. **El escritorio no se tocó** (pedido del dueño: "si la desktop
hace lo mismo no tocar, sólo dejar una nota"). Esta nota lista lo que el
escritorio decide por su cuenta, para un refactor aparte y en orden.

## 1. Alta de contratista

| Regla | Dónde vive ahora | Escritorio hoy |
|---|---|---|
| Requiere PRAIND (personal de ruta, PRAIND, IN HOUSE) | `domain::contratista::requiere_praind_de` | **Duplicada en TS**: `desktop/src/api/contratistas.ts` → `requierePraind()` |
| PRAIND vencido no se registra | `domain::contratista::praind_vencido` + `ContratistaService::crear_en_persona` | **No la aplica**: `FormularioContratista.tsx` guarda una fecha pasada |
| Personal de ruta sólo PRAIND / IN HOUSE | `domain::contratista::admite_personal_ruta` + `crear_en_persona` | **No la aplica**: la casilla se ve para todos los tipos, y el test `debe_crear_personal_de_ruta_con_fecha` (`tests/contratista_service.rs`) crea POR CORREO + ruta con `crear` |
| Alta en persona queda con acceso | `crear_en_persona` | El formulario deja elegir `tiene_acceso` (tiene sentido en la administración) |

Qué hacer en el refactor del escritorio:

1. Decidir si el formulario de escritorio es "alta en persona" (usar
   `AppCore::crear_contratista_en_persona`) o administración (seguir con
   `crear_contratista`, pero aplicando igual PRAIND vencido y personal de
   ruta: moverlos de `crear_en_persona` a `construir_contratista` y ajustar
   el test citado).
2. Reemplazar `requierePraind()` de TS por un comando Tauri que llame a
   `requiere_praind_de`, igual que el móvil (`requierePraindParaFormulario`).
3. Ocultar la casilla de personal de ruta con `admite_personal_ruta`.
