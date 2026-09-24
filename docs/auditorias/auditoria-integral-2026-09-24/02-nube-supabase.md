# Auditoría 02: nube, Supabase y sincronización (escritorio y móvil)

Fecha: 2026-09-24. Análisis estático, solo lectura. No se ejecutó nada contra Supabase remoto ni `cargo build/test`.
Alcance: `src/nube/*`, `src/application/nube.rs`, `supabase/functions/*`, `supabase/migrations/*` (estado final tras las 79 migraciones), `supabase/tests`, y los puntos de `desktop/src-tauri` y `mobile/rust-core` que orquestan la nube.
Se contrastó con `docs/auditorias/auditoria-seguridad-web-supabase-2026-09-10.md` (A-xx), `reporte-seguridad-web-2026-09-09.md` (P-xx) y `auditoria-integral-android-2026-09-09.md`.

## Resumen ejecutivo

| Severidad | Cantidad |
|---|---|
| Crítica | 0 |
| Alta | 7 |
| Media | 10 |
| Baja | 8 |
| Info | 2 |

**Los 5 hallazgos principales**
1. **NS-01**: con dos o más `salida_ruta` pendientes se dispara un `unreachable!()`. El pánico aborta la sincronización en cada ciclo y la cola queda bloqueada.
2. **NS-02**: "Eliminar" un dispositivo que ya tiene historial solo lo oculta, no lo revoca. El secreto sigue autenticando y el dispositivo ya no se ve en el panel.
3. **NS-03**: el cierre enviado antes que la apertura se marca como `enviado` sin afectar ninguna fila. Queda un ingreso abierto "fantasma" en la nube (también en visitas, proveedores, KOF y rutas).
4. **NS-04**: cualquier dispositivo de cualquier sitio, incluido `visor`, puede hacer UPDATE global de `contratistas.activo` y `empresas`. Es el pendiente A-01 que sigue abierto para estas tablas.
5. **NS-05**: el login con Supabase Auth une la identidad solo por el correo sintético `cedula@brisas.local`. No se compara `auth_user_id`. Si el registro público está abierto, se puede tomar una cuenta que aún no tiene usuario en Auth, incluidas cuentas ROOT.

Otros hallazgos Altos: NS-06 (el upsert por clave natural reescribe el `id` y bloquea para siempre la sincronización de contratistas y sus ingresos), NS-07 (token de dispositivo de 12 h; revocar o suspender no tiene efecto inmediato, empeora A-04).

---

## Hallazgos

### [NS-01] Pánico `unreachable!` con dos o más `salida_ruta` pendientes
- Severidad: Alta
- Categoría: Integridad de datos
- Ubicación: `src/nube/sincronizacion.rs:150` (`destino_lote`), `:175` (`construir_cuerpo`), `:241-259` (`drenar_grupo`)
- Estado: Nuevo
- Descripción y evidencia: `destino_lote` devuelve un lote para `("salida_ruta", _) => Some(("salidas_ruta", None))`, pero `construir_cuerpo` no tiene rama `"salida_ruta"` y cae en `otra => unreachable!(...)`. `salida_ruta_repository.rs:199` encola `"crear"`. Cuando `grupo.len() > 1`, `enviar_lote` llama a `construir_cuerpo` y el programa entra en pánico. Las pruebas (`sincronizacion.rs:5648`) solo cubren una salida.
- Escenario de impacto: el punto de acceso registra dos salidas de ruta sin conexión. En cada sincronización (cada 2 min, en escritorio, móvil y Realtime) el hilo entra en pánico dentro de `drenar_cola`. No corre nada de lo que viene después: cierres, catálogo (incluidas bajas de usuarios y contratistas), historial, ni la expulsión de sesión. La situación se mantiene hasta que alguien edite la base local. En móvil, un pánico que cruza UniFFI se convierte en excepción, y con `panic="abort"` (perfil `production`) el proceso se cierra.
- Referencia externa: CWE-617 (aserción alcanzable) https://cwe.mitre.org/data/definitions/617.html
- Recomendación: agregar `"salida_ruta" => construir_cuerpo_salida_ruta(...)` o quitar `salida_ruta` de `destino_lote`. Reemplazar el `unreachable!` por un `Err` que caiga al camino fila por fila. Agregar una prueba con dos `salida_ruta` pendientes y otra que verifique que cada entidad de `destino_lote` tiene su cuerpo.

