# Entregar un gafete provisional KOF

## Para qué sirve

Prestar un gafete provisional a un encargado de KOF y dejar constancia de
a quién se le entregó. Los encargados no se buscan por cédula sino por
**nombre o código de empleado**.

## Quién puede hacerlo

Cualquier usuario con sesión abierta en el escritorio.

## Dónde

- Atajo **Alt+K**, desde cualquier pantalla.
- O en **Activos**, menú **Ingreso** → **Gafete KOF**.

## Pasos

1. Pulse **Alt+K**. Se abre **Entregar gafete provisional KOF**.
2. En **Buscar encargado**, escriba el nombre o el código de empleado.
3. Elija al encargado de la lista (flechas y **Enter**, o clic).
   - Si al lado dice **Tiene #04**, ese encargado no ha devuelto un gafete:
     no se le puede entregar otro (ver abajo).
4. Escriba el **Número de gafete provisional**.
5. Pulse **Entregar gafete** (o **Enter**). La ventana se cierra y el
   préstamo aparece en **Activos**.

Para elegir a otro encargado antes de entregar, pulse **Cambiar**.

### Cuando devuelve el gafete

Registre la devolución como una salida: **Ctrl+S**, elija **Gafete** y
escriba el número. Ver [Registrar una salida](registrar-salida.md).

## Mensajes que pueden aparecer

| Mensaje | Qué significa | Qué hacer |
|---|---|---|
| "⚠ Ya tiene prestado el gafete #04 sin devolver" | El encargado tiene un gafete pendiente. | No se le puede entregar otro hasta que devuelva ese. Registre primero la devolución (Ctrl+S, por gafete). |
| "Este encargado ya tiene un gafete provisional prestado" | Al intentar entregar, el programa confirmó que tiene uno pendiente (en este sitio o en otro). | Igual que el anterior: primero la devolución. Si dice que lo devolvió en otro sitio, avise a un administrador. |
| "El número de gafete es obligatorio" / "Ingrese un número de gafete válido" | Falta el número o no es válido. | Escriba el número del gafete provisional. |
| "Ese número de gafete ya está prestado a otra persona" | Ese gafete lo tiene otro encargado. | Entregue otro gafete. |
| "El gafete N ya está prestado en otro dispositivo del sitio" | Se entregó en el otro equipo del sitio. | Entregue otro gafete. |
| "El número de gafete provisional no existe en el catálogo" | Ese número no existe como gafete provisional KOF. | Revise el número. Si es un gafete nuevo, se agrega en **Gafetes**. |
| "El gafete está marcado como perdido" / "El gafete está dado de baja" | Ese gafete no se puede usar. | Entregue otro gafete. |
| "El encargado está dado de baja en el catálogo" | El encargado ya no está activo. | No entregue el gafete. Avise a un administrador. |
| No aparece el encargado en la lista | No está en el catálogo de encargados de este sitio. | Avise a un administrador para que lo agregue. |

## Relacionado

- [Registrar una salida](registrar-salida.md)
