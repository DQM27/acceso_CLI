# Auditoría de cambios

## Para qué sirve

Ver quién cambió qué y cuándo en los datos de contratistas, empresas,
usuarios y gafetes. Sirve para aclarar dudas como "¿quién le quitó el
acceso a este contratista?" o "¿cuándo se marcó perdido este gafete?".

## Quién puede hacerlo

Cualquier usuario con sesión abierta. Es sólo de consulta: no se puede
borrar ni cambiar nada desde aquí.

## Dónde

**Auditoría**, en el menú lateral.

## Qué se ve en la tabla

Un cambio por fila, los más recientes primero:

| Columna | Qué muestra |
|---|---|
| Fecha y Hora | Cuándo se hizo el cambio |
| Entidad | A quién o qué se le cambió (el nombre actual del contratista o la empresa, o "Gafete 07") |
| Tipo | Contratista, Empresa, Usuario o Gafete |
| Campo | Qué dato cambió: Cédula, Nombre, Empresa, Tipo de ingreso, Vencimiento PRAIND, Personal de ruta, Acceso, Rol, Activo, Contraseña o Estado (gafetes) |
| Valor anterior / Valor nuevo | El dato antes y después del cambio |
| Modificado por | Qué usuario hizo el cambio |

Algunos casos:

- **Contraseña**: sólo dice que se cambió y cuándo; nunca muestra la
  contraseña (los valores aparecen como "—").
- **Gafetes**: "Perdido — asignado a (nombre)" cuando se marcó perdido;
  "Disponible — pagado" o "Disponible — apareció" cuando se resolvió.
- **Vencimiento PRAIND** vacío aparece como "Sin fecha".

## Pasos

1. Abra **Auditoría** en el menú lateral.
2. Escriba en el buscador ("Entidad, campo, valor…") el nombre, la cédula,
   el número de gafete o el dato que busca.
3. Para afinar, use los filtros por columna (por ejemplo, **Campo** =
   Acceso) u ordene pulsando el título de una columna.

Abajo, en la barra de estado, se ve cuántos cambios hay ("N cambio(s)
auditado(s)").

**⚠ Por confirmar**: la Auditoría muestra los cambios hechos **desde este
equipo**. Los cambios hechos en el otro equipo del sitio o en el panel web
no aparecen aquí.

## Mensajes que pueden aparecer

| Mensaje | Qué significa | Qué hacer |
|---|---|---|
| "Hay más de N cambios de auditoría — se muestran solo los primeros…" | El registro es muy grande y se cargó sólo una parte. | El buscador sólo busca en lo cargado. Si no encuentra algo antiguo, pida ayuda a un administrador. |

## Relacionado

- [Dar de alta o editar un contratista](contratista-alta-y-edicion.md)
- [Empresas de contratistas y empresas proveedoras](empresas.md)
- [Gafete perdido, pagado o aparecido](gafete-perdido.md)
- [Historial de ingresos](historial-y-exportar.md)