### [NS-02] "Eliminar" un dispositivo con historial lo oculta pero lo deja activo
- Severidad: Alta
- Categoría: Seguridad
- Ubicación: `supabase/functions/admin-delete-device/index.ts:70-88`; `supabase/functions/device-auth/index.ts:111-135`; `web/src/pantallas/Dispositivos.tsx:439-446`
- Estado: Nuevo
- Descripción y evidencia: si el DELETE falla con `23503` (el dispositivo ya generó filas), la función solo hace `update({ oculto_en_panel: true })` y responde `ok: true`. No cambia `revoked_at` ni `suspended_at`. `device-auth` solo filtra por `revoked_at IS NULL` y `suspended_at`, y la migración `20260906202346` dice explícitamente que ocultar "no afecta autenticación". El botón "Eliminar" se muestra también para dispositivos no revocados, y el panel filtra los ocultos (`Dispositivos.tsx:170`).
- Escenario de impacto: se pierde un teléfono y el admin pulsa "Eliminar". El dispositivo desaparece del panel, pero su secreto sigue emitiendo tokens de 12 h indefinidamente: lectura del catálogo global (cédulas y usuarios) y escritura de ingresos y contratistas. No queda ninguna señal visible para el admin. Solo se puede recuperar por SQL.
- Referencia externa: OWASP ASVS 5.0 V7 (Session/Token revocation) https://github.com/OWASP/ASVS ; OWASP API5:2023 https://owasp.org/API-Security/editions/2023/en/0xa5-broken-function-level-authorization/
- Recomendación: en `admin-delete-device`, fijar siempre `revoked_at = now()` antes de intentar borrar u ocultar. En el panel, solo permitir eliminar dispositivos ya revocados. Registrar quién hizo la acción (`revoked_by`).

### [NS-03] Un cierre que llega antes que su apertura se da por enviado y la apertura queda abierta para siempre
- Severidad: Alta
- Categoría: Integridad de datos
- Ubicación: `src/nube/sincronizacion.rs:888-975` (el cuerpo de apertura no incluye `hora_salida`), `:1006-1043`, `:1133-1171`, `:1244-1283`, `:1417-1459`, `:1727-1763`; `:352-356` (`_ => Ok(())`)
- Estado: Nuevo
- Descripción y evidencia: el cierre es `PATCH ...?id=eq.{uuid}&hora_salida=is.null` con `Prefer: return=minimal`, que responde 204 aunque no afecte ninguna fila. El propio comentario lo asume: "0 filas afectadas, no un error". Si el `crear` del mismo ingreso falló de forma transitoria (5xx, FK de contratista aún no subida, backoff), el `cerrar` de la misma pasada o de una posterior se marca `enviado`. Cuando el `crear` por fin entra, sube sin `hora_salida` (`construir_cuerpo_ingreso` no la envía). Pasa lo mismo con `movimientos_visita`, `ingresos_proveedor`, `prestamos_gafete_provisional` y `salidas_ruta`. Además, el comodín `_ => Ok(())` marca como `enviado` cualquier combinación entidad/operación desconocida.
- Escenario de impacto: la persona salió, pero en la nube sigue "adentro". Los demás dispositivos del sitio la ven en Activos (`recibir_ingresos_abiertos`). `gafete_ocupado_en_otro_dispositivo` bloquea ese gafete (la política de producto es "bloquear si hay duda") y el panel web muestra una ocupación falsa. No hay reconciliación.
- Referencia externa: PostgREST, Prefer return/count https://docs.postgrest.org/en/stable/references/api/preferences.html
- Recomendación: incluir los campos de cierre en el cuerpo del `crear` (upsert idempotente, compatible con los triggers de inmutabilidad), o usar `Prefer: return=representation`/`count=exact` y dejar pendiente el cierre si afectó 0 filas y la fila no existe. Cambiar `_ => Ok(())` por un error explícito.

### [NS-04] Escritura global de `contratistas` y `empresas` para cualquier dispositivo, incluido `visor`
- Severidad: Alta
- Categoría: Seguridad
- Ubicación: estado final: `supabase/migrations/20260906205504_globaliza_contratistas_y_empresas.sql` + `20260907031334_cierra_acceso_global_...sql:47-80`; inserción: `20260905160953...sql:12-16`; también `vehiculos_ruta`/`encargados_ruta` (`20260915184544`, `20260915185520`) y `gafetes`/`rutas` (inserción y actualización sin exclusión de `visor`)
- Estado: Pendiente de auditoría previa (A-01 de 2026-09-10; corregido solo para `usuarios` en `20260912064233`)
- Descripción y evidencia: `"actualizar contratistas (global)" using/with check ((jwt->>'sitio_id') is not null or private.es_admin_global())`. No verifica el sitio ni `tipo <> 'visor'`. Un JWT de dispositivo de cualquier sitio puede hacer `PATCH /rest/v1/contratistas?identificacion=eq.X {"activo":true}`. El propio upsert de la app reescribe también `sitio_id`/`dispositivo_origen_id` (la procedencia) de la fila global. `supabase/tests/contratistas_autorizacion.sql` documenta la lectura global, pero no prueba que un `visor` o un sitio ajeno no puedan escribir.
- Escenario de impacto: con el secreto de un visor web (pensado como solo lectura) o de un teléfono de otro sitio, se reactiva en todos los sitios a un contratista vetado. También se le puede quitar el acceso a todos (denegación de servicio operativa). Combinado con NS-02 y NS-07, persiste tras "eliminar" o revocar.
- Referencia externa: Supabase RLS https://supabase.com/docs/guides/database/postgres/row-level-security ; OWASP API1:2023 BOLA https://owasp.org/API-Security/editions/2023/en/0xa1-broken-object-level-authorization/
- Recomendación: quitar el UPDATE directo de `activo`/`tiene_acceso` a los dispositivos (solo `admin_global` o una RPC auditada), o al menos exigir `tipo in ('pc','mobile')`. Si el dispositivo debe editar datos, usar una RPC con lista blanca de columnas, sin tocar `sitio_id`/`dispositivo_origen_id` de la fila global. Extender `supabase/tests` con casos de visor y de otro sitio.

