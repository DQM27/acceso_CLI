# Gafete perdido, pagado o recuperado

## Para qué sirve

Que un gafete que no está físicamente **no se pueda asignar por error**, y
dejar escrito qué pasó. Cada paso es un **reporte** que queda a nombre de
quien lo hace, con su detalle en texto libre: quien escribe algo responde
por lo que escribió.

Un gafete tiene dos estados:

| Estado | Qué significa |
|---|---|
| Disponible | Está en la portería y se puede asignar. |
| Perdido | No está: se perdió, nadie sabe dónde está o espera reimpresión. **No se puede asignar.** Puede tener responsable o no. |

Ningún paso es definitivo: un gafete siempre se puede reimprimir.

## Quién puede hacerlo

Cualquier usuario con sesión abierta en el escritorio.

## Dónde

**Gafetes**, en el menú lateral. **Doble clic** en la fila del gafete (o el
botón de la columna **Acciones**) abre sus acciones. Lo que se puede hacer
depende de su estado:

| Estado | Acciones |
|---|---|
| Disponible | **Reportar perdido…** |
| Perdido con responsable | **Registrar pago…** y **Recuperar…** |
| Perdido sin responsable | **Recuperar…** |

## Pasos — reportar un gafete perdido

1. En **Gafetes**, elija el tipo y busque el número.
2. Haga **doble clic** en la fila. Se abre la ventana **Gafete NN**.
3. Pulse **Reportar perdido…**. La ventana pregunta **¿Se sabe a quién se
   le asignó?**
4. **Sí, buscar a la persona**:
   - **Contratista**: escriba la cédula o el nombre y elija a la persona de
     la lista.
   - **Provisional KOF**: escriba el nombre o código del encargado y pulse
     **Elegir**.
   - **Visita**: escriba la cédula del visitante y pulse **Buscar**.

   El **detalle** es opcional. Pulse **Reportar perdido**. El gafete queda
   **Perdido** y esa persona queda marcada.
5. **No se sabe**: escriba el **detalle** (obligatorio), por ejemplo
   *"Recibo el turno sin el gafete 07; me entrega Fulano de Tal"*, y pulse
   **Reportar perdido**. El gafete queda **Perdido** sin responsable.

Los gafetes de **proveedor** van directo a "no se sabe": no hay catálogo de
personas donde buscar.

### Qué pasa después

Cuando un contratista marcado vuelve a entrar (Ctrl+N), aparece el aviso
**"⚠ Este contratista debe el gafete #NN"**. La persona **puede entrar
igual**; el aviso sirve para recordarle la deuda.

## Pasos — registrar el pago

Cuando el responsable paga el gafete, se le quita la marca aunque el gafete
nuevo todavía no exista:

1. Busque el gafete (estado **Perdido**) y ábralo.
2. Pulse **Registrar pago…**, escriba un detalle si hace falta (por
   ejemplo, el número de recibo) y pulse **Registrar pago**.
3. La persona deja de deber el gafete. El gafete **sigue Perdido**: no se
   puede asignar hasta que llegue reimpreso o aparezca.

## Pasos — recuperar un gafete

Cuando el gafete aparece o llega reimpreso:

1. Busque el gafete (estado **Perdido**) y ábralo.
2. Pulse **Recuperar…** y escriba el **detalle** (obligatorio), por
   ejemplo *"Lo encontró Fulano en el parqueo"* o *"Llegó reimpreso"*.
3. Pulse **Recuperar**. El gafete vuelve a **Disponible**. Si todavía tenía
   responsable, se le quita la marca.

## Ver lo que pasó con un gafete

El botón **Detalles** de la columna **Historial** muestra cada reporte:
fecha, hora, qué se hizo, **quién lo reportó**, el responsable y el
detalle que se escribió.

## Mensajes que pueden aparecer

| Mensaje | Qué significa | Qué hacer |
|---|---|---|
| "Escriba el detalle del reporte" | Un reporte sin responsable, o una recuperación, necesita detalle. | Escriba qué pasó. |
| "El gafete está asignado a un ingreso activo" | Ese gafete lo tiene alguien que está adentro ahora. | Registre primero la salida de esa persona (Ctrl+S). |
| "El gafete no está en un estado válido para esa operación" | El gafete cambió de estado (quizá desde el otro equipo). | Cierre la ventana y vuelva a abrir el gafete. |
| "El visitante ya no existe" o un error al buscar la cédula de una visita | No se encontró a esa visita. | Revise la cédula, o repórtelo como "no se sabe" y escriba el nombre en el detalle. |

## Relacionado

- [Conteo de gafetes](conteo-gafetes.md)
- [Gafetes: dar de alta y consultar](gafetes-alta-y-consulta.md)
- [Registrar el ingreso de un contratista](registrar-ingreso-contratista.md)
