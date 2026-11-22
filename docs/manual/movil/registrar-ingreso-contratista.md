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

1. En **Contratistas**, toque el botón redondo azul **+**, abajo a la
   derecha: se abre la cámara.
2. Coloque la cédula (o el gafete PRAIND) **dentro del recuadro**.
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
  (**"Acceso denegado"**).
- **Roja, VENCIDO** (calendario con una X): su PRAIND está vencido
  (**"PRAIND vencido"**).

## El formulario de ingreso

Arriba se ve el nombre y "cédula · empresa". Si la persona tiene su gafete
PRAIND cargado, a la derecha aparece **Ver gafete PRAIND**: lo muestra a pantalla
completa para comparar la foto con quien está en la portería (**Cerrar**
vuelve al formulario). La primera vez que se ve un gafete PRAIND en este teléfono
hace falta internet; después queda guardado y se ve aunque no haya señal.
Si no hay internet y nunca se vio, aparece **"Para ver este gafete PRAIND por
primera vez, conéctese a internet…"**.

### Cargar el gafete PRAIND con su QR

Si la persona **no tiene** el gafete PRAIND cargado y un administrador le dio
permiso a este teléfono desde el panel, en lugar de **Ver gafete PRAIND**
aparece **Cargar gafete PRAIND**. Hace falta internet.

1. Toque **Cargar gafete PRAIND**. Se abre la cámara.
2. Apunte al **código QR del gafete PRAIND** que trae la persona. Se lee solo,
   sin escribir nada.
3. El teléfono lee el gafete y lo compara con la ficha del contratista. Aparece
   una línea por dato, en verde si está bien, en amarillo si hay que revisarlo
   y en rojo si hay un problema:
   - **Cédula**: si no coincide, **no se puede guardar**: puede ser el gafete
     de otra persona.
   - **Nombre** y **empresa**: si no coinciden, señala la palabra que difiere
     (por ejemplo, **«JOHANNA» en el gafete, «JOHANA» en la ficha**). Igual se
     puede guardar. Debajo aparece **Corregir el nombre de la ficha** (o la
     empresa): márquelo si la ficha está mal escrita y debe quedar como dice
     el gafete. La empresa sólo se ofrece si existe en el catálogo.
   - **Vencimiento**:
     - Si el gafete es **más nuevo** que la ficha, al guardar se actualiza la
       fecha de la ficha.
     - Si el gafete está **vencido**, se guarda sólo como imagen ilustrativa:
       pida a la persona el gafete actualizado. El botón dice **Guardar como
       imagen**.
     - Si es **más viejo** que la ficha, la ficha no cambia.
   - **Escanear otra vez** vuelve a la cámara; **Cancelar** no guarda nada.
4. Al guardar aparece **"Gafete PRAIND de … guardado."** (y la fecha nueva, si
   se actualizó) y el botón pasa a **Ver gafete PRAIND**. El escritorio y el
   panel lo ven también.

Para cambiar uno que ya está cargado: **Ver gafete PRAIND** → **Reemplazar
gafete PRAIND**, y los mismos pasos.

| Mensaje | Qué significa | Qué hacer |
|---|---|---|
| "Ese gafete PRAIND ya no existe en el sistema PRAIND: pudo haberse borrado…" | La página del QR ya no tiene el gafete PRAIND. | Pida a la persona su gafete PRAIND actualizado. |
| "El QR no lleva a ningún gafete PRAIND…" | El enlace del QR no abre ningún gafete PRAIND. | Igual que el anterior. Si pasa con todos los gafetes, el sistema PRAIND puede estar fuera de servicio: intente más tarde. |
| "El enlace no es de un gafete PRAIND." | El QR leído no es de un gafete PRAIND. | Apunte al QR del gafete PRAIND. |
| No aparece **Cargar gafete PRAIND** | Este teléfono no tiene permiso, o no hay internet. | Que un administrador lo cargue desde el panel, o lo permita para este teléfono. |

Luego:

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
| "Gafete asignado a otra persona" | Ese gafete lo tiene otra persona adentro. | Entregue otro gafete. |
| "Gafete N asignado en el otro equipo" | Lo entregó la PC del puesto. | Entregue otro gafete. |
| "Ese gafete no existe en el catálogo" | Ese número no está registrado. | Revise el número. |
| "Gafete perdido: no se puede asignar" | Ese gafete no se puede usar. | Entregue otro gafete. |
| "La placa es obligatoria cuando el ingreso es en vehículo" | Falta la placa. | Escríbala o escanéela. |
| "Revise la fecha y hora del equipo antes de continuar" | La hora del teléfono quedó antes del último registro. | Ver [Problemas frecuentes](problemas-frecuentes.md#revise-la-fecha-y-hora-del-equipo). |

## Si la persona no aparece

Si escanea o busca y no aparece, dice **"No está registrado"** con la
opción **Registrar nuevo contratista**. Tóquela: se abre la cámara del
gafete PRAIND con lo que buscó ya puesto y, al guardar, pasa directo a su
ingreso. Ver [Nuevo contratista](nuevo-contratista.md).

## Relacionado

- [Registrar la salida de un contratista](registrar-salida-contratista.md)
- [Nuevo contratista](nuevo-contratista.md)
- [Problemas frecuentes](problemas-frecuentes.md)
