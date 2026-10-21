# Buscar un contratista y entender su estado

## Para qué sirve

Consultar si un contratista puede entrar, cuándo vence su PRAIND y si está
adentro, sin tener que registrar nada.

## Quién puede hacerlo

Cualquier usuario con sesión abierta en el escritorio.

## Dónde

**Contratistas**, en el menú lateral.

## Pasos

1. Abra **Contratistas**.
2. En el buscador ("Cédula o nombre…"), escriba parte de la cédula o del
   nombre. La tabla se filtra mientras escribe.
3. Mire la columna **Estado** de la persona.

Abajo, en la barra de la ventana, se ve cuántos resultados hay.

## Qué se ve en la tabla

| Columna | Qué muestra |
|---|---|
| Cédula, Nombre, Empresa | Datos de la persona |
| Tipo | PRAIND, IN_HOUSE o SWAT |
| PRAIND vence | Fecha de vencimiento del PRAIND, si la tiene |
| Estado | Si puede entrar o no, y por qué (ver abajo) |
| Adentro | "Sí" si está adentro ahora |
| Personal de ruta | Interruptor: si la persona es personal de ruta |

## Qué significa cada estado

| Estado | Color | Significa |
|---|---|---|
| "Puede entrar" | Verde | Todo en orden. |
| "Puede entrar (PRAIND vence en N días)" o "(PRAIND vence hoy)" | Amarillo | Puede entrar, pero debe renovar pronto (30 días o menos). |
| "PRAIND vencida hace N días" o "PRAIND vencida" | Rojo | No puede entrar hasta renovar el PRAIND. |
| "Sin PRAIND registrada" | Rojo | Necesita PRAIND y no tiene fecha registrada. |
| "Acceso denegado" | Rojo | Un administrador le quitó el acceso. |
| "Empresa inactiva" | Rojo | Su empresa está desactivada. |

Qué hacer en cada caso rojo: ver [PRAIND vencido, acceso denegado y otros
bloqueos](praind-y-bloqueos-de-acceso.md).

## Filtrar por columna

Debajo del título de cada columna hay un campo para filtrar sólo por esa
columna. Por ejemplo, en **Empresa** para ver a todos los de una empresa, o
en **PRAIND vence** para buscar por fecha.

## Relacionado

- [Dar de alta o editar un contratista](contratista-alta-y-edicion.md)
- [PRAIND vencido, acceso denegado y otros bloqueos](praind-y-bloqueos-de-acceso.md)
