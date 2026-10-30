# Manual de uso

Manual para las personas que usan acceso_CLI día a día. Es la fuente de
verdad de la ayuda de uso (ver `docs/features-futuras/plan-asistente-ia.md`,
Parte B): el asistente sólo responde con lo que está escrito aquí, así que
**si algo no está aquí, el asistente no lo sabe**.

El manual también se lee dentro de las apps, sin internet: en el
escritorio (sección **Ayuda**, F1), en el panel web (**Ayuda**) y en el
móvil (menú de la cuenta → **Ayuda**). Las tres lo toman tal cual de esta
carpeta (el móvil, sólo `movil/`, metido en la app al compilar), y los
índices de abajo son los que muestran, en el mismo orden. Un proceso nuevo
se agrega a su índice; los enlaces entre procesos son relativos
(`otro-proceso.md#seccion`). Las pruebas de
`desktop/src/ayuda/manual.test.ts` y `mobile/rust-core/src/manual/tests.rs`
revisan que cada proceso esté en el índice y que ningún enlace quede roto.

## Cómo se escribe cada proceso

Un archivo por proceso, en la carpeta de su app (`escritorio/`, `movil/`,
`panel/`), con estas secciones:

1. **Para qué sirve**: una o dos líneas.
2. **Quién puede hacerlo**: rol.
3. **Dónde**: pantalla o atajo de teclado.
4. **Pasos**: numerados, cortos, con los nombres de botones tal como se
   ven en pantalla.
5. **Mensajes que pueden aparecer**: el texto **exacto** que ve el usuario,
   qué significa y qué hacer. Es lo que más se pregunta.
6. **Relacionado**: otros procesos.

Reglas de redacción:

- Lenguaje sencillo, trato de usted, sin términos técnicos (nada de
  "sincronización Realtime" ni "base de datos").
- Copiar los textos de la pantalla tal cual, entre comillas.
- Lo que dependa de una práctica del sitio y no del programa se marca con
  **⚠ Por confirmar** hasta que el dueño lo revise.
- Cuando cambie una pantalla o un mensaje, se actualiza su proceso en el
  mismo cambio.

## Índice — Escritorio (puesto de control)

Estado: ✅ redactado · ⬜ pendiente.

### Inicio y sesión

- ✅ [Vincular el equipo por primera vez ("Vincular este equipo")](escritorio/vincular-equipo.md)
- ✅ [Iniciar sesión y fijar la contraseña la primera vez](escritorio/iniciar-sesion.md)
- ✅ [Cambiar mi contraseña](escritorio/cambiar-contrasena.md)
- ✅ [Cerrar sesión (Ctrl+Q)](escritorio/cerrar-sesion.md)
- ✅ [Cambiar el tema (claro u oscuro) y ordenar el menú lateral](escritorio/tema-y-menu-lateral.md)
- ✅ [Buscar ayuda en el manual (F1)](escritorio/ayuda.md)

### Ingresos

- ✅ [Registrar el ingreso de un contratista (Ctrl+N)](escritorio/registrar-ingreso-contratista.md)
- ✅ [Registrar el ingreso de un proveedor (Alt+P)](escritorio/registrar-ingreso-proveedor.md)
- ✅ [Registrar una visita autorizada por correo (Alt+C)](escritorio/registrar-visita-por-correo.md)
- ✅ [Entregar un gafete provisional KOF (Alt+K)](escritorio/entregar-gafete-kof.md)

### Salidas

- ✅ [Registrar una salida (Ctrl+S)](escritorio/registrar-salida.md)

### Quién está adentro (Activos)

- ✅ [Ver y filtrar a las personas que están adentro](escritorio/ver-quien-esta-adentro.md)
- ✅ [Avisos de permanencia ("lleva más de 12 h adentro", "más de 14 h")](escritorio/avisos-de-permanencia.md)

### Contratistas y empresas

- ✅ [Buscar un contratista y entender su estado](escritorio/buscar-contratista.md)
- ✅ [Dar de alta o editar un contratista (datos, tipo, fecha de PRAIND, personal de ruta)](escritorio/contratista-alta-y-edicion.md)
- ✅ [Empresas de contratistas y empresas proveedoras](escritorio/empresas.md)

### Gafetes

