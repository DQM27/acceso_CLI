# Ver el carné PRAIND de un contratista

## Para qué sirve

Ver el carné de inducción (PRAIND) de una persona, con su foto, para
confirmar en la portería que es quien dice ser. Si el equipo tiene permiso,
también se puede cargar o reemplazar el carné.

## Quién puede hacerlo

- **Ver**: cualquier usuario con sesión abierta en un equipo vinculado.
- **Cargar o reemplazar**: sólo si un administrador lo permitió desde el
  panel web. Si no aparece el botón **Cargar carné**, el equipo no tiene
  permiso: el carné lo carga un administrador en el panel.

## Dónde

**Contratistas**, en el menú lateral: **doble clic** en la fila de la
persona abre **Detalles del contratista**, con sus datos a la izquierda y el
carné a la derecha.

## Pasos — ver el carné

1. Busque a la persona y haga **doble clic** en su fila.
2. El carné aparece a la derecha. Haga clic sobre él para verlo en grande.
3. Para cambiar los datos de la persona, pulse **Editar**.

La primera vez que se abre un carné en este equipo hace falta internet:
se baja de la nube y **queda guardado en el equipo**. Desde entonces se ve
al instante, aunque no haya internet. El equipo no baja todos los carnés:
sólo guarda los que alguien abrió.

## Pasos — cargar o reemplazar el carné

1. En **Detalles del contratista**, pulse **Cargar carné** (o **Reemplazar
   carné** si ya tiene uno).
2. Escanee el QR del carné con el lector y pegue el enlace en **Enlace del
   QR del carné** (empieza con `https://prd.pw/`).
3. Pulse **Buscar carné**. Aparece de quién es el carné:
   - En verde, si el nombre coincide con el del contratista: pulse
     **Guardar carné**.
   - En amarillo, si **no** coincide: revise que sea el carné correcto de
     esta persona antes de pulsar **Guardar igual**.
4. Si tiene el carné como imagen (PNG o JPEG), también puede pulsar
   **Elegir imagen…** y seleccionar el archivo.

## Mensajes que pueden aparecer

| Mensaje | Qué significa | Qué hacer |
|---|---|---|
| "Este contratista no tiene carné PRAIND cargado." | Nadie cargó su carné todavía. | Si el equipo tiene permiso, cárguelo; si no, avise a un administrador. |
| "Para ver este carné por primera vez, conectate a internet…" | El carné nunca se abrió en este equipo y ahora no hay internet. | Vuelva a abrirlo cuando haya conexión. Después queda guardado. |
| "El enlace no es de un carné PRAIND." | Lo pegado no es el enlace del QR del carné. | Escanee de nuevo el QR del carné. |
| "La página del QR no tiene ese carné." | El enlace ya no lleva a un carné. | Pida el carné vigente a la persona. |
| "Este contratista todavía no llegó a la nube…" | El contratista se creó hace poco y no se sincronizó. | Espere un par de minutos y vuelva a intentar. |
| "No tiene permiso para cargar carnés PRAIND." | Un administrador quitó el permiso a los equipos. | El carné lo carga un administrador en el panel. |

## Relacionado

- [Dar de alta o editar un contratista](contratista-alta-y-edicion.md)
- [PRAIND vencido, acceso denegado y otros bloqueos](praind-y-bloqueos-de-acceso.md)
- [Trabajar sin internet y sincronizar a mano](trabajar-sin-internet.md)
