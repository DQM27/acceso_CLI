# Gafetes: dar de alta y consultar

## Para qué sirve

Mantener el catálogo de gafetes del sitio: agregar gafetes nuevos, ver en
qué estado está cada uno y quién lo tiene.

Hay **cuatro tipos** de gafete, y **cada tipo tiene su propia numeración**:
el gafete 5 de contratista y el gafete 5 de proveedor son dos gafetes
distintos.

| Tipo | Se entrega a |
|---|---|
| Contratista | Contratistas (Ctrl+N) |
| Proveedor | Proveedores (Alt+P) |
| Visita | Visitas por correo (Alt+C) |
| Provisional KOF | Encargados de KOF (Alt+K) |

Un gafete sólo se puede entregar si existe en el catálogo de su tipo.

## Quién puede hacerlo

Cualquier usuario con sesión abierta en el escritorio.

## Dónde

**Gafetes**, en el menú lateral.

## Consultar gafetes

1. Abra **Gafetes**.
2. Elija el tipo en el selector de la derecha (**Contratista**, **Visita**,
   **Provisional KOF** o **Proveedor**). Se ve un tipo a la vez.
3. Para buscar uno, escriba el número en el buscador ("Número…").

Columnas:

| Columna | Qué muestra |
|---|---|
| Número | Número del gafete |
| Tipo | Tipo del gafete |
| Estado | **Disponible**, **Perdido** o **De baja** |
| Asignado a | Quién lo debe, si está perdido |
| Resolver | Botón, sólo en gafetes perdidos (ver [Gafete perdido](gafete-perdido.md)) |
| Historial | Botón **Detalles**: los incidentes del gafete |

## Pasos — agregar gafetes

1. Pulse **+** (Nuevo gafete).
2. En **Tipo de gafete**, elija **Contratista**, **Proveedor**, **KOF** o
   **Visita**.
3. En **Números**, elija:
   - **Uno**: escriba el número.
   - **Rango**: escriba **Desde** y **Hasta** (máximo 200 a la vez).
4. Revise la frase que aparece debajo, por ejemplo "Se crearán 20 gafetes
   de contratista, del #01 al #20."
5. Pulse **Crear gafete** (o **Crear N gafetes**).

## Ver el historial de un gafete

En la fila del gafete, pulse **Detalles**. Se ve cada incidente: cuándo se
marcó **perdido**, a quién se asignó, cuándo se **resolvió**, con qué
motivo y qué operador lo hizo. Si nunca tuvo problemas, dice "Este gafete
no tiene incidentes registrados."

## Mensajes que pueden aparecer

| Mensaje | Qué significa | Qué hacer |
|---|---|---|
| "Ingrese un número de gafete válido" | El número está vacío o es cero. | Escriba un número mayor a cero. |
| 'Ingrese un "desde" válido' / "El rango no es válido" | El rango está incompleto o al revés. | Revise que **Hasta** sea mayor o igual que **Desde**. |
| "El rango es demasiado grande (máximo 200 a la vez)" | Pidió más de 200 gafetes. | Créelos en varias tandas. |
| "Ya existe un gafete con ese número" | Ese número ya existe para ese tipo. | No hace falta crearlo; búsquelo en la tabla. |

## Relacionado

- [Gafete perdido, pagado o aparecido](gafete-perdido.md)
