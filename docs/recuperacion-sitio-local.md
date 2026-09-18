# Recuperación de un sitio local (`.db` corrupto o disco perdido)

Espejo de `docs/recuperacion-supabase.md`, pero para el otro lado: qué
hacer cuando el `.db` local de un sitio (escritorio o mobile) se corrompe,
se borra por accidente, o el disco/teléfono se pierde entero. Decisión
tomada 2026-09-17 (ver `docs/auditorias/plan-qa-buenas-practicas-2026-09-17.md`,
punto 7): depender de la nube como respaldo es aceptable, no se implementa
ningún respaldo local (justificación completa en ese documento).

## El punto de seguridad que hay que tener presente

En escritorio, tres archivos viven **juntos en la misma carpeta**
(`%LOCALAPPDATA%\ControlAcceso\`, ver `database::connection::ruta_base_datos`
y `nube::credenciales::directorio_default`):

- `control_acceso.db` -- la base en sí.
- `db_key.dat` -- la clave de `SQLCipher`, protegida con DPAPI.
- `dispositivo-nube.secret` -- el secreto que autentica a ESTE dispositivo
  contra Supabase (`device-auth`), también protegido con DPAPI.

**Esto significa que un evento que destruye el `.db` (disco muerto, perfil
de Windows corrupto, reinstalación que borra la carpeta) casi siempre
destruye también el secreto de dispositivo.** La recuperación NO es
"reinstalar y que se sincronice solo" -- ese dispositivo ya no tiene forma
de autenticarse como sí mismo. Hay que tratarlo como si fuera un
dispositivo nuevo, con el flujo normal de aprovisionamiento.

Esto es una propiedad buena, no un defecto a corregir: significa que
**no existe ningún atajo de recuperación que evite pasar por
`admin-provision-device`** -- la restauración de un sitio usa exactamente
la misma puerta de entrada, con la misma autorización de administrador,
que dar de alta un dispositivo nuevo desde cero. No hay un camino más
débil escondido para el caso de emergencia.

## Detección y cuarentena automáticas (escritorio, 2026-09-18)

Ya no hace falta seguir el procedimiento de abajo a mano desde el primer
síntoma. Si `AppCore::abrir_con_reloj_cifrado` o `clave_cifrado::resolver_clave`
fallan al arrancar, la app:

1. Loguea el error técnico real y lo manda a Sentry.
2. Muestra un diálogo nativo Sí/No: ofrece reconstruir desde la nube,
   explicando qué se pierde (auditoría/incidentes/lo que no llegó a subir).
3. Si el usuario dice que sí: copia `control_acceso.db` y `db_key.dat` (el
   que exista de los dos) a
   `<carpeta de la base>\respaldos-corruptos\<timestamp>\` -- para un
   rescate manual posterior con herramientas de recuperación de `SQLite`,
   sin garantía de que sirva pero sin costo real de intentarlo -- y borra
   los originales.
4. Reintenta abrir una vez más. Como ya no quedan `control_acceso.db` ni
   `db_key.dat`, esto crea una base nueva y una clave nueva, exactamente el
   mismo camino que un sitio nunca activado -- de ahí en adelante sigue el
   flujo normal de "requiere configuración inicial" (activar con secreto de
   dispositivo, pantalla ya existente, sin código nuevo).
5. Si el usuario dice que no, o si el reintento del paso 4 también falla,
   se muestra el error fatal de siempre (mismo mensaje/log/Sentry que
   cualquier otro fallo de arranque).

El secreto de dispositivo (`dispositivo-nube.secret`) NO se toca en este
proceso -- sigue en `%APPDATA%`, separado desde el fix de arriba. En el
caso más común (se corrompió sólo la base, no todo el perfil), el
dispositivo puede reactivarse con su secreto de siempre en la pantalla de
configuración inicial, sin que un administrador tenga que intervenir.

Implementación: `desktop/src-tauri/src/recuperacion_local.rs` (cuarentena +
borrado, con tests) y `desktop/src-tauri/src/lib.rs`
(`abrir_nucleo_con_recuperacion`/`confirmar_reconstruccion_desde_nube`).

## Procedimiento (manual, si el diálogo automático no aplica)

1. **Si el dispositivo viejo todavía es alcanzable de algún modo** (ej. el
   disco murió pero se pudo copiar el secreto antes), revocarlo con
   `admin-revoke-device` -- higiene, no es estrictamente necesario para que
   el nuevo funcione, pero evita dejar un `dispositivo_id` fantasma
   habilitado en la tabla de dispositivos.
2. **Aprovisionar un secreto nuevo** para el mismo sitio, vía
   `admin-provision-device` (mismo `sitio_nombre` que ya existe -- la
   función hace `upsert` por nombre, así que resuelve al `sitio_id` que ya
   tenía toda la historia, no crea un sitio duplicado). Esto es una acción
   de administrador, igual que activar cualquier PC/teléfono nuevo.
3. **Reinstalar la app** (o dejar que arranque en una máquina/perfil
   limpio) y activarla con el secreto nuevo del paso 2
   (`configurar_dispositivo_inicial`).
4. Con eso:
   - **Inmediato:** el catálogo completo del sitio -- empresas,
     contratistas, gafetes, rutas, vehículos, encargados
     (`recibir_catalogo_del_sitio`/`recibir_catalogo_rutas_del_sitio`, ver
     `application::nube::AppCore::configurar_dispositivo_inicial`).
   - **Dentro de los próximos ~2 minutos**, sin que nadie tenga que hacer
     nada: la sincronización automática en segundo plano
     (`iniciar_sincronizacion_automatica`, cada
     `INTERVALO_SINCRONIZACION_AUTOMATICA`) trae el historial de
     ingresos/salidas y movimientos de visita
     (`recibir_historial_del_sitio`/`recibir_historial_visitas_del_sitio`/
     `recibir_historial_ingresos_proveedor_del_sitio`) y los abiertos de
     otros dispositivos del mismo sitio.

## Qué NO se recupera (decisión aceptada, no un bug)

- **`auditoria_cambios`/`auditoria_contratistas`** -- el log de "quién
  cambió qué" de ese dispositivo en particular, desde siempre.
- **`gafetes_incidentes`** -- historial de incidentes de gafetes de ese
  dispositivo.
- **Lo que estuviera en `cola_salida` sin terminar de subir** en el
  momento exacto del desastre (normalmente segundos/minutos de operaciones
  recientes).

Todo lo demás (contratistas, ingresos, préstamos de gafete, rutas,
proveedores, historial) tiene contraparte remota completa y se reconstruye
solo con el procedimiento de arriba.

## A valorar más adelante (no ahora, sin decisión tomada)

Si en algún momento se decide que la pérdida de auditoría/incidentes deja
de ser aceptable (ej. un cliente/auditor externo lo exige), la vía natural
es sumar esas tres tablas al mecanismo de sync existente (mismo patrón que
`cola_salida` ya usa para todo lo demás) -- no requiere una arquitectura
nueva, sólo el mismo trabajo que ya se hizo para el resto de las tablas.
No se implementa hoy porque no hay una necesidad real detrás todavía.
