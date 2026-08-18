# Plan de respaldo y recuperación

> **Prioridad:** alta, obligatoria antes de producción.
>
> **Estado:** Fase 1 (motor de creación y validación) y el motor de la Fase 2
> (`restaurar_respaldo`) están implementados y probados. La pantalla de la
> Fase 3 (Configuración → Respaldos) ya está en la TUI con Crear, Listar,
> Revalidar y Exportar — ver detalle al final de cada sección. Restaurar
> desde la pantalla (Fase 3, acción pendiente) y la orquestación de la Fase 2
> con la TUI (cerrar sesión, descartar `AppCore`, exigir login nuevo) quedan
> deliberadamente para después, junto con la Fase 4 (automatización/retención).

## Objetivo

Poder crear, verificar, listar, exportar y restaurar copias consistentes de SQLite sin
depender de internet y sin arriesgar la base activa. Una restauración fallida debe
recuperar automáticamente el estado que existía antes de iniciarla.

Este trabajo debe completarse antes de publicar la aplicación o ejecutar la primera
actualización productiva que cambie el esquema de la base.

## Decisiones de diseño

- La base activa conserva su ubicación fija en
  `%LOCALAPPDATA%\ControlAcceso\control_acceso.db`.
- Los respaldos internos se guardan en
  `%LOCALAPPDATA%\ControlAcceso\backups`.
- La interfaz no permitirá mover la base activa. Permitirá exportar una copia cerrada y
  verificada a otra ubicación.