### [NS-05] El login con Supabase Auth une la persona solo por correo sintético, sin verificar `auth_user_id`
- Severidad: Alta (condicionada a la configuración de Auth en producción)
- Categoría: Seguridad
- Ubicación: `desktop/src-tauri/src/comandos/autenticacion.rs:243-295`; `mobile/rust-core/src/lib.rs:2715-2745`; `src/nube/auth_supabase.rs:22-24,105-129`; `supabase/config.toml` (`enable_signup = true`, `[auth.email] enable_confirmations = false`, `minimum_password_length = 6`)
- Estado: Nuevo (relacionado con el pendiente "autenticación" de `auditoria-integral-android-2026-09-09`)
- Descripción y evidencia: `login_supabase` llama a `nube::login(cedula, password)` (correo `cedula@brisas.local`) y, si responde bien, usa `resolver_identidad_local(&cedula)`. Nunca compara `sesion_supabase.usuario_id` con `usuarios.auth_user_id`: el catálogo ni siquiera descarga esa columna, y `SesionSupabase.usuario_id` no se usa. El ROOT sincronizado y los usuarios anteriores al backfill (`auth_user_id NULL`, según `admin-reset-password-usuario`) no tienen usuario en Auth.
- Escenario de impacto: si el proyecto productivo permite registro público por correo sin confirmación (es la configuración del `config.toml` versionado), un atacante con la publishable key (que es pública) hace `POST /auth/v1/signup {email:"<cedula_root>@brisas.local"}` y entra como ROOT en cualquier dispositivo donde ese ROOT figure con `SIN_PASSWORD_LOCAL`. Esto no se verificó contra producción.
- Referencia externa: Supabase, deshabilitar registros https://supabase.com/docs/guides/auth/general-configuration ; OWASP ASVS V2/V3 (binding de identidad) https://owasp.org/www-project-application-security-verification-standard/
- Recomendación: (1) verificar hoy en el dashboard que "Allow new users to sign up" está desactivado y la confirmación de correo activa; (2) sincronizar `auth_user_id` y exigir `sesion.usuario_id == auth_user_id` antes de abrir la sesión; (3) crear en Auth, desde `admin-create-usuario`, a todos los usuarios con `auth_user_id NULL`, incluido ROOT.

### [NS-06] El upsert por clave natural reescribe la PK `id` y rompe la integridad referencial entre dispositivos
- Severidad: Alta
- Categoría: Integridad de datos
- Ubicación: `src/nube/sincronizacion.rs:639-649,667-682` (contratistas), `:698-742` (empresas), `:139-158` (lotes); lado de recepción `:3736-3747` (`uuid = COALESCE(contratistas.uuid, excluded.uuid)`), `:3677-3700`
- Estado: Nuevo
- Descripción y evidencia: el cuerpo incluye `"id": uuid_local` y se envía con `on_conflict=identificacion` (o `nombre`) y `resolution=merge-duplicates`. PostgREST genera `ON CONFLICT (identificacion) DO UPDATE SET id = EXCLUDED.id, ...` para todas las columnas del payload. Si otro dispositivo creó la misma cédula con otro UUID (el caso real de los 117 duplicados), el UPDATE intenta cambiar la PK. Si ya hay `ingresos.contratista_id` que la referencian, falla con 23503. Si no, cambia de id en silencio y deja huérfanos los ingresos futuros del otro dispositivo. Al recibir, el dispositivo conserva su UUID local (`COALESCE`), así que nunca converge. Mismo efecto con `idx_empresas_nombre_plegado` (23505 ante "Dos Pinos" / "DOS PINOS") y con `empresas_proveedor.nombre`, que es único global aunque la RLS sea por sitio (403).
- Escenario de impacto: en un dispositivo, un contratista y todos sus ingresos agotan 20 reintentos y quedan en estado `fallido`. Esos movimientos nunca llegan a la nube: no hay historial compartido ni detección de gafete ocupado o conflictos, y el panel web queda incompleto.
- Referencia externa: PostgREST Upsert/on_conflict https://docs.postgrest.org/en/stable/references/api/tables_views.html#upsert
- Recomendación: no enviar `id` en el upsert por clave natural, o usar `columns=` sin `id`. Al recibir, adoptar el id remoto (`uuid = excluded.uuid` cuando coincide la clave natural) y remapear las referencias locales. Para `empresas_proveedor`, alinear la unicidad con el alcance por sitio (`(sitio_id, plegar_texto(nombre))`).

