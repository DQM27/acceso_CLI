# Nuevo contratista

## Para qué sirve

Dar de alta a un contratista que todavía no está registrado, para poder
registrar su ingreso. En el teléfono sólo se da de alta: para corregir
datos de alguien que ya existe se usa el escritorio.

## Quién puede hacerlo

Cualquier usuario con sesión abierta.

## Dónde

- Lo más común: al escanear o buscar a alguien en **Contratistas** que no
  aparece, toque **Registrar nuevo contratista**. Arranca con la cédula o
  el nombre que buscó.
- Si ya sabe que es alguien nuevo: toque el cuadrito con su inicial (arriba
  a la derecha) y luego **Nuevo contratista**. Conviene buscarlo antes: si
  ya estaba registrado, darlo de alta otra vez lo duplica.

## Pasos — con el QR del gafete PRAIND (datos y gafete en un paso)

Hace falta internet y que un administrador le haya dado permiso a este
teléfono desde el panel para cargar gafetes PRAIND. Sin eso, la opción no
aparece: use la cámara (más abajo).

1. En la cámara que se abre, toque **Leer el QR del gafete** (o, en el
   formulario, **Lector QR**).
2. Apunte al **código QR** del gafete PRAIND. Se lee solo.
3. La app baja el gafete, lo lee y llena cédula, nombre, empresa y
   vencimiento. Dice **"Gafete PRAIND leído: revise los datos. Al guardar se
   carga el gafete."**
   - Si el gafete está **vencido**, lo dice con la fecha y **Guardar** queda
     deshabilitado: pida a la persona el gafete actualizado.
   - Si falta algún dato (no se pudo leer), dice **"complete lo que falta"**.
4. **Revise todo** y toque **Guardar**. Se registra al contratista **y
   queda cargado su gafete PRAIND**: en el formulario de ingreso ya aparece
   **Ver gafete PRAIND**, y arriba se ve **"Gafete PRAIND de … guardado."**

**Si la persona ya está registrada** (por ejemplo, alguien que no viene
hace tiempo y tiene la ficha vieja), en vez de **Guardar** aparece
**Actualizar su ficha con este gafete**: abre su ingreso con la revisión del
gafete, que actualiza la fecha (y, si lo marca, el nombre o la empresa).
Así no se intenta darla de alta otra vez.

Si cambia la cédula por otra, el gafete no se carga (sería de otra
persona). Si la carga del gafete falla (por ejemplo, se cortó internet), el
contratista queda registrado igual y se avisa: cárguelo desde su ingreso con
**Cargar gafete PRAIND**.

## Pasos — escaneando el gafete PRAIND con la cámara

1. Se abre directo la cámara: apunte al gafete PRAIND.
2. La app llena cédula, nombre, empresa y vencimiento, y dice **"Gafete
   PRAIND leído — revise los datos antes de guardar"**.
3. **Revise todo** contra el gafete PRAIND. Si la empresa del gafete PRAIND no coincide
   con ninguna de la lista, verá debajo de la empresa: "El gafete PRAIND dice
   «(empresa)»: no hay una empresa igual, elija la correcta."
4. Toque **Guardar**.
5. Pasa directo al formulario de ingreso de esa persona: ver
   [Registrar el ingreso de un contratista](registrar-ingreso-contratista.md#el-formulario-de-ingreso).

En la cámara, **Llenar a mano** lleva al formulario vacío y la **X**
cancela sin registrar a nadie.

## Pasos — escribiendo los datos

1. En la cámara, toque **Llenar a mano**.
2. **Persona**: **Cédula** y **Nombre**.
3. **Acceso**:
   - **Empresa**: toque **Elegir** y búsquela abajo. Si no está, hay que
     crearla en el panel.
   - **Tipo de ingreso**: **PRAIND**, **IN HOUSE** o **SWAT**.
   - **Personal de ruta**: actívelo si corresponde (sólo aparece para
     PRAIND e IN HOUSE).
4. **PRAIND** → **Vence**: toque **Elegir fecha** y elija el vencimiento en
   el calendario, cuando el tipo lo pide.
5. Toque **Guardar**. Pasa directo a su ingreso.

**Escanear gafete PRAIND** (o **Lector OCR**, al lado de
**Lector QR** cuando está la opción del QR), arriba del formulario, vuelve a abrir la
cámara (borra lo escrito). **‹ Volver** cancela sin guardar.

## Mensajes que pueden aparecer

| Mensaje | Qué significa | Qué hacer |
|---|---|---|
| "PRAIND vencido: elija una fecha vigente." | La fecha elegida ya pasó. **Guardar** queda deshabilitado. | Elija la fecha de un gafete PRAIND vigente. Si la persona no tiene uno vigente, no puede quedar registrada. |
| "Elija la empresa" | No eligió empresa. | Toque **Elegir** en **Empresa**. |
| "Documento inválido" | Lo escaneado no es un gafete PRAIND que la app pueda leer. | Vuelva a escanear con mejor luz, o escriba los datos. |
| "La cédula es obligatoria" / "El nombre es obligatorio" | Falta ese dato. | Complételo. |
| "La cédula debe tener sólo números, entre 9 y 13 dígitos" | La cédula no tiene el formato correcto. | Revísela. |
| "Ya existe un contratista con esa cédula" | La persona ya está registrada. | Búsquela en **Contratistas** y registre su ingreso. Para corregir sus datos, use el escritorio. |
| "Fecha PRAIND requerida" | El tipo elegido necesita fecha de PRAIND. | Elija la fecha del gafete PRAIND. |
| "El gafete PRAIND venció el …: pida a la persona el gafete actualizado." | El gafete del QR está vencido. | No puede quedar registrada hasta tener un gafete vigente. |
| "Contratista guardado, pero el gafete PRAIND no se cargó…" | El alta quedó, pero el gafete no subió. | En su ingreso, toque **Cargar gafete PRAIND**. |

## Relacionado

- [Registrar el ingreso de un contratista](registrar-ingreso-contratista.md)
