# Acceso negado en todas las puertas

Estado: **implementado en `claude/veto-por-persona`, pendiente de prueba del
dueño** (2026-09-28).

## La regla

Se niega el acceso con **el mismo interruptor de siempre**: `tiene_acceso`
del contratista (en la nube, `contratistas.activo`), que se cambia en la
pantalla Contratistas del panel web o del escritorio. Nada nuevo que
aprender ni que administrar.

Antes, ese interruptor sólo lo miraba la puerta de contratistas: la misma
persona entraba como **proveedor** o como **visita** con su cédula. Ahora
las tres puertas lo respetan:

| Puerta | Qué pasa si la cédula tiene el acceso negado |
|---|---|
| Contratista | Como siempre: acceso denegado (Regla 1 de `domain::acceso`) |
| Proveedor | No se registra: "Esta persona tiene el acceso denegado." |
| Visita | No entra, aunque tenga la cita agendada: el mismo mensaje |

Es una sola consulta
(`database::queries::contratistas::cedula_con_acceso_negado`) sobre la
tabla de contratistas que cada equipo ya tiene. Funciona sin internet y
llega a los equipos por la misma sincronización (y el mismo aviso en vivo)
que ya trae los cambios de contratistas. En Supabase no cambia nada.

## La cédula en forma única

Para que el bloqueo no se esquive cambiando el formato
(`1-1234-0567`, `112340567` y `01-1234-0567` son la misma persona), toda
cédula pasa por `domain::cedula::Cedula::normalizar`: quita espacios,
guiones y puntos, pasa a mayúsculas y quita el cero inicial del TSE. En
SQLite, su gemela `NORMALIZAR_CEDULA` compara los dos lados, así que los
registros viejos con guiones también cuentan.

Regla de documentos (dueño, 2026-09-27): contratistas y proveedores sólo
con cédula nacional o de extranjero, sólo números, de 9 a 13 dígitos. El
pasaporte (con letras) sólo en visitas.

## Límite conocido

Sólo se le puede negar el acceso a alguien que exista como contratista.
Para bloquear a un proveedor que nunca fue contratista, se lo da de alta
como contratista con el acceso apagado.

Desde el panel web: **Contratistas → + Nuevo**, con la casilla "Crear con el
acceso denegado". El formulario pide cédula, nombre, empresa y tipo de ingreso
(a quien se crea bloqueado no se le pide el PRAIND). La empresa es obligatoria:
los equipos descartan un contratista de la nube que no tenga una empresa que
puedan resolver, y sin ella el bloqueo no les llegaría. Si la empresa no
existe, se crea desde el mismo formulario (botón "+"). Las reglas (cédula en
forma única, 9 a 13 dígitos, nombre en mayúsculas, PRAIND según el tipo, cédula
repetida en cualquier formato) las aplican las funciones
`panel_crear_contratista` y `panel_crear_empresa` de Supabase (migración
`panel_crea_contratistas`), gemelas de las del núcleo; el panel solo muestra su
mensaje. La fila la ve la unidad por cuyo canal en vivo se avisa el cambio al
instante; las demás unidades la reciben en su siguiente sincronización.

## Apuntes sobre visitas (dueño, 2026-09-28: bajo riesgo, se dejan anotados)

Las visitas son personas seleccionadas por un anfitrión, así que estos
casos son poco probables. No se implementan por ahora:

- **La web de anfitriones deja agendar la cita** a alguien con el acceso
  negado. El bloqueo ocurre al llegar a la portería (check-in), no al
  agendar. Si hiciera falta avisar antes, se agregaría el mismo chequeo en
  la RPC `crear_cita_anfitrion` de la web de visitas.
- **Pasaporte en vez de cédula:** si la persona llega como visita con
  pasaporte, no coincide con la cédula de su ficha de contratista y entra.
  Es otro documento y el sistema no puede saber que es la misma persona.

## Descartado

Se llegó a construir un veto aparte, con tabla propia, motivo, historial y
funciones de administración (commits `7fee2e6`, `3577c50` y `2e98966`,
revertidos). El dueño pidió lo más simple: el mismo interruptor. Staging
volvió a quedar como antes (migración `revierte_personas_vetadas`).

## Pendiente, si se quiere más adelante

- Un solo ingreso activo por persona entre roles: que un contratista
  adentro no pueda entrar además como proveedor.