### [NS-07] Token de dispositivo de 12 h sin revalidación: revocar y suspender tardan hasta 12 h
- Severidad: Alta
- Categoría: Seguridad
- Ubicación: `supabase/functions/device-auth/index.ts:41,125-135,154-163`; todas las políticas RLS solo miran `jwt->>'sitio_id'`
- Estado: Pendiente de auditoría previa (A-04 recomendaba un TTL de 5-15 min o revalidación en vivo; después de esa auditoría el TTL subió de 1 h a 12 h)
- Descripción y evidencia: el comentario lo reconoce: "Un token ya emitido antes de suspender sigue válido hasta que expire". Ninguna política consulta `dispositivos.revoked_at/suspended_at`. Los clientes cachean el token (`application/nube.rs:856-921`, `mobile/rust-core` y `GuiState`). Tampoco lleva `aud`/`iss`/`jti`.
- Escenario de impacto: ante el robo de un equipo, el atacante que ya tiene un token conserva lectura global de usuarios y contratistas, escritura del sitio y el canal Realtime hasta 12 h después de la revocación.
- Referencia externa: Supabase JWT https://supabase.com/docs/guides/auth/jwts ; OWASP ASVS V3.3
- Recomendación: crear una función `private.dispositivo_vigente()` (`security definer`, `set search_path=''`, índice por PK) que verifique `revoked_at/suspended_at` para `auth.jwt()->>'sub'` cuando el JWT trae `sitio_id`, y agregarla con AND a las políticas de dispositivo. Otra opción es bajar el TTL a 15 min (el cliente ya reintenta ante 401).

### [NS-08] El chequeo "activo en otro sitio" nunca encuentra nada: la RLS lo impide
- Severidad: Media
- Categoría: Seguridad
- Ubicación: `src/nube/sincronizacion.rs:2287-2560`; políticas finales `ingresos` (`20260905161257:23-27`), `movimientos_visita` (`20260909204415`), `ingresos_proveedor` (`20260917040937`), `sitios` (`20260912015533`)
- Estado: Nuevo
- Descripción y evidencia: las consultas piden `ingresos?...&sitio_id=neq.<mío>&hora_salida=is.null&select=sitios(nombre)`, pero la política de lectura es `sitio_id = jwt.sitio_id or es_admin_global()`. Para un dispositivo siempre vuelve `[]`. Tampoco puede leer `sitios`. Las pruebas usan un servidor simulado, así que no lo detectan. Desktop lo usa como bloqueo (`comandos/ingresos.rs:40`, `proveedores.rs:164`, `citas.rs:111`) y como aviso tras sincronizar.
- Escenario de impacto: `pendientes.md:391` da por implementado el "bloqueo y aviso simétrico entre sitios", pero en producción nunca bloquea: la misma cédula puede estar activa en dos sitios. Es una falsa sensación de control.
- Referencia externa: Supabase RLS https://supabase.com/docs/guides/database/postgres/row-level-security
- Recomendación: exponer una RPC `security definer` que reciba cédulas y devuelva solo `(cedula, sitio_nombre)` de los ingresos abiertos en otros sitios. Agregar una prueba SQL con JWT de dispositivo.

### [NS-09] Marcas de agua compartidas entre tablas y sin traslape: se pierden actualizaciones del catálogo y de citas
- Severidad: Media
- Categoría: Integridad de datos
- Ubicación: `src/nube/sincronizacion.rs:3484-3604` (una sola `marca_mas_nueva` para empresas, contratistas, usuarios, rutas y empresas_proveedor), `:3704-3730` (contratistas omitidos igual avanzan la marca), `:3281-3355` (citas), `:4576-4668` (préstamos KOF sin paginar ni `order`)
- Estado: Nuevo
- Descripción y evidencia: `updated_at = now()` corresponde al inicio de la transacción. La marca es el máximo de todas las tablas, que se consultan en secuencia y con paginación por offset (`order=id.asc`, UUID aleatorio). Una fila de `contratistas` que cambia después de su GET pero antes que otra tabla más reciente queda por debajo de la nueva marca y no vuelve a pedirse. El historial de ingresos sí tiene un traslape de 7 días; el catálogo y las citas no. `recibir_historial_gafetes_provisionales_del_sitio` usa `obtener_json` sin `order` ni paginación: por encima de 1000 filas (`max_rows`) trunca y la marca salta al máximo.
- Escenario de impacto: la baja de acceso de un contratista (`activo=false`) hecha en el panel mientras un dispositivo sincroniza se pierde para siempre en ese dispositivo, que lo sigue dejando entrar sin conexión.
- Referencia externa: PostgREST paginación/Range https://docs.postgrest.org/en/stable/references/api/pagination_count.html
- Recomendación: una marca por tabla, paginación keyset `order=updated_at.asc,id.asc` con `(updated_at,id) > (marca,id)`, un traslape fijo (por ejemplo 5 min) y una reconciliación completa periódica de los flags de acceso (`activo`).

