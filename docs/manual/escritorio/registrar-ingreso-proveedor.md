# Registrar el ingreso de un proveedor

## Para qué sirve

Dejar constancia de que entró una persona de una empresa proveedora
(entregas, servicios), con su empresa, su placa si viene en vehículo y el
gafete que se le entrega.

## Quién puede hacerlo

Cualquier usuario con sesión abierta en el escritorio.

## Dónde

- Atajo **Alt+P**, desde cualquier pantalla.
- O en **Activos**, menú **Ingreso** → **Proveedor**.

## Pasos

1. Pulse **Alt+P**. Se abre **Nuevo ingreso de proveedor**.
2. En **Buscar proveedor**, escriba la cédula o el nombre.
3. En la lista aparecen las personas que **ya entraron antes** a este sitio.
   Si alguna tiene el aviso **Adentro #07**, ya está adentro con ese gafete.
   - Si la persona está en la lista, elíjala: se completan su nombre y su
     última empresa.
   - Si es la primera vez, elija **Nuevo proveedor** (siempre está al final)
     y escriba **Cédula** y **Nombre**.
4. En **Empresa**, escriba y elija la empresa de la lista. Tiene que estar
   en el catálogo; no se puede escribir una empresa nueva aquí.
5. **Placa (opcional)**: escríbala si viene en vehículo. Si la deja vacía,
   queda registrado como caminando.
6. Escriba el **N.° de gafete** que le entrega. Es obligatorio.
7. Pulse **Registrar ingreso** (o **Enter**). La ventana se cierra y la
   persona aparece en **Activos**.

Para elegir a otra persona antes de registrar, pulse **Cambiar**.

La salida se registra igual que la de cualquier persona: ver [Registrar una
salida](registrar-salida.md).

## Mensajes que pueden aparecer

| Mensaje | Qué significa | Qué hacer |
|---|---|---|
| "⚠ Ya está adentro con el gafete #07" | La persona figura adentro como proveedor. | No la registre otra vez. Si en realidad salió, registre primero su salida (Ctrl+S). |
| "La cédula es obligatoria" / "El nombre es obligatorio" | Falta ese dato. | Complételo. |
| "La cédula debe tener sólo números, entre 9 y 13 dígitos" | La cédula no tiene el formato correcto. | Escríbala sin letras. Los proveedores sólo entran con cédula nacional o de extranjero; el pasaporte no se acepta. |
| "Elija una empresa del catálogo" | No eligió una empresa de la lista. | Escriba y elija la empresa de la lista. Si no existe, se agrega en **Empresas**. |
| "La empresa proveedora está dada de baja" | La empresa está desactivada. | No registre el ingreso. Avise a un administrador. |
| "El número de gafete es obligatorio" / "Ingrese un número de gafete válido" | Falta el gafete o no es un número válido. | Escriba el número del gafete que le entrega. |
| "El gafete ya está asignado a otro ingreso de proveedor" | Ese gafete lo tiene otra persona adentro. | Entregue otro gafete. |
| "El gafete no está registrado en el catálogo" | Ese número no existe como gafete de proveedor. | Revise el número. Si es un gafete nuevo, se agrega en **Gafetes**. |
| "El gafete no está disponible" | El gafete está marcado como perdido o dado de baja. | Entregue otro gafete. |
| "El gafete N ya está en uso en otro dispositivo del sitio" | Ese gafete se entregó en el otro equipo del sitio. | Entregue otro gafete. |
| "Esta persona tiene el acceso denegado." | Un administrador le quitó el acceso a esta cédula; aplica en todas las entradas. | No registre el ingreso. Sólo un administrador puede devolverle el acceso. |
| "Esta persona ya está adentro (como contratista, por ejemplo) — registre primero esa salida" | Figura adentro por otra vía. | Registre primero esa salida (Ctrl+S) y luego este ingreso. |
| "Esta cédula ya tiene un ingreso de proveedor activo en (sitio)" | Figura adentro en otro sitio. | No registre el ingreso. Avise a un administrador. |
| "Revise la fecha y hora del equipo antes de continuar" | El reloj del equipo está mal. | Avise a un administrador para que lo corrija. |

## Relacionado

- [Registrar una salida](registrar-salida.md)
- [Registrar el ingreso de un contratista](registrar-ingreso-contratista.md)
