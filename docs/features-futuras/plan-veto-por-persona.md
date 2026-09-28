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

## Descartado

Se llegó a construir un veto aparte, con tabla propia, motivo, historial y
funciones de administración (commits `7fee2e6`, `3577c50` y `2e98966`,
revertidos). El dueño pidió lo más simple: el mismo interruptor. Staging
volvió a quedar como antes (migración `revierte_personas_vetadas`).

## Pendiente, si se quiere más adelante

- Un solo ingreso activo por persona entre roles: que un contratista
  adentro no pueda entrar además como proveedor.