### [NS-10] Las citas no propagan bajas: visitantes o sitios quitados siguen autorizados en el dispositivo
- Severidad: Media (visitas en pausa)
- Categoría: Integridad de datos
- Ubicación: `src/nube/sincronizacion.rs:3200-3279` (solo upsert de `cita_visitantes`), `:3281` (filtro RLS por `cita_sitios`)
- Estado: Nuevo
- Descripción y evidencia: no hay lápidas (tombstones). Si el anfitrión borra un visitante, ese visitante no se elimina localmente. Si se borra la fila `cita_sitios` de un sitio, la cita deja de ser visible para ese dispositivo por RLS, y la copia local sigue `VIGENTE` hasta `fecha_hasta`.
- Escenario de impacto: una persona retirada de la cita todavía puede entrar por ese punto de acceso.
- Referencia externa: OWASP ASVS V4 (autorización coherente)
- Recomendación: usar borrado lógico (`cita_visitantes.activo`, `cita_sitios.retirado_en`) visible para el sitio afectado, o reconciliar el conjunto completo de citas vigentes del sitio.

### [NS-11] `cerrar_*_remoto` usa el reloj del sistema sin corregir y confirma cierres que no ocurrieron
- Severidad: Media
- Categoría: Integridad de datos
- Ubicación: `src/nube/sincronizacion.rs:4269-4303`, `:4305-4340`, `:4510-4544`
- Estado: Nuevo
- Descripción y evidencia: `"hora_salida": serializar_utc(chrono::Utc::now())` ignora `RelojCorregido` (el caso real de 11 min de desfase). Con `return=minimal` y 0 filas afectadas (ya cerrado por otro dispositivo, o un uuid ajeno), igual borra la caché y lo reporta como éxito. El plan de diseño pedía que "el segundo reciba un rechazo con el motivo".
- Escenario de impacto: horas de salida erróneas en la auditoría. El operador cree que registró la salida cuando en realidad la registró otro.
- Referencia externa: CWE-367 https://cwe.mitre.org/data/definitions/367.html
- Recomendación: recibir la hora del `AppCore` (reloj corregido), usar `return=representation` y devolver un error de conflicto si vuelve vacío.

### [NS-12] Metadata forense de `device-auth` sin validar y sobrescribible en cada renovación
- Severidad: Media
- Categoría: Seguridad
- Ubicación: `supabase/functions/device-auth/index.ts:74-99,171-187`
- Estado: Pendiente de auditoría previa (A-04: "validación estructural estricta, límite de body, rate limit")
- Descripción y evidencia: `identificador_hardware`, `nombre_dispositivo`, etc. se guardan sin validar tipo ni longitud, en cualquier llamada (no solo en la activación). No hay límite de tamaño del cuerpo ni rate limit. La actualización es fire-and-forget (`.then(()=>{})`) sin `EdgeRuntime.waitUntil`, y un error de base de datos responde 401 `invalid_credentials`.
- Escenario de impacto: quien tenga el secreto puede suplantar o borrar la huella del equipo legítimo (se pierde la evidencia) o inyectar texto enorme que el panel luego muestra.
- Referencia externa: Supabase background tasks https://supabase.com/docs/guides/functions/background-tasks ; OWASP API4:2023 https://owasp.org/API-Security/editions/2023/en/0xa4-unrestricted-resource-consumption/
- Recomendación: aceptar metadata solo si la fila no tiene metadata previa (activación) o registrarla en una tabla de eventos append-only. Validar tipos y longitudes (≤200), limitar el body (4 KB), usar `waitUntil` y responder 503 ante errores de base de datos.

### [NS-13] `debe_cambiar_password` vive en `user_metadata`, que el propio usuario puede modificar
- Severidad: Media
- Categoría: Seguridad
- Ubicación: `supabase/functions/admin-create-usuario/index.ts:92-97`; `admin-reset-password-usuario/index.ts:90-101`; `src/nube/auth_supabase.rs:78-82,182-185`
- Estado: Nuevo
- Descripción y evidencia: el cambio obligatorio se decide con `user_metadata.debe_cambiar_password`. Con su propio token, el usuario puede hacer `PUT /auth/v1/user {"data":{"debe_cambiar_password":false}}` sin cambiar la contraseña. El reset tampoco cierra las sesiones vigentes (no llama a `auth.admin.signOut`).
- Escenario de impacto: la contraseña temporal (compartida por WhatsApp o papel) queda como definitiva. Tras un reset por compromiso, el refresh token del atacante sigue siendo válido.
- Referencia externa: Supabase, user vs app metadata https://supabase.com/docs/guides/auth/managing-user-data
- Recomendación: guardar el flag en `app_metadata` (solo `service_role`) o en `usuarios`, y cerrar las sesiones del usuario al resetear.

### [NS-14] Autorización administrativa basada solo en `auth.email()`
- Severidad: Media (condicionada a la configuración de Auth)
- Categoría: Seguridad
- Ubicación: `private.es_admin_global()` (`20260905080325`), función `correoAdminAutorizado` duplicada en las 8 funciones `admin-*`, políticas de `citas`/`anfitriones` (`20260912015156`)
- Estado: Nuevo (relacionado con A-01/P1)
- Descripción y evidencia: basta con que coincida el correo del JWT. No se verifica `email_confirmed_at` ni el proveedor, ni se enlaza por `auth.uid()`. La inserción directa en `citas` (`"anfitrion crea sus propias citas"`) no exige `anfitriones.activo`, a diferencia de la RPC.
- Escenario de impacto: con registro abierto sin confirmación (o cambio de correo sin doble confirmación), una cuenta nueva con el correo de un admin o anfitrión que aún no tiene usuario en Auth obtiene sus privilegios. Un anfitrión dado de baja puede seguir creando citas por REST.
- Referencia externa: https://supabase.com/docs/guides/database/postgres/row-level-security#helper-functions
- Recomendación: guardar `auth_user_id` en `administradores_panel`/`anfitriones` y comparar con `auth.uid()`. Exigir `activo` en las políticas de inserción y actualización de citas. Unificar la verificación en `supabase/functions/_shared`.

