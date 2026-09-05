# Realtime: corrección y verificación del 5 de septiembre de 2026

La suscripción privada del proyecto funciona tras corregir la política de
autorización y conservar el token del dispositivo en el cliente JavaScript.
No fue necesario sustituir las claves ES256 ni abrir los canales privados.

## Causas comprobadas

- La política de `realtime.messages` exigía `private = true`. Supabase evalúa
  el permiso de entrada usando filas temporales que omiten ese campo; su
  valor predeterminado en este proyecto es `false`. La política impedía leer
  esas filas aunque el sitio y el JWT fueran correctos.
- `setAuth(token)` sin el callback `accessToken` no conservaba el token del
  dispositivo durante la conexión con la versión instalada de supabase-js.
  Se verificó que el valor cambiaba y que el canal devolvía `Unauthorized`.
- Los triggers de Broadcast ya estaban instalados y habilitados. La nota
  que indicaba que faltaba aplicar el SQL estaba desactualizada.

La migración `20260905091122_corregir_autorizacion_realtime.sql` está aplicada
en remoto. Conserva el rol `authenticated`, el filtro de Broadcast y la
igualdad entre el canal solicitado y el sitio del JWT. La prueba SQL en
`supabase/tests/realtime_autorizacion.sql` crea una fila temporal y revierte
toda la transacción al terminar.

## Comportamiento de los clientes

- Escritorio usa Broadcast privado, renueva el JWT y recupera cambios al
  suscribirse. Los avisos que llegan durante una sincronización quedan
  pendientes. Las entradas, salidas y cambios de catálogo solicitan una
  subida sin esperar al pulso periódico. La red se ejecuta fuera del hilo
  de interfaz y sin mantener bloqueado el núcleo compartido.
- Android vuelve a abrir Realtime al entrar en primer plano y lo cierra
  al salir. Los avisos locales y remotos pasan por el mismo sincronizador,
  que agrupa pendientes. El colector del canal se cancela antes de renovar
  el token para que no retenga indefinidamente la sesión anterior.
- El historial y el catálogo web escuchan Postgres Changes sobre las
  tablas ya publicadas (`ingresos`, `contratistas`, `empresas`), con la
  sesión autenticada y las políticas existentes del panel. Las demás
  pantallas administrativas mantienen su actualización periódica.
- La sincronización periódica sigue como respaldo ante pérdida de conexión.

## Verificaciones

- JWT ES256 del dispositivo: firma válida con la clave pública publicada,
  vigencia correcta y lectura REST con HTTP 200.
- Dos clientes independientes alcanzaron `SUBSCRIBED` en el canal privado
  y recibieron el mismo evento de diagnóstico emitido desde Supabase.
  No se modificaron registros operativos para esta prueba.
- Suscripciones WebSocket: sitio propio permitido, otro sitio rechazado,
  cliente sin sesión rechazado.
- SQL: cuatro comprobaciones correctas de autorización, incluyendo una
  sesión autenticada sin sitio. Todos los datos de prueba se revirtieron.
- Frontend de escritorio: 149 pruebas correctas, incluidas renovación,
  cierre de sesión, agrupación de eventos y pendientes durante una recarga.
- Web: dos pruebas de recarga por eventos, visibilidad y limpieza del canal.
- Android: compilación y APK debug correctos; 17 pruebas JVM correctas.
- Frontends web y escritorio: compilación de producción correcta.

Las pruebas nativas de Tauri encontraron un bloqueo del entorno:
GNU/MinGW produce `STATUS_ENTRYPOINT_NOT_FOUND` al arrancar el ejecutable;
el toolchain MSVC instalado no encuentra `link.exe`. Es el problema descrito
en `desktop/docs/pendientes.md`, sección de entorno de desarrollo. La
validación de tipos de Rust se ejecuta por separado con `cargo check`.

El asesor de seguridad devolvió los mismos avisos anteriores al cambio:
acceso a `es_admin_global()` como función SECURITY DEFINER y protección de
contraseñas filtradas desactivada. No aparecieron avisos nuevos por esta
migración. Referencias para esos avisos:
[funciones públicas](https://supabase.com/docs/guides/database/database-linter?lint=0028_anon_security_definer_function_executable),
[funciones autenticadas](https://supabase.com/docs/guides/database/database-linter?lint=0029_authenticated_security_definer_function_executable),
[contraseñas](https://supabase.com/docs/guides/auth/password-security#password-strength-and-leaked-password-protection).

## Referencias del diagnóstico

- [Autorización oficial de Realtime](https://supabase.com/docs/guides/realtime/authorization).
- [Código de las comprobaciones internas](https://github.com/supabase/realtime/blob/main/lib/realtime/tenants/authorization.ex).
- [Esquema del mensaje usado para comprobar permisos](https://github.com/supabase/realtime/blob/main/lib/realtime/api/message.ex).

La base remota ya tiene la corrección. Los cambios de cliente entran en uso
al ejecutar o distribuir las nuevas compilaciones; no se publicó una
release ni se reemplazaron las aplicaciones instaladas durante esta tarea.
