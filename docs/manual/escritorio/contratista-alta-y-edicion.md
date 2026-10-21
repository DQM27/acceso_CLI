# Dar de alta o editar un contratista

## Para qué sirve

Registrar a un contratista nuevo para que pueda entrar, o corregir sus
datos: nombre, cédula, empresa, tipo, fecha del PRAIND o si es personal de
ruta.

## Quién puede hacerlo

Cualquier usuario con sesión abierta en el escritorio.

## Dónde

**Contratistas**, en el menú lateral.

- Contratista nuevo: botón **+** (Nuevo contratista), junto al buscador.
- Editar: **doble clic** en la fila de la persona.

## Pasos — contratista nuevo

1. Pulse **+**. Se abre **Nuevo contratista**.
2. **Cédula**: sólo números (de 9 a 13 dígitos). Los guiones y espacios se
   quitan solos.
3. **Nombre**: sólo letras; se escribe en mayúscula solo.
4. **Empresa**: elíjala de la lista. Si no está, primero hay que crearla
   (ver [Empresas](empresas.md)).
5. **Tipo de ingreso**: PRAIND, IN_HOUSE o SWAT.
6. **Personal de ruta**: márquelo si corresponde. Sólo aparece para PRAIND
   e IN_HOUSE.
7. **Fecha de vencimiento PRAIND**: aparece cuando el tipo lo necesita
   (PRAIND, IN_HOUSE o personal de ruta). Escriba la fecha del carnet.
8. Pulse **Guardar**.

El contratista nuevo queda **con acceso**: ya puede registrar su ingreso
(Ctrl+N).

## Pasos — editar

1. Busque a la persona y haga **doble clic** en su fila. Se abre **Editar
   contratista**.
2. Cambie lo que haga falta.
3. Pulse **Guardar**.

El caso más común es **renovar el PRAIND**: cambie **Fecha de vencimiento
PRAIND** por la fecha del carnet nuevo. Lo puede hacer el operador.

**Personal de ruta** también se puede cambiar directo en la tabla, con el
interruptor de su columna, sin abrir el formulario.

⚠ En el formulario de edición aparece la casilla **Con acceso**. No la
cambie: quitar o devolver el acceso a una persona lo decide un
administrador.

## Mensajes que pueden aparecer

| Mensaje | Qué significa | Qué hacer |
|---|---|---|
| "La cédula es obligatoria" / "El nombre es obligatorio" | Falta ese dato. | Complételo. |
| "La cédula debe tener sólo números, entre 9 y 13 dígitos" | La cédula no tiene el formato correcto. | Revísela. El pasaporte no se acepta para contratistas. |
| "El nombre no puede tener números ni símbolos" | El nombre tiene caracteres no permitidos. | Escriba sólo letras. |
| "Elija la empresa" | No eligió empresa. | Elíjala de la lista. |
| "Ya existe un contratista con esa cédula" | La persona ya está registrada. | Búsquela en la tabla y edítela en lugar de crearla de nuevo. |
| "Fecha PRAIND requerida" | El tipo elegido necesita PRAIND y no hay fecha. | Escriba la fecha de vencimiento del carnet. |
| "El PRAIND está vencido — ingrese una fecha vigente" (y **Guardar** no se puede pulsar) | La fecha escrita ya pasó. | Escriba la fecha de un carnet vigente. Si la persona no tiene uno vigente, no puede quedar registrada como habilitada. |
| "Personal de ruta sólo aplica a PRAIND e IN HOUSE" | Se marcó personal de ruta con otro tipo. | Cambie el tipo o desmarque personal de ruta. |
| "No se puede cambiar la cédula mientras está adentro — registre primero la salida" | Intentó corregir la cédula de alguien que está adentro. | Registre su salida (Ctrl+S), corrija la cédula y vuelva a registrar su ingreso. |
| "«Por correo» ya no es un tipo de contratista: registre la visita en «Por correo»" | Ese tipo ya no se usa. | Registre a la persona como [visita por correo](registrar-visita-por-correo.md). |
| Empresa con "(inactiva)" en la lista | La empresa actual de la persona está desactivada. | La persona no podrá entrar mientras siga así. Avise a un administrador. |

## Relacionado

- [Buscar un contratista y entender su estado](buscar-contratista.md)
- [PRAIND vencido, acceso denegado y otros bloqueos](praind-y-bloqueos-de-acceso.md)
- [Empresas](empresas.md)
