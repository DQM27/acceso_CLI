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
- `dispositivo-nube.clave` -- la clave privada que identifica a ESTE
  equipo ante Supabase (`device-auth`, ver `nube::firmante`), también
  protegida con DPAPI. (Antes de 2026-09-30 era `dispositivo-nube.secret`,
  un secreto compartido; se retiró.)

> Nota: desde 2026-09-18 `db_key.dat` y la clave del dispositivo viven en
> `%APPDATA%\ControlAcceso`, separadas de la base (ver la sección de
> abajo); el párrafo anterior describe el diseño original.

**Un evento que destruye el `.db` y también `%APPDATA%` (disco muerto,
perfil de Windows corrupto, reinstalación completa) se lleva la clave del
dispositivo.** La recuperación NO es "reinstalar y que se sincronice
solo": ese equipo ya no puede autenticarse como sí mismo. Se re-vincula
con un código nuevo del panel.

Esto es una propiedad buena, no un defecto a corregir: **no existe ningún
atajo de recuperación que evite pasar por el panel** -- la restauración
usa la misma puerta de entrada, con la misma autorización de
administrador, que vincular un equipo nuevo. No hay un camino más débil
escondido para el caso de emergencia.

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
   flujo normal de "requiere configuración inicial" (vincular con un
   código del panel, pantalla ya existente).
5. Si el usuario dice que no, o si el reintento del paso 4 también falla,
   se muestra el error fatal de siempre (mismo mensaje/log/Sentry que
   cualquier otro fallo de arranque).

La clave del dispositivo (`dispositivo-nube.clave`) NO se toca en este
proceso -- sigue en `%APPDATA%`, separada de la base. Aun así, la pantalla
de configuración inicial pide un código: el administrador usa
"Re-vincular" en el panel (mismo `dispositivo_id`, conserva su
historial) y el equipo lo canjea generando una clave nueva. Pendiente
posible: si la clave sigue siendo válida, reconstruir sin pedir código
(ver `docs/handoff-registro-dispositivos.md`).

Implementación: `desktop/src-tauri/src/recuperacion_local.rs` (cuarentena +
borrado, con tests) y `desktop/src-tauri/src/lib.rs`
(`abrir_nucleo_con_recuperacion`/`confirmar_reconstruccion_desde_nube`).

## Procedimiento (manual, si el diálogo automático no aplica)

1. **En el panel, "Re-vincular" el dispositivo** (Dispositivos →
   Re-vincular): emite un código de un solo uso para el MISMO
   `dispositivo_id`, que conserva su sitio y todo su historial. Al
   canjearse, cualquier clave anterior deja de servir en el acto (si el
   equipo viejo sigue existiendo en otro lado, queda afuera).
   Alternativa: dar de alta un dispositivo nuevo en el mismo sitio
   (`admin-provision-device`, recibe el `sitio_id`).
2. **Reinstalar la app** (o dejar que arranque en una máquina/perfil
   limpio) y vincularla con ese código (`vincular_dispositivo_inicial`).
3. Si el equipo viejo no se va a volver a usar y se dio de alta uno nuevo,
   revocar el viejo (`admin-revoke-device`) -- higiene, evita dejar un
   `dispositivo_id` fantasma.
4. Con eso:
   - **Inmediato:** el catálogo completo del sitio -- empresas,
     contratistas, gafetes, rutas, vehículos, encargados
     (`recibir_catalogo_del_sitio`/`recibir_catalogo_rutas_del_sitio`, ver
     `application::nube::AppCore::vincular_dispositivo_inicial`).
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
