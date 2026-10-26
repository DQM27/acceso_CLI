# Historial de ingresos (todas las unidades)

## Para qué sirve

Consultar los ingresos y salidas de **todas las unidades** —
contratistas, proveedores, visitas por correo y gafetes provisionales KOF —
y exportarlos a Excel, CSV o PDF.

## Quién puede hacerlo

Administradores del panel. Es sólo de consulta.

## Dónde

**Historial**, en el menú lateral. Es la primera sección.

## Las clases

Junto al buscador hay un selector con cuatro clases, como en el
escritorio: **Contratistas**, **Proveedores**, **Visitas por correo** y
**Gafetes KOF**. Cada clase tiene su propia lista, sus columnas y su
exportación. Al cambiar de clase y volver, la lista queda como la dejó
(página, orden y filtros).

El **período** y las **unidades** son los mismos para las cuatro clases.

## Qué se ve

| Clase | Columnas |
|---|---|
| Contratistas | Unidad operativa, Cédula, Nombre, Empresa, Dispositivo (PC o celular), Tipo, Medio, Gafete, Fecha y hora de ingreso, Fecha y hora de salida (**Activo** si sigue adentro), Dio ingreso y Dio salida. |
| Proveedores | Unidad operativa, Cédula, Nombre, Empresa, Dispositivo, Placa, Gafete, Fecha y hora de ingreso y de salida (**Activo** si sigue adentro), Dio ingreso y Dio salida. |
| Visitas por correo | Las mismas que proveedores, con **Motivo** en lugar de Empresa. |
| Gafetes KOF | Unidad operativa, Código empleado, Encargado, Dispositivo, Gafete, Fecha y hora de **Entrega**, Fecha y hora de **Devolución** (**Prestado** si no lo devolvió), **Entregó** y **Recibió**. |

Se muestran de a **100 filas por página**.

## Pasos — filtrar

1. **Clase**: elija una en el selector.
2. **Unidades**: el botón "Todas las unidades ▾" deja marcar qué unidades
   ver.
3. **Período**: el botón del calendario (accesos rápidos o **Desde** /
   **Hasta**).
4. **Buscador** ("Cédula o nombre…"; en KOF, "Código empleado o nombre…")
   y **filtros por columna**. Para filtrar por fecha use las columnas de
   fecha (vacía = sigue adentro o el gafete sigue prestado).
5. Ordene tocando el título de una columna.

## Pasos — exportar

1. Deje la clase y el filtro como los quiere en el archivo.
2. Toque **Excel**, **CSV** o **PDF** (arriba a la derecha). Se exporta
   **todo el resultado del filtro** de esa clase, no sólo la página que ve,
   con las columnas visibles.
3. El PDF abre la ventana de impresión: elija **Guardar como PDF**.

## Mensajes que pueden aparecer

| Mensaje | Qué significa | Qué hacer |
|---|---|---|
| "No hay filas para exportar con el filtro actual." | El filtro no trae nada. | Amplíe el período o las unidades. |
| "No hay columnas visibles para exportar." | Ocultó todas las columnas. | Muestre alguna en **Columnas visibles**. |
| "Se exportan las primeras N filas; el filtro tiene más. Acote las fechas o la unidad para exportar el resto." | El resultado es demasiado grande para un solo archivo. | Exporte por partes (por mes o por unidad). |
| "No se pudo preparar el PDF." | El navegador bloqueó la ventana de impresión. | Permita las ventanas emergentes para el panel y vuelva a intentar. |

## Relacionado

- [Activos](activos.md)
- [Sesiones](sesiones.md)