- ✅ [Dar de alta y consultar gafetes, e historial de un gafete](escritorio/gafetes-alta-y-consulta.md)
- ✅ [Gafete perdido, pagado o aparecido, y "Este contratista debe el gafete…"](escritorio/gafete-perdido.md)

### Consultas

- ✅ [Historial de ingresos y exportar a Excel o PDF](escritorio/historial-y-exportar.md)
- ✅ [Auditoría de cambios](escritorio/auditoria.md)

### Conexión y actualizaciones

- ✅ [Trabajar sin internet y sincronizar a mano](escritorio/trabajar-sin-internet.md)
- ✅ ["Tiene un ingreso activo aquí Y en otro sitio"](escritorio/ingreso-activo-en-otro-sitio.md)
- ✅ [El equipo fue retirado o la sesión se cerró sola](escritorio/equipo-retirado-o-sesion-cerrada.md)
- ✅ [Buscar e instalar una actualización](escritorio/buscar-actualizacion.md)

### Problemas frecuentes

- ✅ [PRAIND vencido, acceso denegado y otros bloqueos](escritorio/praind-y-bloqueos-de-acceso.md)
- ✅ ["El contratista ya tiene un ingreso activo"](escritorio/contratista-ya-tiene-ingreso-activo.md)
- ✅ ["Revise la fecha y hora del equipo"](escritorio/fecha-y-hora-del-equipo.md)

### Fuera del manual

- **Visitas** (las agendadas en la web) y **Rutas**: no se documentan
  (decisión del dueño, 2026-10-21). La visita **por correo** (Alt+C) sí,
  porque se registra desde Activos.
- **Catálogo KOF**: oculto en el escritorio; se documenta si se vuelve a
  mostrar.

## Mejoras de pantalla detectadas al documentar

Cosas que el manual explica pero que conviene corregir en el programa.
Cuando se corrija una, se actualiza su proceso.

1. **Gafete KOF**: el aviso "Ya tiene prestado el gafete… sin devolver" se
   ve como informativo (amarillo), pero el programa bloquea la entrega.
   Decisión del dueño (2026-10-23): **es un bloqueo** (sin devolver no se
   asigna otro), así que el aviso debe verse como bloqueo (rojo).
2. **Activos**: el botón **Salida** de cada fila registra sin confirmar. Un
   clic en la fila equivocada saca a la persona equivocada.
3. ~~**Contratistas** y **Empresas**: se podían activar o desactivar desde
   el escritorio.~~ Corregido (2026-10-24): decisión del dueño, **nadie**
   activa ni desactiva empresas o contratistas desde el escritorio; es sólo
   del panel web. El escritorio ya no lo ofrece y el núcleo conserva el
   acceso del contratista al guardar.
4. **Empresas en el panel**: el panel web todavía no tiene sección
   **Empresas**, así que activar o desactivar una empresa hoy se hace por
   SQL en Supabase (decisión del dueño 2026-10-24, hasta que exista).
5. **Gafetes**: **Dar de baja** se aplica sin confirmar y es definitivo.
6. **Gafetes**: marcar perdido un gafete de **visita** busca la cédula entre
   las visitas agendadas en la web; con una visita **por correo** puede no
   encontrarla. Verificar.
7. **Cerrar sesión**: Ctrl+Q y el botón cierran la sesión sin confirmar.
   Un Ctrl+Q por error saca a la persona en pleno registro.
8. ~~**Equipo retirado**: no había forma de volver a usar un equipo
   retirado sin borrarle la base.~~ Resuelto (2026-10-24): el panel
   autoriza la reconexión con un código propio de ese equipo; en la misma
   unidad conserva todo, en otra borra su base y baja la nueva (ver
   `docs/features-futuras/plan-reconexion-equipo-retirado.md`).
9. ~~**Mensajes**: varios textos mezclaban "usted" con voseo y tuteo.~~
   Corregido (2026-10-24): todos los mensajes de las tres apps tratan de
   "usted" y dicen "aquí" en vez de "acá"; el de archivo no guardado ya no
   trae espacios de más.
10. **Auditoría**: muestra sólo los cambios hechos desde este equipo; no
    los del otro equipo del sitio ni los del panel. Para reunirlos hace
    falta una tabla en Supabase (para después, dueño 2026-10-23).
