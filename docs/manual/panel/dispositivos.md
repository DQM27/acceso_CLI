# Dispositivos: registrar y retirar equipos

## Para qué sirve

Registrar los equipos del puesto de control (la PC del escritorio y el
teléfono) y darles el **código de vinculación**; y retirar un equipo
perdido, robado o reemplazado. Un equipo nunca se da de alta solo: **sólo
desde aquí**.

También se crean aquí las **unidades operativas** (los sitios).

## Quién puede hacerlo

Administradores del panel.

## Dónde

**Dispositivos**, en el menú lateral.

## Qué se ve

Etiqueta, Tipo (PC, Celular, Visor web), Unidad operativa, Creado,
Último uso, **Vinculación** (por ejemplo, "Código pendiente (8 min)"),
**Estado** (Activo / Retirado), **Conexión** (Conectado / Desconectado, en
vivo) y **Acción**.

## Pasos — registrar un equipo nuevo

1. Toque **+ Nuevo**. Se abre **Nuevo dispositivo**.
2. Elija la **Unidad operativa**. Si no existe, créela con el **+** (ver
   abajo).
3. Elija el **Tipo**: **PC**, **Celular** o **Visor web (solo lectura)**.
4. Escriba una **Etiqueta** que lo identifique, por ejemplo "Brisas - PC
   recepción".
5. Toque **Crear y generar código**.
6. Aparece el **Código de vinculación**, con un **QR**:
   - en el **teléfono**, escanee el QR desde **Vincular este teléfono**;
   - en la **PC**, escriba el código en **Vincular este equipo**.
7. Hágalo **antes de que venza** ("Vence en …"). El código sirve una sola
   vez y **no se vuelve a mostrar** al cerrar la ventana.

Si el código venció ("Este código ya venció: elimine este dispositivo y
regístrelo de nuevo."), toque **Eliminar** en esa fila y registre el
equipo otra vez.

Un equipo **reinstalado o reemplazado** se registra como uno nuevo (los
datos vuelven solos con la primera sincronización) y el viejo se retira.

## Pasos — crear una unidad operativa

1. En **Nuevo dispositivo**, toque el **+** junto a **Unidad operativa**.
2. Escriba el nombre (por ejemplo, "Brisas") y toque **Crear unidad
   operativa**.

## Pasos — retirar un equipo

Para un equipo perdido, robado o que ya no se usa.

1. Toque **Retirar** en su fila.
2. Confirme ("¿Retirar "(etiqueta)"? Ese equipo deja de sincronizar en el
   acto…") con **Sí, enviar código**.
3. Le llega un **código a su correo**. Escríbalo y toque **Confirmar**.
4. El equipo **deja de sincronizar en el acto**. La persona que lo use ve
   el aviso "Este equipo fue retirado en el panel…" y el botón para
   reconectarlo, que sólo funciona con un código de reconexión.

## Pasos — reconectar un equipo retirado

Para volver a usar un equipo que se retiró (por error, o para moverlo a
otra unidad). El equipo **no se borra ni se vuelve a registrar**: sigue
siendo el mismo.

1. En la fila del equipo **Retirado**, toque **Autorizar reconexión**.
2. Elija la **unidad operativa** (por defecto, la misma) y la
   **etiqueta** (por defecto, la que tenía), y toque **Generar código de
   reconexión**.
3. Confirme con el **código que le llega al correo**.
4. El panel muestra el **código de reconexión** una sola vez: sirve sólo
   para ese equipo y vence en 10 minutos. Désela a la persona que tiene
   el equipo; ella lo escribe en **Reconectar este equipo**.

- **Misma unidad**: el equipo conserva todos sus datos y envía lo que
  registró mientras estuvo retirado.
- **Otra unidad**: el equipo borra los datos de la unidad anterior y baja
  los de la nueva. Si tenía registros sin enviar, el equipo avisa cuántos
  se pierden y pide confirmar; queda anotado en **Intentos y alertas**.

Un código de reconexión no sirve para dar de alta un equipo nuevo, ni uno
de alta sirve para reconectar.

## Pasos — eliminar de la lista

**Eliminar** pide la misma confirmación por correo. Si el equipo ya tiene
historial, no se borra del todo: se oculta de la lista ("…ya tiene
historial y no se puede borrar del todo -- se ocultó de la lista.").

## Intentos y alertas

El botón **Intentos y alertas (N)** muestra los intentos rechazados de
vincular o usar un equipo (código usado o vencido, firma inválida…), con
cuándo, qué pasó, el dispositivo y la IP. También las reconexiones:
**Reconexión autorizada** (por quién) y **Equipo reconectado** (en la
misma u otra unidad, y si se descartaron registros sin enviar y quién lo
confirmó).

## Mensajes que pueden aparecer

| Mensaje | Qué significa | Qué hacer |
|---|---|---|
| "Elija una unidad operativa (o cree una con el botón +)." | Falta la unidad. | Elíjala o créela. |
| "Código copiado." / "No se pudo copiar -- seleccione el texto a mano." | Resultado de copiar el código. | Si no se copió, selecciónelo a mano. |

## Relacionado

- [Sesiones](sesiones.md)
- Escritorio: [Vincular el equipo](../escritorio/vincular-equipo.md)
- Móvil: [Vincular el teléfono](../movil/vincular-telefono.md)
