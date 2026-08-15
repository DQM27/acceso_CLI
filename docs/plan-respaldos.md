# Plan de respaldo y recuperación

> **Prioridad:** alta, obligatoria antes de producción.
>
> **Estado:** planificado; ejecución aplazada de forma intencional mientras la
> aplicación continúa en desarrollo.

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

## Fase 1: motor de creación y validación

1. Crear un módulo de infraestructura para la Online Backup API de SQLite.
2. Definir tipos de aplicación neutrales para solicitud, resumen, validación y error.
3. Crear el directorio de respaldos si no existe.
4. Escribir primero a un archivo `.partial` único.
5. Ejecutar el respaldo desde la conexión activa.
6. Abrir la copia por separado y ejecutar:

   ```sql
   PRAGMA integrity_check;
   PRAGMA foreign_key_check;
   ```

   `integrity_check` no cubre claves foráneas, por lo que ambas verificaciones son
   necesarias según la [documentación de PRAGMA de SQLite](https://sqlite.org/pragma.html).

7. Leer y guardar la versión del esquema.
8. Renombrar el archivo únicamente si todas las verificaciones terminan bien.
9. Eliminar archivos `.partial` fallidos sin tocar respaldos válidos.
10. Implementar listado y validación bajo demanda.

## Fase 2: restauración segura

1. Seleccionar el respaldo mediante su identificador, nunca mediante texto de interfaz
   usado directamente como ruta.
2. Volver a ejecutar integridad, claves foráneas y compatibilidad de esquema.
3. Rechazar una base creada por una versión futura que la aplicación no comprenda.
4. Crear un respaldo `pre_restauracion` de la base activa.
5. Solicitar al bucle principal terminar la sesión y cerrar la TUI.
6. Descartar `AppCore` para cerrar completamente SQLite.
7. Mantener vivo `InstanciaGuard` durante toda la operación.
8. Copiar la candidata a un archivo temporal dentro del directorio de datos.
9. Intercambiar la base activa sin destruir inmediatamente la anterior.
10. Abrir la base restaurada, aplicar sólo migraciones compatibles y repetir las
    verificaciones.
11. Si cualquier paso falla, reinstalar automáticamente la base anterior y verificarla.
12. Si termina correctamente, volver al flujo de arranque y exigir un nuevo login.

No se moverá ni reemplazará una base mientras tenga una conexión abierta. SQLite
documenta los riesgos de copiar o restaurar durante transacciones activas en
[How To Corrupt An SQLite Database File](https://sqlite.org/howtocorrupt.html).

## Fase 3: pantalla de respaldos

Añadir una opción `Respaldos` al menú de mantenimiento con una tabla que muestre:

```text
Fecha | Tipo | Tamaño | Esquema | Estado
```

Acciones previstas:

- Crear respaldo manual.
- Actualizar la lista.
- Ver detalles y resultado de verificación.
- Verificar nuevamente.
- Exportar una copia.
- Restaurar.
- Eliminar, con confirmación, únicamente un respaldo no utilizado por una operación.

La restauración debe requerir una confirmación fuerte que indique fecha y tipo del
respaldo. La pantalla no podrá editar ni consultar los datos internos como si fueran la
base activa.

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