11. **Conflictos** (ingreso aquí y en otro sitio, gafete en conflicto): hoy
    sólo avisan, sin forma de resolverlos desde la pantalla. En
    construcción.

## Índice — Móvil (teléfono del puesto de control)

App Android **Lattis**. Sólo registros rápidos: el alta de empresas,
la edición de contratistas, el Historial y la Auditoría se hacen en el
escritorio.

### Inicio y sesión

- ✅ [Vincular el teléfono por primera vez (QR o código)](movil/vincular-telefono.md)
- ✅ [Iniciar sesión y fijar la contraseña la primera vez](movil/iniciar-sesion.md)
- ✅ [Menú de la cuenta: tema y cerrar sesión](movil/cuenta-tema-y-cerrar-sesion.md)
- ✅ [Buscar ayuda en el manual](movil/ayuda.md)
- ✅ [La pantalla principal: secciones, botón redondo y avisos](movil/pantalla-principal.md)

### Contratistas (Activos)

- ✅ [Registrar el ingreso de un contratista (escaneando o buscando)](movil/registrar-ingreso-contratista.md)
- ✅ [Registrar la salida de un contratista (lista, nombre o gafetes)](movil/registrar-salida-contratista.md)
- ✅ [Nuevo contratista (con el carnet PRAIND)](movil/nuevo-contratista.md)

### Externos y KOF

- ✅ [Ingreso y salida de un proveedor](movil/ingreso-y-salida-proveedor.md)
- ✅ [Ingreso y salida de una visita por correo](movil/ingreso-y-salida-por-correo.md)
- ✅ [Entregar y recibir un gafete provisional KOF](movil/gafete-kof.md)

### Avisos, conexión y actualizaciones

- ✅ [Avisos de la administración (la campanita)](movil/avisos-de-la-administracion.md)
- ✅ [Trabajar sin internet y sincronizar](movil/sin-internet-y-sincronizar.md)
- ✅ [El teléfono fue retirado o la sesión se cerró sola](movil/telefono-retirado-o-sesion-cerrada.md)
- ✅ [Buscar e instalar una actualización](movil/buscar-actualizacion.md)

### Problemas frecuentes

- ✅ [Cámara, permisos, sonido, ingreso activo, conflictos y hora](movil/problemas-frecuentes.md)

### Fuera del manual

- **Rutas** y **Visita** (agendadas en la web): ocultas en la app; se
  documentan cuando se vuelvan a mostrar.

### Mejoras de pantalla detectadas al documentar el móvil

1. ~~**Mensajes**: mezclaban "usted" con voseo y tuteo.~~ Corregido
   (2026-10-24), igual que en el escritorio.
2. **Salida por gafete escaneado**: se registra al instante, sin
   confirmar (decisión de diseño). Un gafete mal leído saca a otra
   persona; el mensaje verde/rojo es la única señal.

## Índice — Panel web (administradores)

**En desarrollo**: estos procesos pueden cambiar. El panel se usa en el
navegador, con una cuenta de Google autorizada.

- ✅ [Entrar al panel y moverse por él](panel/entrar-al-panel.md)
- ✅ [Activos (todas las unidades)](panel/activos.md)
- ✅ [Historial de ingresos y exportar](panel/historial.md)
- ✅ [Contratistas: alta, edición y acceso](panel/contratistas.md)
- ✅ [Usuarios del puesto de control (alta, contraseña temporal, baja)](panel/usuarios.md)
- ✅ [Sesiones (bitácora de inicios de sesión)](panel/sesiones.md)
- ✅ [Dispositivos: registrar y retirar equipos, unidades operativas](panel/dispositivos.md)
- ✅ [Enviar avisos a los teléfonos](panel/avisos.md)
- ✅ [Asistente (en desarrollo)](panel/asistente.md)
- ✅ [Ayuda: el manual de las tres apps](panel/ayuda.md)

### Mejoras de pantalla detectadas al documentar el panel

1. **Usuarios**: **Resetear contraseña** se aplica al instante, sin
   confirmar. Un clic en la fila equivocada le cambia la contraseña a otra
   persona.
2. ~~**Mensajes**: algunos usaban voseo.~~ Corregido (2026-10-24).