- Crear un respaldo utilizará la
  [Online Backup API de SQLite](https://www.sqlite.org/backup.html), no una copia directa
  del archivo mientras SQLite esté abierto.
- La restauración se ejecutará después de cerrar `AppCore` y su conexión SQLite, pero
  manteniendo el bloqueo de instancia.
- La pantalla será un cliente del motor de respaldo; no contendrá lógica de copia,
  validación, retención o restauración.
- La aplicación continuará funcionando completamente sin conexión a internet.

## Alcance de la primera versión

### Incluido

- Crear respaldos manuales.
- Crear respaldos automáticos antes de migraciones.
- Listar y validar respaldos internos.
- Exportar un respaldo ya verificado.
- Restaurar con copia de seguridad previa y rollback automático.
- Aplicar una política de retención únicamente a respaldos automáticos.
- Mostrar errores comprensibles al operador y conservar el detalle técnico en logs.

### Fuera de alcance inicialmente

- Sincronización en nube.
- Copias programadas cuando la aplicación está cerrada.
- Edición o navegación de registros dentro de un respaldo.
- Restauración selectiva de contratistas, empresas o movimientos individuales.
- Cifrado propio de archivos y administración de llaves.

## Formato y estados

Nombre propuesto:

```text
control_acceso_2026-08-15_143000_manual.db
control_acceso_2026-08-15_143000_automatico.db
control_acceso_2026-08-15_143000_pre_migracion.db
control_acceso_2026-08-15_143000_pre_restauracion.db
```

Mientras se construye, el archivo utilizará la extensión `.partial`. Sólo recibirá el
nombre definitivo después de superar todas las verificaciones. Nunca se sobrescribirá
un respaldo existente.

Cada elemento listado debe exponer como mínimo:

- Ruta e identificador único.
- Fecha y hora de creación.
- Tipo de respaldo.
- Tamaño.
- `PRAGMA user_version`.
- Estado: pendiente, válido, inválido o incompatible.
- Resultado y fecha de la última verificación.

## Fase 1: motor de creación y validación — implementada

Implementada en `src/database/backup.rs`, sin ninguna dependencia de la TUI ni de
`AppCore` (recibe una `&Connection` ya abierta y un directorio destino; el futuro
`AppCore`/pantalla serán clientes finos de este módulo, tal como pedía el diseño).

1. [x] Módulo de infraestructura para la Online Backup API — `rusqlite::backup::Backup`
   (feature `backup` de `rusqlite` habilitada en `Cargo.toml`).
2. [x] Tipos neutrales: `TipoRespaldo`, `RespaldoResumen`, `ResultadoValidacion`,
   `RespaldoError`.
3. [x] `crear_respaldo` crea el directorio de respaldos si no existe
   (`fs::create_dir_all`).
4. [x] Escribe primero a un archivo `.partial` con nombre único (`ruta_disponible` nunca
   sobrescribe uno existente, agrega sufijo numérico ante colisión).
5. [x] Ejecuta el respaldo desde la conexión activa vía `Backup::run_to_completion`.
6. [x] Abre la copia aparte (`OpenFlags::SQLITE_OPEN_READ_ONLY`, nunca crea ni modifica)
   y corre `integrity_check` + `foreign_key_check` — ambas, por la misma razón que
   señala este documento.
7. [x] Lee y guarda `PRAGMA user_version` en el resultado de la validación.
8. [x] Sólo renombra al nombre definitivo (`.db`) si la validación resultó `Valido`.
9. [x] Si la validación falla, borra el `.partial` antes de devolver el error — nunca
   deja un respaldo a medias publicado.
10. [x] `listar_respaldos` (barato, sólo sistema de archivos + nombre, nunca abre
    SQLite, nunca lista `.partial`) y `validar_respaldo` (costoso, bajo demanda) son
    funciones separadas, tal como pedía el diseño.

Probado en `tests/respaldo_backup.rs`: un respaldo real conserva los datos y las
relaciones, no queda ningún `.partial` tras una creación exitosa, dos respaldos
seguidos no se pisan entre sí, se rechaza un archivo truncado, se rechaza un
`user_version` de una versión futura, se rechazan claves foráneas inválidas, y el
listado ordena del más reciente y nunca incluye `.partial`.

**Deliberadamente fuera de esta fase** (quedan para Fase 2-4): exportar una copia ya
verificada a otra ubicación, restauración, la pantalla de la TUI, y la política de
retención/automatización antes de migraciones.

## Fase 2: restauración segura — motor implementado, orquestación con la TUI pendiente

`restaurar_respaldo(ruta_candidata, ruta_activa)` en `src/database/backup.rs` implementa
los pasos que son puramente de motor/archivo (1-4, 8-11); es una función pura que no
depende de `AppCore` ni de la TUI, igual que el resto de este módulo. Los pasos 5-7 y 12
(terminar sesión, cerrar la TUI, mantener `InstanciaGuard` vivo, exigir login nuevo) son,
por diseño, responsabilidad de quien la llame — hoy nadie todavía, porque la orquestación
con `App`/`main.rs` se conecta cuando se construya la pantalla (Fase 3).

1. [x] Selección por identificador: la función recibe una `&Path` ya resuelta (la que
   entrega `listar_respaldos`), nunca texto de interfaz interpretado como ruta — eso le
   toca comprobarlo a la futura pantalla antes de llamar aquí.
2. [x] Vuelve a correr `validar_respaldo` (integridad + claves foráneas + compatibilidad
   de esquema) antes de tocar cualquier archivo.
3. [x] Rechaza una base de una versión de esquema futura (vía `validar_respaldo`, antes de
   copiar nada).
4. [ ] Crear el respaldo `pre_restauracion` de la base activa — **no está dentro de
   `restaurar_respaldo`, a propósito**: para respaldar con la Online Backup API hace falta
   una `&Connection` abierta, y esta función asume la conexión ya cerrada (ver punto
   siguiente). Le corresponde a quien orqueste: llamar a `crear_respaldo(&conexion, dir,
   TipoRespaldo::PreRestauracion)` primero, con la conexión todavía abierta.
5. [ ] Terminar sesión y cerrar la TUI — pendiente, vive en `App`/`main.rs`, no en este
   módulo.
6. [ ] Descartar `AppCore` — pendiente, mismo motivo; la documentación de la función deja
   explícito que debe llamarse con la conexión ya cerrada.
7. [ ] Mantener `InstanciaGuard` vivo — pendiente; esta función no lo toca, es
   responsabilidad del llamador (ya está vivo en `main.rs` durante toda la ejecución).
8. [x] Copia la candidata a un temporal en el mismo directorio antes de tocar la base
   activa — si la copia falla, la base activa ni se entera.
9. [x] Intercambia con dos `rename` (activa → `.previa`, temporal → activa) en vez de una
   sobrescritura directa.
10. [x] Abre la base ya intercambiada y corre `initialize_database` de verdad (aplica
    migraciones reales, no sólo el chequeo superficial de `validar_respaldo`).
11. [x] Si cualquier paso posterior al intercambio falla, reinstala automáticamente
    `.previa` sobre la ruta activa.
12. [ ] Volver al flujo de arranque y exigir login nuevo — pendiente, vive en `App`.

No se mueve ni reemplaza una base mientras tenga una conexión abierta — la documentación
de la función lo deja como precondición explícita. SQLite documenta los riesgos de copiar
o restaurar durante transacciones activas en
[How To Corrupt An SQLite Database File](https://sqlite.org/howtocorrupt.html).

Probado en `tests/respaldo_backup.rs`: restaurar un candidato válido reemplaza los datos
activos por los de la candidata; un candidato inválido se rechaza sin tocar la base
activa; si la migración falla después del intercambio (candidato "sano" en la validación
superficial pero con un esquema real incompatible), se reinstala automáticamente la base
anterior y no queda ningún archivo temporal; restaurar sobre una ruta activa inexistente
funciona como primera carga.

## Fase 3: pantalla de respaldos — Crear/Listar/Revalidar/Exportar implementados

Se agregó una entrada `Configuración` al menú principal (visible sólo para ROOT y
Administrador — primer filtrado por rol de la app, en
`src/tui/menu_principal/state.rs::OpcionMenu::visibles_para`), que abre una pantalla con
una lista de sub-secciones (`src/tui/configuracion/`, hoy sólo `Respaldos`, pensada para
crecer sin rediseño). Dentro de Respaldos, la tabla muestra:

```text
Fecha | Tipo | Tamaño | Esquema | Estado
```

tal como se especificó arriba. Acciones implementadas:

- [x] Crear respaldo manual (`C`).
- [x] Actualizar la lista (`A`).
- [x] Verificar nuevamente un respaldo puntual (`R`) — no se valida el listado completo al
  cargar (potencialmente costoso, abre cada `.db`); "Esquema"/"Estado" quedan en `—` hasta
  que el operador revalida esa fila.
- [x] Exportar una copia ya validada a una ruta que tipea el operador (`E`) — copia simple
  (`AppCore::exportar_respaldo`, `std::fs::copy`), sin volver a pasar por el motor de
  respaldo porque el archivo interno ya fue validado al crearse.
- [ ] Restaurar — deliberadamente fuera de esta pasada (ver Fase 2, "orquestación con la
  TUI pendiente": exige cerrar `AppCore` y reabrir la app).
- [ ] Eliminar un respaldo no utilizado, con confirmación — no se acordó para esta pasada;
  mismo patrón que Exportar cuando se agregue.

La pantalla es un cliente delgado del motor: `AppCore` gana `crear_respaldo`,
`listar_respaldos`, `validar_respaldo` y `exportar_respaldo`, cada uno una línea que
delega a `database::backup` (mismo patrón de fachada que el resto de `AppCore`). El
directorio de respaldos se resuelve como `<directorio de la base activa>/backups`, vía un
nuevo campo `AppCore::ruta_base_datos` poblado sólo en `AppCore::abrir` (no rompe
`AppCore::new`/`con_reloj`, usados por la mayoría de los tests existentes).

Probado en `tests/configuracion_respaldos.rs` (crear/listar/validar y exportar a través de
`AppCore`, y que el directorio de respaldos se ubica junto a la base activa) y en
`src/tui/configuracion/tests.rs` (navegación Menu ↔ Respaldos, Esc no burbujea desde el
sub-modo, crear/revalidar disparan la acción correcta sólo con una fila seleccionada).

La restauración debe requerir una confirmación fuerte que indique fecha y tipo del
respaldo, cuando se implemente. La pantalla no edita ni consulta los datos internos como
si fueran la base activa.

## Fase 4: automatización y retención

Política inicial propuesta:

- Respaldo obligatorio antes de cualquier migración de esquema.
- Como máximo un respaldo automático diario cuando la aplicación se inicia.
- Conservar los últimos 7 respaldos automáticos.
- Conservar los últimos 3 respaldos previos a migraciones.
- No eliminar automáticamente respaldos manuales ni exportados.
- No aplicar retención hasta que el nuevo respaldo esté validado.

Estos valores podrán ajustarse antes de producción con datos reales de tamaño y uso.

## Pruebas obligatorias

- Crear un respaldo mientras la base está abierta y contiene datos.
- Comprobar que el respaldo conserva conteos y relaciones.
- Rechazar una copia truncada, corrupta o con claves foráneas inválidas.
- No publicar un archivo `.partial` después de un fallo.
- Rechazar una versión de esquema futura.
- Restaurar una base válida y autenticar contra sus datos.
- Simular fallo después del intercambio y recuperar la base anterior.
- Simular carpeta no escribible y disco sin espacio sin dañar la base activa.
- Probar retención sin eliminar respaldos manuales.
- Probar que una segunda instancia no puede respaldar ni restaurar la misma base.
- Ejecutar una restauración completa en Windows como prueba operativa previa a una
  entrega.

## Criterios de finalización

Este punto sólo podrá marcarse como completado cuando:

- El motor funcione sin depender de la TUI.
- Crear, validar y restaurar tengan pruebas automatizadas.
- Una restauración fallida haya demostrado rollback automático.
- La pantalla no pueda sobrescribir archivos mediante rutas manipuladas.
- Los errores sean visibles y no se silencien con `.ok()`.
- La política de retención esté documentada y probada.
- Se haya realizado al menos una restauración operativa completa con una copia real de
  prueba.
- El procedimiento de recuperación manual esté documentado para el caso de que la
  aplicación no pueda iniciar.

## Riesgo que permanece

Los respaldos internos están en el mismo disco que la base activa. Protegen contra
errores de aplicación, migraciones y restauraciones defectuosas, pero no contra pérdida
o daño físico del equipo. Antes de producción debe existir un procedimiento periódico
para exportar una copia verificada a otro dispositivo seguro.
