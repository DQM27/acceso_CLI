# Gafete perdido, pagado o aparecido

## Para qué sirve

Registrar que una persona perdió (o no devolvió) un gafete, para que quede
como **deuda** suya, y cerrar esa deuda cuando lo paga o el gafete aparece.
También sirve para retirar un gafete que ya no se va a usar.

## Quién puede hacerlo

Cualquier usuario con sesión abierta en el escritorio.

## Dónde

**Gafetes**, en el menú lateral. **Doble clic** en la fila del gafete abre
sus acciones. Lo que se puede hacer depende de su estado:

| Estado | Acciones |
|---|---|
| Disponible | **Marcar perdido…** y **Dar de baja** |
| Perdido | **Resolver · pagado** y **Resolver · apareció** |
| De baja | Ninguna: es definitivo |

## Pasos — marcar un gafete como perdido

1. En **Gafetes**, elija el tipo y busque el número.
2. Haga **doble clic** en la fila. Se abre la ventana **Gafete NN**.
3. Pulse **Marcar perdido…**.
4. Indique quién lo tenía:
   - **Contratista**: escriba la cédula o el nombre y elija a la persona de
     la lista. Se marca al elegirla.
   - **Provisional KOF**: escriba el nombre o código del encargado, elíjalo
     y pulse **Confirmar**.
   - **Visita**: escriba la cédula del visitante y pulse **Confirmar**.
5. El gafete queda **Perdido** y **Asignado a** esa persona.

Los gafetes de **proveedor** no tienen la opción de marcar perdido.

### Qué pasa después

Cuando ese contratista vuelve a entrar (Ctrl+N), aparece el aviso **"⚠ Este
contratista debe el gafete #NN"**. La persona **puede entrar igual**; el
aviso sirve para recordarle la deuda.

## Pasos — cerrar la deuda (pagado o apareció)

1. Busque el gafete (estado **Perdido**).
2. Pulse **Resolver** en su fila (o doble clic).
3. Elija:
   - **Resolver · pagado**: la persona pagó el gafete.
   - **Resolver · apareció**: el gafete se encontró.
4. El gafete vuelve a **Disponible** y la deuda desaparece.

## Dar de baja un gafete

Para un gafete dañado o que no se va a volver a usar: doble clic → **Dar de
baja**.

⚠ Se aplica **al instante, sin pedir confirmación**, y **no se puede
deshacer**. Revise el número antes de pulsar.

## Mensajes que pueden aparecer

| Mensaje | Qué significa | Qué hacer |
|---|---|---|
| "El gafete está asignado a un ingreso activo" | Ese gafete lo tiene alguien que está adentro ahora. | Registre primero la salida de esa persona (Ctrl+S). |
| "El gafete no está en un estado válido para esa operación" | El gafete cambió de estado (quizá desde el otro equipo). | Cierre la ventana y vuelva a abrir el gafete. |
| "El visitante ya no existe" o un error al buscar la cédula de una visita | No se encontró a esa visita. | Revise la cédula. Si sigue sin funcionar, avise a un administrador. |

## Relacionado

- [Gafetes: dar de alta y consultar](gafetes-alta-y-consulta.md)
- [Registrar el ingreso de un contratista](registrar-ingreso-contratista.md)
