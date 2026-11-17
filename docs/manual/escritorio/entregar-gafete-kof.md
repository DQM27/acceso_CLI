# Entregar un gafete provisional KOF

## Para qué sirve

Prestar un gafete provisional a una persona del Personal KOF y dejar constancia de
a quién se le entregó. No se busca por cédula sino por
**nombre o código de empleado**.

## Quién puede hacerlo

Cualquier usuario con sesión abierta en el escritorio.

## Dónde

- Atajo **Alt+K**, desde cualquier pantalla.
- O en **Activos**, menú **Ingreso** → **Provisional KOF**.

## Pasos

1. Pulse **Alt+K**. Se abre **Entregar gafete provisional KOF**.
2. En **Buscar Personal KOF**, escriba el nombre o el código de empleado.
3. Elija a la persona de la lista (flechas y **Enter**, o clic).
   - Si al lado dice **Tiene #04**, esa persona no ha devuelto un gafete:
     no se le puede entregar otro (ver abajo).
4. Escriba el **Número de gafete provisional**.
5. Pulse **Entregar gafete** (o **Enter**). La ventana se cierra y el
   préstamo aparece en **Activos**.

Para elegir a otra persona antes de entregar, pulse **Cambiar**.

### Cuando devuelve el gafete

Registre la devolución como una salida: **Ctrl+S**, elija **Gafete** y
escriba el número. Ver [Registrar una salida](registrar-salida.md).

## Mensajes que pueden aparecer

| Mensaje | Qué significa | Qué hacer |
|---|---|---|
| "⚠ Ya tiene prestado el gafete #04 sin devolver" | La persona tiene un gafete pendiente. Aunque se vea en amarillo, **es un bloqueo**: mientras no lo devuelva, no se le puede entregar otro. | Registre primero la devolución (Ctrl+S, por gafete) y luego entregue el nuevo. |
| "Esta persona ya tiene un gafete provisional prestado" | Al intentar entregar, el programa confirmó que tiene uno pendiente (en este sitio o en otro). | Igual que el anterior: primero la devolución. Si dice que lo devolvió en otro sitio, avise a un administrador. |
| "El número de gafete es obligatorio" / "Ingrese un número de gafete válido" | Falta el número o no es válido. | Escriba el número del gafete provisional. |
| "Ese número de gafete ya está prestado a otra persona" | Ese gafete lo tiene otra persona. | Entregue otro gafete. |
| "Gafete N prestado en el otro equipo" | Se entregó en el otro equipo del sitio. | Entregue otro gafete. |
| "Ese gafete no existe en el catálogo" | Ese número no existe como gafete provisional KOF. | Revise el número. Si es un gafete nuevo, se agrega en **Gafetes**. |
| "Gafete perdido: no se puede asignar" | Ese gafete no se puede usar. | Entregue otro gafete. |
| "Personal KOF dado de baja en el catálogo" | La persona ya no está activa. | No entregue el gafete. Avise a un administrador. |
| No aparece la persona en la lista | No está en el catálogo de Personal KOF de este sitio. | Avise a un administrador para que lo agregue. |

## Relacionado

- [Registrar una salida](registrar-salida.md)
