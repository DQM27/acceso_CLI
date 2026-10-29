# Registrar el ingreso de un contratista

## Para qué sirve

Dejar constancia de que un contratista entra al sitio. Antes de registrar,
la app revisa que tenga acceso, PRAIND vigente y que no esté adentro en
otro lado.

## Quién puede hacerlo

Cualquier usuario con sesión abierta.

## Dónde

Sección **Contratistas** (es la que se ve al entrar).

## Pasos — escaneando el documento (lo más rápido)

1. En **Contratistas**, toque el botón redondo de la cámara (**Escanear
   documento**).
2. Coloque la cédula (o el carnet) **dentro del recuadro**.
3. Si la app reconoce a la persona, pasa directo al formulario de ingreso.
   Si hay varias parecidas, lo leído queda en la búsqueda: elija la
   correcta.
4. Escriba el número de gafete y toque **Registrar ingreso** (o la tecla
   **✓** del teclado).
5. Verá **"Ingreso registrado · (nombre)"**.

Si la persona **ya está adentro**, en vez del formulario sube su salida:
ver [Registrar la salida de un contratista](registrar-salida-contratista.md#pasos--escaneando-la-cédula-de-quien-sale).

**Sonido y vibración** al escanear, para saber qué pasó sin mirar la
pantalla:

- **Tres notas largas**: **acceso denegado**. No puede entrar.
- **Dos pitidos**: puede entrar, pero hay una **advertencia** (por ejemplo,
  PRAIND por vencer).

Estos sonidos van por el canal de **alarma**: suenan aunque el teléfono
esté en silencio.

## Pasos — buscando por nombre o cédula

1. En **Contratistas**, toque la búsqueda de abajo (**"Nombre, cédula o
   gafete"**) y escriba el nombre o la cédula (al menos 4 dígitos: con
   menos, se buscan gafetes).
2. Toque a la persona en los resultados.
3. Complete el formulario y toque **Registrar ingreso**.

Algunas tarjetas se distinguen por su color, con un ícono y una palabra a
la derecha, y un renglón chico que dice qué hacer:

- **Ámbar, ADENTRO**: ya está adentro. Tocarla abre su salida, no un
  ingreso (**"Toque para registrar su salida"**).
- **Roja, DENEGADO** (círculo con una raya): no tiene acceso autorizado
  (**"No puede ingresar: lo habilita la administración"**).
- **Roja, VENCIDO** (calendario con una X): su PRAIND está vencido
  (**"No puede ingresar hasta renovar el PRAIND"**).

## El formulario de ingreso

Arriba se ve el nombre y "cédula · empresa". Luego:

1. **Medio de ingreso**: **Caminando** o **Vehículo**.
2. Con **Vehículo**: **Placa del vehículo**. Puede escribirla o tocar el
   botón de cámara (**Escanear placa**) y apuntar a la placa.
3. **Número de gafete**: el número del gafete que le entrega. Si entra sin
   gafete, toque **S/G** (sin gafete): borra el número y queda encendido.
   Sólo aparece para los tipos que llevan gafete (PRAIND).
4. **Registrar ingreso** se activa cuando está todo: el número de gafete o
   **S/G** encendido, y la placa si entra en vehículo. Así nunca queda "sin
   gafete" por olvido. La tecla **✓** del teclado también registra.

**‹ Volver** (arriba a la izquierda) cancela sin registrar.

Avisos que pueden verse en el formulario (dejan entrar):

- **"⚠ PRAIND vence en N días (fecha)"** (o "vence hoy", "vence mañana"):
  registre el ingreso y avísele que debe renovarlo.
- **"⚠ Este contratista debe el gafete #05"**: tiene un gafete perdido a
  su nombre. Puede entrar; recuérdele que lo debe.

## Si no se puede registrar

Si la persona no puede entrar, en vez del formulario se ve el motivo, con
**‹ Volver**. Los más comunes:

| Mensaje | Qué hacer |
|---|---|
| "Acceso denegado · PRAIND vencido" | No puede entrar hasta renovar el PRAIND. |
| "Acceso denegado · PRAIND sin fecha registrada" | Falta la fecha del PRAIND en sus datos. Avise a un administrador o corríjala en el escritorio. |
| "Acceso denegado · no tiene acceso autorizado" | Un administrador le quitó el acceso. No puede entrar. |
| "Acceso denegado · la empresa está inactiva" | Su empresa está desactivada. Avise a un administrador. |
| "El contratista ya tiene un ingreso activo en (sitio)." | Ver [Problemas frecuentes](problemas-frecuentes.md#el-contratista-ya-tiene-un-ingreso-activo). |
| "Esta persona ya está adentro como proveedor — registre primero esa salida" (o "por correo") | Debajo se ve ese ingreso (gafete y hora). Toque **Registrar esa salida y continuar**: se cierra ahí mismo y pasa al formulario de ingreso. |

Otros mensajes al registrar:

| Mensaje | Qué significa | Qué hacer |
|---|---|---|
| "Ingrese un número de gafete válido" | El número escrito no es válido. | Escriba sólo el número. |
| "El gafete ya está en uso" | Ese gafete lo tiene otra persona adentro. | Entregue otro gafete. |
| "El gafete N ya está en uso en otro dispositivo del sitio" | Lo entregó la PC del puesto. | Entregue otro gafete. |
| "El número de gafete no existe en el catálogo" | Ese número no está registrado. | Revise el número. |
| "El gafete está marcado como perdido" / "El gafete está dado de baja" | Ese gafete no se puede usar. | Entregue otro gafete. |
| "La placa es obligatoria cuando el ingreso es en vehículo" | Falta la placa. | Escríbala o escanéela. |
| "Revise la fecha y hora del equipo antes de continuar" | La hora del teléfono quedó antes del último registro. | Ver [Problemas frecuentes](problemas-frecuentes.md#revise-la-fecha-y-hora-del-equipo). |

## Si la persona no aparece

Si escanea o busca y no aparece, dice **"No está registrado"** con la
opción **Registrar nuevo contratista**. Tóquela: se abre la cámara del
carnet PRAIND con lo que buscó ya puesto y, al guardar, pasa directo a su
ingreso. Ver [Nuevo contratista](nuevo-contratista.md).

## Relacionado

- [Registrar la salida de un contratista](registrar-salida-contratista.md)
- [Nuevo contratista](nuevo-contratista.md)
- [Problemas frecuentes](problemas-frecuentes.md)
