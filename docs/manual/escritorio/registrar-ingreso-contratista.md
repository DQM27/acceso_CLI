# Registrar el ingreso de un contratista

## Para qué sirve

Dejar constancia de que un contratista entró al sitio: a qué hora, cómo
llegó (caminando o en vehículo) y con qué gafete.

## Quién puede hacerlo

Cualquier usuario con sesión abierta en el escritorio.

## Dónde

- Atajo **Ctrl+N**, desde cualquier pantalla.
- O en **Activos**, menú **Ingreso** → **Contratista**.

## Pasos

1. Pulse **Ctrl+N**. Se abre la ventana **Nuevo ingreso**.
2. En **Buscar contratista**, escriba la cédula o el nombre.
3. En la lista aparecen hasta 4 personas. Al lado del nombre pueden verse
   avisos:
   - **Adentro**: ya tiene un ingreso abierto.
   - **ACCESO DENEGADO** o **PRAIND VENCIDO**: no puede entrar.
4. Elija a la persona con las flechas y **Enter**, o con un clic.
5. Si la persona **no puede entrar**, aparece un mensaje en rojo con el
   motivo (ver [PRAIND vencido, acceso denegado y otros
   bloqueos](praind-y-bloqueos-de-acceso.md)). No registre el ingreso.
6. Si puede entrar, complete:
   - **Medio de ingreso**: **Caminando** o **Vehículo**. Si es vehículo,
     escriba la **Placa del vehículo**.
   - **Número de gafete**, si se lo pide. Si no se le entrega gafete, marque
     **Sin gafete**.
7. Pulse **Registrar entrada** (o **Enter**).
8. Verá **"✓ Ingreso registrado — (nombre)"**. La ventana queda abierta y
   vacía para la siguiente persona. Para cerrarla, pulse **Esc**.

Para elegir a otra persona antes de registrar, pulse **Cambiar**.

### ¿Cuándo pide gafete?

| Tipo de contratista | ¿Pide gafete? |
|---|---|
| PRAIND | Sí |
| IN_HOUSE | No |
| SWAT | No |
| Personal de ruta | No |

## Mensajes que pueden aparecer

| Mensaje | Qué significa | Qué hacer |
|---|---|---|
| "⚠ PRAIND vence en N días (fecha)" (o "vence hoy", "vence mañana") | Puede entrar, pero su PRAIND está por vencer (30 días o menos). | Registre el ingreso y avísele que debe renovarlo. |
| "⚠ Este contratista debe el gafete #05" | Tiene un gafete pendiente de una visita anterior (perdido o no devuelto). | Puede registrar el ingreso. Recuérdele la deuda. ⚠ Por confirmar: si el sitio exige algo más. |
| "Ingrese el número de gafete o marque «Sin gafete»" | Este contratista requiere gafete y el campo está vacío. | Escriba el número del gafete que le entrega o marque **Sin gafete**. |
| "Ingrese un número de gafete válido" | El número escrito no es válido. | Escriba sólo el número del gafete. |
| "El contratista ya tiene un ingreso activo." | Ya figura adentro. | Si realmente había salido, registre primero su salida (Ctrl+S) y luego el ingreso. |
| "El contratista ya tiene un ingreso activo en (sitio)." | Figura adentro en otro sitio. | No registre el ingreso. Avise a un administrador para que lo revise con el otro sitio. |
| "La nube rechazó la verificación de si el contratista ya tiene un ingreso activo. Intente de nuevo; si se repite, avise a un administrador para revisar este equipo." | No se pudo confirmar con la nube si ya está adentro en otro lugar. | Intente otra vez. Si se repite, avise a un administrador. |
| Mensajes que empiezan con "Acceso denegado ·" | No puede entrar. | Ver [PRAIND vencido, acceso denegado y otros bloqueos](praind-y-bloqueos-de-acceso.md). |

## Relacionado

- [Registrar una salida](registrar-salida.md)
- [PRAIND vencido, acceso denegado y otros bloqueos](praind-y-bloqueos-de-acceso.md)
