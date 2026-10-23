# Manual de uso

Manual para las personas que usan acceso_CLI día a día. Es la fuente de
verdad de la ayuda de uso (ver `docs/features-futuras/plan-asistente-ia.md`,
Parte B): el asistente sólo responde con lo que está escrito aquí, así que
**si algo no está aquí, el asistente no lo sabe**.

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
- ✅ ["Tiene un ingreso activo acá Y en otro sitio"](escritorio/ingreso-activo-en-otro-sitio.md)
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
3. **Contratistas**: la columna **Acceso** se oculta al Operador, pero la
   casilla **Con acceso** sí aparece en el formulario de edición y la
   puede cambiar.
4. **Empresas**: cualquier usuario puede desactivar una empresa, lo que deja
   sin entrada a todos sus contratistas.

   Decisión del dueño (2026-10-23) para 3 y 4: **sólo un administrador**
   activa o desactiva empresas y contratistas. Hoy el programa no lo
   impide (los roles están aplanados en `src/domain/autorizacion.rs`); el
   manual ya lo indica como regla.
5. **Gafetes**: **Dar de baja** se aplica sin confirmar y es definitivo.
6. **Gafetes**: marcar perdido un gafete de **visita** busca la cédula entre
   las visitas agendadas en la web; con una visita **por correo** puede no
   encontrarla. Verificar.
7. **Cerrar sesión**: Ctrl+Q y el botón cierran la sesión sin confirmar.
   Un Ctrl+Q por error saca a la persona en pleno registro.
8. **Equipo retirado**: el aviso dice "registrarlo como dispositivo nuevo",
   pero la pantalla **Vincular este equipo** sólo aparece en un equipo sin
   datos. Falta un camino claro para volver a vincular un equipo usado
   (por definir, dueño 2026-10-23).
9. **Mensajes**: varios textos mezclan "usted" con voseo ("pedí",
   "intentá", "Revisá") y otros usan "tú" ("Ya tienes la última versión").
   Unificar en "usted", como el resto de la app. El de archivo no guardado
   ("No se pudo guardar el archivo…") trae espacios de más en el código.
10. **Auditoría**: muestra sólo los cambios hechos desde este equipo; no
    los del otro equipo del sitio ni los del panel. Para reunirlos hace
    falta una tabla en Supabase (para después, dueño 2026-10-23).
11. **Conflictos** (ingreso acá y en otro sitio, gafete en conflicto): hoy
    sólo avisan, sin forma de resolverlos desde la pantalla. En
    construcción.

## Índice — Móvil y panel web

Pendientes; se inventarían después de validar el formato con el escritorio.