### [NS-15] El step-up (OTP) de las operaciones sensibles solo existe en la interfaz
- Severidad: Media
- Categoría: Seguridad
- Ubicación: `supabase/functions/admin-revoke-device`, `admin-delete-device`, `admin-reset-password-usuario`, `admin-create-usuario`
- Estado: Pendiente de auditoría previa (P1 de `reporte-seguridad-web-2026-09-09`)
- Descripción y evidencia: ninguna función revisa `aal`, `amr` ni una prueba ligada a la operación (`grep aal|amr|reauth` sin resultados).
- Escenario de impacto: un token de admin robado crea usuarios ROOT o restablece contraseñas sin pasar el OTP.
- Referencia externa: https://supabase.com/docs/guides/auth/auth-mfa
- Recomendación: exigir `aal2` (MFA) o una prueba de un solo uso validada en el servidor.

### [NS-16] Chequeo de gafete ocupado y URLs PostgREST armadas con datos sin codificar
- Severidad: Media
- Categoría: Seguridad
- Ubicación: `src/nube/sincronizacion.rs:1834-1840`, `:2158-2168`, `:2375-2383`, `:4279-4282` (interpolación con `format!`)
- Estado: Nuevo
- Descripción y evidencia: las cédulas son texto libre (`contratista_service.rs:222` solo verifica que no esté vacía) y se meten en `eq.{cedula}` e `in.({cedulas})` sin comillas ni codificación. Un `#`, `&` o `)` corta o cambia el filtro. Para `usuario_sigue_activo_remoto`, "sin fila" significa activo (fail-open). Las listas `in.(...)` de todos los abiertos locales no tienen límite: con cientos de abiertos se llega a 414 o a un error, y como la llamada se hace con `?`, se cae toda la sincronización.
- Escenario de impacto: una cédula con caracteres reservados hace que la verificación de baja responda "activo" o anula la detección de conflictos. Un sitio con muchas personas adentro deja de sincronizar.
- Referencia externa: PostgREST, reserved characters https://docs.postgrest.org/en/stable/references/api/url_grammar.html#reserved-characters ; CWE-88
- Recomendación: construir la query con `reqwest` `.query(&[...])` y comillas dobles para `in.()`, partir las listas en bloques de ≤100, y usar fail-closed cuando la fila falta y el usuario no es ROOT local.

### [NS-17] Orquestación de la sincronización duplicada en 4 lugares, con deriva real
- Severidad: Media
- Categoría: Acoplamiento
- Ubicación: `src/application/nube.rs:313-376`; `desktop/src-tauri/src/comandos/nube.rs:224-326`; `mobile/rust-core/src/lib.rs:2796-2880` y `~2960-3000`
- Estado: Nuevo
- Descripción y evidencia: cada capa repite autenticar, armar `ContextoSincronizacion` y la secuencia de `recibir_*`. Las versiones difieren: la de `AppCore` no llama a `recibir_cierres_de_ingresos_propios_proveedor` ni a préstamos KOF, y ya hubo un bug documentado (`comandos/nube.rs:269-272`). La secuencia usa `?` en serie, así que un fallo en una función pausada (`recibir_citas_del_sitio`) impide la evaluación de `sesion_expulsada`. El patrón "cargar secreto + autenticar + contexto" se repite más de 20 veces. Las funciones de `AppCore` con `directorio` usan `cargar_secreto_en` sin identificador, el mismo bug que documenta `nube.rs:189-201`.
- Escenario de impacto: nuevas entidades que solo se sincronizan en una plataforma, y bajas que no expulsan la sesión si falla un módulo secundario.
- Referencia externa: https://martinfowler.com/bliki/BoundedContext.html
- Recomendación: un solo `ServicioSincronizacion` en el núcleo que reciba un `trait FuenteSecreto` y un `trait ProveedorToken` y ejecute pasos aislados (cada paso con su propio `Result`, y la verificación de sesión siempre al final).

