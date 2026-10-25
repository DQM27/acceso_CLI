# Historial de ingresos y exportar a Excel o PDF

## Para qué sirve

Consultar los ingresos y salidas pasados (quién entró, cuándo, quién lo
registró) y sacarlos a un archivo de **Excel**, **CSV** o **PDF**, por
ejemplo para un informe.

## Quién puede hacerlo

Cualquier usuario con sesión abierta.

## Dónde

**Historial**, en el menú lateral.

Arriba, en el centro, hay cuatro botones con íconos para elegir **qué
clase** de ingresos ver (al pasar el mouse se ve el nombre):

- **Contratistas**
- **Proveedores**
- **Visitas por correo**
- **Gafetes KOF** (entregas y devoluciones de gafetes provisionales)

Cada clase tiene su propia tabla y sus columnas. Las visitas agendadas en
la web no están aquí.

## Qué se ve en la tabla

**Contratistas**: Cédula, Nombre, Empresa, Dispositivo (en qué equipo se
registró: la PC o el celular del sitio), Tipo, Medio, Gafete, Fecha
ingreso, Hora ingreso, Fecha salida, Hora salida, Dio ingreso y Dio
salida. Si la persona sigue adentro, la salida dice **Activo**.

**Proveedores**: Cédula, Nombre, Empresa, Placa, Gafete, Fecha, Entrada,
Salida, Dio ingreso y Dio salida.

**Visitas por correo**: igual que proveedores, con **Motivo** en lugar de
Empresa.

**Gafetes KOF**: Encargado, Código empleado, Gafete, Fecha, Entrega,
Devolución, Entregó y Recibió.

## Pasos — elegir el período

Al abrir, el Historial muestra **los últimos 6 meses** (incluido el día de
hoy).

1. Pulse el botón del calendario (dice, por ejemplo, **6 MESES**). Está
   arriba a la derecha.
2. Elija un **acceso rápido**: Hoy, Ayer, Semana, Sem. pasada, Mes, Mes
   pasado, 7 días, 30 días, 6 meses o Todo. Se aplica al instante.
3. O escriba las fechas en **Desde** y **Hasta** y pulse **Aplicar**.
   **Cancelar** cierra sin cambiar nada.

El período es el mismo para las cuatro clases.

## Pasos — buscar y filtrar

- **Buscador** (arriba a la izquierda): escriba parte de la cédula, el
  nombre o la empresa (en Visitas por correo, el motivo; en KOF, el nombre
  o el código de empleado).
- **Filtro por columna**: el botón de filtros, en el grupo de botones de
  la tabla, muestra u oculta un campo debajo del título de cada columna.
- **Columnas visibles**: en ese mismo grupo, elija qué columnas mostrar.
- **Ordenar**: pulse el título de una columna.

Abajo, en la barra de estado, se ve cuántos movimientos hay ("N
movimiento(s)").

## Pasos — exportar

Los tres botones de exportar están arriba a la derecha, al lado del
calendario: **Excel**, **CSV** y **PDF** (al pasar el mouse se ve cuál es
cuál).

1. Deje la tabla como la quiere en el archivo: período, búsqueda, filtros,
   orden y columnas visibles. **El archivo sale con lo que se ve.**
2. Pulse **Excel**, **CSV** o **PDF**.
3. Elija la carpeta y el nombre del archivo, y guarde.
4. Verá **"Exportando…"** y luego **"N fila(s) exportadas."** (Excel) o
   **"PDF exportado."**

El PDF lleva debajo del título el período elegido (por ejemplo, "Filtro:
Últimos 6 meses").

## Períodos muy largos

Si el período tiene demasiados movimientos, la tabla de contratistas
muestra sólo los primeros y avisa: **"Este rango tiene más de N
movimientos — se muestran solo los primeros…"**. En ese caso:

- El buscador y los filtros sólo actúan sobre lo que se ve.
- **Excel y PDF** igual traen **todo** el período (sin el buscador ni los
  filtros por columna).
- **CSV** sólo trae lo que se ve.

Para trabajar con filtros sobre todo, elija un período más corto.

## Mensajes que pueden aparecer

| Mensaje | Qué significa | Qué hacer |
|---|---|---|
| "No hay filas para exportar con el filtro actual." | Con ese período, búsqueda y filtros, la tabla está vacía. | Amplíe el período o borre la búsqueda y los filtros. |
| "No hay columnas visibles para exportar." | Ocultó todas las columnas. | Vuelva a mostrar alguna en **Columnas visibles**. |
| "El CSV sólo incluye lo cargado. Para todo el rango use Excel o PDF." | El período es muy largo y el CSV sale incompleto. | Use Excel o PDF, o acorte el período. |
| "No se pudo guardar el archivo. Revise que la carpeta exista, que tenga permiso y que no esté abierto en otro programa" | El archivo no se pudo escribir. | Cierre el archivo si está abierto en Excel, o guárdelo en otra carpeta (por ejemplo, Documentos). |

## Relacionado

- [Ver quién está adentro (Activos)](ver-quien-esta-adentro.md)
- [Dar de alta y consultar gafetes, e historial de un gafete](gafetes-alta-y-consulta.md)
- [Auditoría de cambios](auditoria.md)
