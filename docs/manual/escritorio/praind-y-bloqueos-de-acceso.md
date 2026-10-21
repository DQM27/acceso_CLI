# PRAIND vencido, acceso denegado y otros bloqueos

## Para qué sirve

Entender por qué el programa no deja entrar a un contratista y qué hacer en
cada caso.

## Quién puede hacerlo

Cualquier usuario. Algunos casos sólo los puede resolver un administrador.

## Dónde se ve

- En la lista de **Nuevo ingreso** (Ctrl+N): avisos **ACCESO DENEGADO** o
  **PRAIND VENCIDO** al lado del nombre.
- Al elegir a la persona: mensaje en rojo que empieza con
  **"Acceso denegado ·"**.
- En **Contratistas**, columna de estado.

## Qué es el PRAIND

El PRAIND es la inducción que deben tener vigente algunos contratistas para
entrar. El programa revisa la fecha de vencimiento cada vez que se registra
un ingreso.

| Tipo de contratista | ¿Necesita PRAIND? |
|---|---|
| PRAIND | Sí |
| IN_HOUSE | Sí |
| Personal de ruta | Sí |
| SWAT | No |

## Cada bloqueo y qué hacer

| Mensaje al registrar | En Contratistas se ve | Qué significa | Qué hacer |
|---|---|---|---|
| "Acceso denegado · PRAIND vencido" | "PRAIND vencida" | La fecha de vencimiento ya pasó. | No registre el ingreso. La persona debe renovar su PRAIND. Cuando presente el carnet renovado, se actualiza la fecha en su ficha (ver abajo). |
| "Acceso denegado · PRAIND sin fecha registrada" | "Sin PRAIND registrada" | Necesita PRAIND pero su ficha no tiene fecha. | No registre el ingreso. Si presenta el carnet vigente, se registra la fecha en su ficha. |
| "Acceso denegado · no tiene acceso autorizado" | "Acceso denegado" | Un administrador le quitó el acceso. Aplica también si intenta entrar como proveedor o visita. | No registre el ingreso. Sólo un administrador puede devolverle el acceso, desde el panel web. |
| "Acceso denegado · la empresa está inactiva" | "Empresa inactiva" | Su empresa está desactivada; nadie de esa empresa puede entrar. | No registre el ingreso. Avise a un administrador. |

### PRAIND por vencer (sí puede entrar)

Si vence en **30 días o menos**, la persona **puede entrar**. Al elegirla
aparece en amarillo, por ejemplo **"⚠ PRAIND vence en 12 días
(02-11-2026)"**, y en Contratistas se ve **"Puede entrar (PRAIND por
vencer)"**. Registre el ingreso y recuérdele que debe renovarlo.

### Actualizar la fecha del PRAIND

1. Vaya a **Contratistas**.
2. Busque a la persona y haga **doble clic** en su fila.
3. Cambie **Fecha de vencimiento PRAIND** con la fecha del carnet renovado.
4. Pulse **Guardar**.

⚠ Por confirmar: si el operador del puesto de control puede actualizar esta
fecha él mismo o si debe pedirlo a un administrador.

## Relacionado

- [Registrar el ingreso de un contratista](registrar-ingreso-contratista.md)