### [NS-18] `sincronizacion.rs` (8140 líneas) mezcla cinco responsabilidades
- Severidad: Baja
- Categoría: Mala práctica
- Ubicación: `src/nube/sincronizacion.rs` (≈4670 líneas de producción y ≈3470 de pruebas)
- Estado: Nuevo
- Descripción y evidencia: contiene el transporte HTTP/PostgREST (paginación, errores), la cola de salida (lotes, backoff), la serialización por entidad (13 `construir_cuerpo_*`/`enviar_*` casi idénticos), la recepción incremental (8 variantes de "página + marca") y las consultas en vivo (gafete ocupado, conflictos). Hay tres copias de `recibir_cierres_*`, tres de `recibir_*_abiertos` y cuatro de `aplicar_pagina_*`.
- Recomendación: dividirlo en `nube/postgrest.rs` (cliente, paginación, `exigir_2xx`, codificación de filtros), `nube/cola/` (drenado + `trait EntidadSaliente { tabla, on_conflict, cuerpo(), cierre() }`), `nube/recepcion/` (un `trait EntidadEntrante` genérico con marca y página) y `nube/consultas_vivas.rs`. Las pruebas, a `tests/` por módulo. Así NS-01 y NS-03 quedan en un solo lugar.

### [NS-19] `on_conflict` de lote de gafetes no coincide con la restricción real
- Severidad: Baja
- Categoría: Rendimiento
- Ubicación: `src/nube/sincronizacion.rs:143` (`sitio_id,numero`) frente a `:817` (`sitio_id,numero,tipo`); `20260913203542` cambió la unicidad a `(sitio_id,numero,tipo)`
- Estado: Nuevo
- Descripción y evidencia: el lote de dos o más gafetes siempre falla con 42P10 y cae al envío fila por fila, lo que duplica las peticiones. También `usuario` sigue en `destino_lote` y `usuario_repository.rs:111,171` sigue encolando, pero la escritura está cerrada solo a `admin_global` (`20260912064233`): esas filas agotan 20 reintentos y quedan en `fallido`, y ensucian `contar_fallos_permanentes`.
- Recomendación: corregir `on_conflict` y dejar de encolar `usuario` (o descartarlo en origen).

### [NS-20] Rendimiento: el historial vuelve a descargar 7 días completos cada 2 minutos
- Severidad: Baja
- Categoría: Rendimiento
- Ubicación: `src/nube/sincronizacion.rs:481,2789-2797`
- Estado: Nuevo
- Descripción y evidencia: `marca - 7 días` se aplica siempre. En un sitio con 500 movimientos por día son unas 3500 filas por ciclo y por dispositivo (720 ciclos al día).
- Recomendación: usar un traslape corto (minutos) combinado con keyset (NS-09), y reservar los 7 días para una reconciliación diaria.

### [NS-21] Secreto de dispositivo: vuelve a texto plano en silencio y la clave portable se deriva de `ANDROID_ID`
- Severidad: Baja
- Categoría: Seguridad
- Ubicación: `src/nube/credenciales.rs:290-298,319-335,44-47`; `src/nube/mod.rs:51-78`
- Estado: Pendiente parcial de `auditoria-calidad-2026-09` (Android ya usa Keystore)
- Descripción y evidencia: si DPAPI falla, se guarda en claro sin avisar. El esquema portable usa `SHA-256(ANDROID_ID)` (identificador no secreto). En `%APPDATA%` (Roaming) el blob DPAPI viaja con el perfil del dominio. `CONTROL_ACCESO_SUPABASE_URL` acepta `http://`, lo que permitiría mandar el secreto en claro a un host arbitrario si alguien controla el entorno.
- Recomendación: fallar en lugar de degradar a texto plano, eliminar la escritura portable (dejar solo la lectura legada), exigir `https://` salvo en compilaciones de prueba y considerar `%LOCALAPPDATA%` separado para evitar el roaming.

### [NS-22] Mensajes internos filtrados y validación débil en las edge functions `admin-*`
- Severidad: Baja
- Categoría: Seguridad
- Ubicación: `admin-create-site/index.ts:~60` (`body.nombre?.trim()` sin comprobar el tipo), `detail: error.message` en todas las funciones `admin-*`; CORS `*`
- Estado: Pendiente de auditoría previa (A-07 y P2 de 2026-09-09)
- Recomendación: usar un esquema de entrada (zod), errores genéricos con `correlation_id`, una lista blanca de orígenes y una verificación compartida en `_shared/`.

### [NS-23] `public.plegar_texto` sin `search_path` fijo y expuesta como RPC
- Severidad: Baja
- Categoría: Seguridad
- Ubicación: `supabase/migrations/20260919021138_...sql:9-15`
- Estado: Nuevo
- Descripción y evidencia: es la única función nueva sin `set search_path` (lint 0011) y queda en `public` con EXECUTE para PUBLIC y anon (`/rest/v1/rpc/plegar_texto`).
- Referencia externa: https://supabase.com/docs/guides/database/database-linter?lint=0011_function_search_path_mutable
- Recomendación: `alter function public.plegar_texto(text) set search_path = ''` y revocar su ejecución a anon y authenticated (los índices no necesitan ese permiso).

### [NS-24] GRANTs por defecto amplios y columnas sensibles de `dispositivos` legibles por el sitio
- Severidad: Baja
- Categoría: Seguridad
- Ubicación: ninguna migración revoca los privilegios de tabla; política `"leer dispositivos (propio sitio o admin_global)"` (`20260912015533`)
- Estado: Pendiente de auditoría previa (A-02)
- Descripción y evidencia: cualquier dispositivo, incluido `visor`, lee `secret_hash`, `last_ip`, `identificador_hardware` de todos los dispositivos de su sitio. Todas las tablas conservan los GRANT ALL por defecto para anon y authenticated (la RLS está activa en las 20 tablas).
- Recomendación: `revoke select (secret_hash, last_ip, identificador_hardware, version_build) on dispositivos from authenticated`, o una vista mínima. Aplicar GRANTs mínimos por tabla.

