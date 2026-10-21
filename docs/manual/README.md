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

- ⬜ Vincular el equipo por primera vez ("Vincular este equipo")
- ⬜ Iniciar sesión y fijar la contraseña la primera vez
- ⬜ Cambiar mi contraseña
- ⬜ Cerrar sesión (Ctrl+Q)
- ⬜ Cambiar el tema (claro u oscuro) y ordenar el menú lateral

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

- ⬜ Historial de ingresos y exportar a Excel o PDF
- ⬜ Auditoría de cambios

### Conexión y actualizaciones

- ⬜ Trabajar sin internet y sincronizar a mano
- ⬜ "Tiene un ingreso activo acá Y en otro sitio"
- ⬜ El equipo fue retirado o la sesión se cerró sola
- ⬜ Buscar e instalar una actualización

### Problemas frecuentes

- ✅ [PRAIND vencido, acceso denegado y otros bloqueos](escritorio/praind-y-bloqueos-de-acceso.md)
- ⬜ "El contratista ya tiene un ingreso activo"
- ⬜ "Revise la fecha y hora del equipo"

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
   ve como informativo (amarillo), pero el programa bloquea la entrega. O se
   vuelve un bloqueo claro, o se permite (decisión pendiente del dueño).
2. **Activos**: el botón **Salida** de cada fila registra sin confirmar. Un
   clic en la fila equivocada saca a la persona equivocada.
3. **Contratistas**: la columna **Acceso** se oculta al Operador, pero la
   casilla **Con acceso** sí aparece en el formulario de edición.
4. **Empresas**: cualquier usuario puede desactivar una empresa, lo que deja
   sin entrada a todos sus contratistas. Revisar si debe quedar sólo para
   administradores.
5. **Gafetes**: **Dar de baja** se aplica sin confirmar y es definitivo.
6. **Gafetes**: marcar perdido un gafete de **visita** busca la cédula entre
   las visitas agendadas en la web; con una visita **por correo** puede no
   encontrarla. Verificar.

## Índice — Móvil y panel web

Pendientes; se inventarían después de validar el formato con el escritorio.
