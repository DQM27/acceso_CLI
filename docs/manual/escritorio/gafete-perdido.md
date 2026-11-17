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
botón de la columna **Acciones**) abre la ventana del gafete: **Reportar
perdido · Gafete NN · Tipo** si está disponible, **Gafete NN perdido · Tipo**
si ya está perdido. Las opciones son tarjetas: se elige una y lo que pide aparece debajo. Abajo,
**Cancelar** sale sin hacer nada y el botón de la derecha confirma; se
habilita cuando el reporte está completo. Antes de confirmar, la ventana
muestra **Reportado por** con su usuario.

| Estado | Opciones |
|---|---|
| Disponible | **Responsable conocido** y **Responsable desconocido** (reportar perdido) |
| Perdido con responsable | **Registrar pago** y **Recuperar** |
| Perdido sin responsable | **Recuperar** |

## Pasos — reportar un gafete perdido

1. En **Gafetes**, elija el tipo y busque el número.
2. Haga **doble clic** en la fila. La ventana abre en **Reportar perdido** y
   pregunta **¿Se sabe a quién se le asignó?**
3. **Responsable conocido**:
   - **Contratista**: escriba la cédula o el nombre y elija a la persona de
     la lista.
   - **Provisional KOF**: escriba el nombre o código de la persona y pulse
     **Elegir**.
   - **Visita**: escriba la cédula del visitante y pulse **Buscar**.

   El **detalle** es opcional. Pulse **Reportar perdido**. El gafete queda
   **Perdido** y esa persona queda marcada.
4. **Responsable desconocido**: escriba el **detalle** (obligatorio), por
   ejemplo *"Se recibe el turno sin el gafete 07; entrega Fulano de Tal"*, y pulse
   **Reportar perdido**. El gafete queda **Perdido** sin responsable.

Los gafetes de **proveedor** solo tienen **Responsable desconocido**: no hay
catálogo de personas donde buscar.

### Qué pasa después

Cuando un contratista marcado vuelve a entrar (Ctrl+N), aparece el aviso
**"⚠ Este contratista debe el gafete #NN"**. La persona **puede entrar
igual**; el aviso sirve para recordarle la deuda.

## Pasos — registrar el pago

Cuando el responsable paga el gafete, se le quita la marca aunque el gafete
nuevo todavía no exista:

1. Busque el gafete (estado **Perdido**) y ábralo.
2. Elija **Registrar pago**, escriba un detalle si hace falta (por
   ejemplo, el número de recibo) y pulse **Registrar pago**.
3. La persona deja de deber el gafete. El gafete **sigue Perdido**: no se
   puede asignar hasta que llegue reimpreso o aparezca.

## Pasos — recuperar un gafete

Cuando el gafete aparece o llega reimpreso:

1. Busque el gafete (estado **Perdido**) y ábralo.
2. Elija **Recuperar** y escriba el **detalle** (obligatorio), por
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
| "Gafete asignado a NOMBRE: registre primero su salida" | Ese gafete lo tiene asignado alguien que está adentro ahora. | Registre primero la salida de esa persona (Ctrl+S). |
| "El gafete cambió de estado: cierre y vuelva a abrirlo" | El gafete cambió de estado (quizá desde el otro equipo). | Cierre la ventana y vuelva a abrir el gafete. |
| "Visitante no encontrado" o un error al buscar la cédula de una visita | No se encontró a esa visita. | Revise la cédula, o repórtelo como "no se sabe" y escriba el nombre en el detalle. |

## Relacionado

- [Conteo de gafetes](conteo-gafetes.md)
- [Gafetes: dar de alta y consultar](gafetes-alta-y-consulta.md)
- [Registrar el ingreso de un contratista](registrar-ingreso-contratista.md)