### [NS-25] Código muerto en la capa de nube
- Severidad: Baja
- Categoría: Código muerto
- Ubicación: `src/application/nube.rs`: `sincronizar_con_nube` (313), `usuario_sigue_activo_remoto` (465), `refrescar_catalogo_sin_sesion` (285), `gafete_ocupado_en_sitio` / `gafete_provisional_ocupado_en_sitio` / `gafete_de_proveedor_ocupado_en_sitio` (686-774), `listar_historial_sitio` (55); `src/nube/auth_supabase.rs`: `obtener_jwks`, `verificar_token_offline`, `Jwk`, `SesionSupabase.usuario_id`; `src/application/autenticacion.rs:119` `fijar_password_inicial`
- Estado: Nuevo (`fijar_password_inicial` corresponde al pendiente de `auditoria-integral-android-2026-09-09`)
- Descripción y evidencia: `grep` en `desktop/`, `mobile/`, `web/`, `tests/` y `examples/`: no hay llamadores de producción. Desktop y mobile tienen sus propias versiones, y solo `tests/` usa `fijar_password_inicial`. `fijar_password_inicial` permite fijar la contraseña de cualquier cuenta sincronizada conociendo solo la cédula: es peligroso si alguien lo vuelve a conectar. `verificar_token_offline` no valida `iss`/`aud` y aceptaría también tokens de dispositivo (misma clave de firma).
- Recomendación: eliminarlos, o como mínimo `fijar_password_inicial`, junto con sus pruebas.

### [NS-26] Sin pruebas automatizadas de RLS ni de edge functions en CI; áreas sin cubrir
- Severidad: Media
- Categoría: Pruebas
- Ubicación: `supabase/tests/*.sql` (9 scripts manuales); `.github/workflows/*` (ninguno menciona supabase)
- Estado: Pendiente de auditoría previa (A-01, "Tests multi-tenant automatizados")
- Descripción y evidencia: no hay pruebas para `citas`/`cita_sitios`/`cita_visitantes`/`anfitriones`/`movimientos_visita`, rutas, proveedores, préstamos KOF ni para escrituras de `visor`. Las pruebas Rust usan servidores simulados, por eso NS-08 no se detectó.
- Recomendación: `supabase start` + `supabase test db` (pgTAP) en CI, con una matriz actor × tabla × operación (anon, visor, pc de A, pc de B, admin, anfitrión).

### [NS-27] Artefactos de entorno en migraciones
- Severidad: Info
- Categoría: Mala práctica
- Ubicación: `20260905194700_...sql:16-17` (JWT anon legado en el historial de git), `20260907031342:44` (URL de producción fija en `sync_access_policy()`)
- Estado: Nuevo
- Descripción y evidencia: la clave anon es pública, así que no es un secreto. En cambio, en staging el trigger llama a la función de producción. No se encontraron claves `service_role`/`sb_secret_` en el árbol ni en el historial (`git log -S`).
- Recomendación: leer la URL desde Vault o `current_setting` y rotar a claves publishable/secret nuevas.

### [NS-28] Estado final de RLS: resumen de verificación
- Severidad: Info
- Categoría: Seguridad
- Descripción: las 20 tablas tienen RLS. No hay políticas `to anon` (las de `rutas` no declaran rol, pero son inertes para anon). Las funciones `SECURITY DEFINER` (`private.es_admin_global`, `private.sitios_de_cita`, `private.anfitrion_de_cita`, `private.emitir_cambio_nube_sitio`, `sync_access_policy`) tienen `search_path` fijo y no se exponen por RPC. Realtime: SELECT de broadcast y presence acotado a `sitio:<jwt.sitio_id>` y sin política INSERT de broadcast, así que los clientes no pueden falsificar avisos.

---

## Aspectos bien resueltos
- TLS con `rustls` y raíces webpki, sin `danger_accept_invalid_certs`. Cliente único con timeout de 10 s.
- Secreto de dispositivo de alta entropía (2×UUIDv4), guardado solo como SHA-256. JWT ES256 con clave asimétrica. `Debug` redacta tokens y apikey.
- Triggers de inmutabilidad y de "salida única" en todas las tablas de movimientos. Cierre condicional `...=is.null`.
- Marca de agua tomada del `updated_at` del servidor (no del reloj local), redondeada hacia abajo al segundo. Recepción paginada con commit por página. Una fecha ilegible no bloquea el lote.
- Correcciones previas bien hechas: escritura de `usuarios` limitada a `admin_global`, helpers movidos a `private`, `sync-access-policy` con secreto propio en Vault y comparación en tiempo constante, alta y baja de `administradores_panel` retirada de la API, RPC `crear_cita_anfitrion` atómica, idempotente y `SECURITY INVOKER`.
