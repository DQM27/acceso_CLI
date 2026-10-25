# Reconexión de un equipo retirado — plan

Estado: **plan, sin código** (2026-10-24). Decisiones del dueño del
2026-10-24.

## Problema

Cuando un administrador retira un equipo en el panel, el equipo deja de
sincronizar y muestra "Este equipo fue retirado en el panel…". Hoy la única
forma de volver a conectarlo es **borrar la base local o desinstalar la
app** y vincularlo como nuevo. Con eso se pierde lo que el equipo registró
y no llegó a subir, y vuelve como un equipo distinto, sin su historia.

Es la mejora 8 de `docs/manual/README.md`.

## Decisiones (dueño, 2026-10-24)

- El botón **"Reconectar este equipo"** sólo existe en un equipo **retirado
  por el panel**. En los demás equipos no aparece.
- Sólo funciona con un **código de reconexión** emitido en el panel **para
  ese equipo**. Sin código válido no pasa nada.
- **Misma unidad operativa**: no se borra nada. Sigue el mismo registro de
  dispositivo, con sus datos y accesos. Sólo cambia el nombre si en el panel
  le ponen otra etiqueta.
- **Otra unidad operativa**: se borra la base local y se baja la de la
  unidad nueva.
- **Registros sin subir al cambiar de unidad** (opción A): el equipo avisa
  "Hay N registros sin subir que se van a perder" y pide confirmación antes
  de borrar. La pérdida queda registrada: cuántos registros y quién
  confirmó.

## Flujo

1. **Panel → Dispositivos**, fila de un equipo **Retirado** → **Autorizar
   reconexión**.
   - Se eligen la unidad operativa (por defecto, la misma) y la etiqueta
     (por defecto, la misma).
   - Se confirma con el código por correo de siempre (`ConfirmacionSensible`).
   - El panel muestra el código **una sola vez** (`CodigoVinculacionEmitido`).
     Vence en 10 minutos.
2. **En el equipo**, el aviso fijo de equipo retirado trae el botón
   **Reconectar este equipo**. Pide el código.
3. **Consulta sin gastar el código**: el núcleo manda el código con
   `confirmar: false`. El servidor lo valida y responde a qué unidad va, sin
   marcarlo como usado.
   - El núcleo compara esa unidad con la suya.
   - Si **cambia de unidad y hay registros sin subir**, devuelve
     `NecesitaConfirmacion { pendientes, unidad }`.
   - La pantalla muestra el aviso y espera la confirmación.
4. **Canje**: el núcleo estrena un **par de claves nuevo** y manda el código
   con `confirmar: true` y la clave pública nueva. Si hubo aviso, manda
   también `pendientes_descartados` y quién confirmó. El servidor hace todo
   de una vez (ver la función `canjear_codigo_reconexion` más abajo).
5. **Después del canje**:
   - **Misma unidad**: el equipo guarda el token, deja de estar "retirado" y
     corre una sincronización completa. Lo pendiente, incluido lo registrado
     mientras estuvo retirado, sube; si algo choca, lo atrapan los índices
     únicos, como en cualquier registro hecho sin conexión.
   - **Otra unidad**: el núcleo borra la base local, conservando sólo la
     clave nueva del equipo, y baja el catálogo de la unidad nueva, igual que
     un equipo recién vinculado. Vuelve al inicio de sesión.

## Por qué no se puede manipular

- **El equipo no decide nada**: el botón sólo manda el código. Que el
  equipo vuelva a estar vigente lo decide el servidor.
- **Hacen falta dos cosas**: una cuenta de administrador del panel, con su
  confirmación por correo, y el equipo físico. Copiar la base o la carpeta
  de la app a otra PC no sirve, porque la clave privada no está ahí y
  además se estrena una nueva.
- **Código atado al equipo**: es de un solo uso, vence en 10 minutos y se
  guarda como hash. Sólo vale para ese `dispositivo_id`, y sólo si el
  equipo **sigue retirado**. No sirve para dar de alta un equipo nuevo, y
  un código de alta no sirve para reconectar.
- **Evidencia**: cada intento fallido queda en
  `eventos_seguridad_dispositivos` con IP, hora y datos del equipo, igual
  que en el alta. Lo mismo la autorización (qué administrador la dio) y el
  canje (unidad anterior y nueva, registros descartados, quién confirmó).
  Ver [plan-sesion-unica-dispositivos.md](plan-sesion-unica-dispositivos.md).

## Cambios por capa

### Supabase (migración nueva, con su batería en `supabase/tests/`)

- `codigos_vinculacion`:
  - `proposito text not null default 'alta' check (proposito in ('alta', 'reconexion'))`;
  - `sitio_destino_id uuid references sitios(id)` y `etiqueta_destino text`
    (sólo para reconexión).
- `canjear_codigo_vinculacion` (alta) agrega `and c.proposito = 'alta'`:
  un código de reconexión no da de alta.
- Función nueva `consultar_codigo_reconexion(p_codigo_hash)`: devuelve
  dispositivo, sitio actual, sitio destino y etiqueta destino si el código
  es de reconexión, está vigente y sin usar, y el equipo sigue retirado. No
  cambia nada.
- Función nueva `canjear_codigo_reconexion(p_codigo_hash, p_clave_publica_jwk,
  p_clave_huella, p_metadata, p_ip, p_detalle)`, atómica igual que la de alta:
  - marca el código usado sólo si es de reconexión, está vigente, sin usar y
    sin anular, y el equipo tiene `revoked_at is not null`;
  - en `dispositivos`: `revoked_at = null`, clave nueva, `vinculado_en`,
    `sitio_id` y `etiqueta` del destino, y la metadata del equipo;
  - registra el evento `reconexion_canjeada` con
    `{sitio_anterior, sitio_nuevo, pendientes_descartados, confirmado_por}`;
  - devuelve `dispositivo_id, sitio_id, tipo, cambio_de_unidad`.
- `eventos_seguridad_dispositivos.tipo`: se suman `reconexion_autorizada` y
  `reconexion_canjeada`.
- **Permisos** (según `AGENTS.md`): las funciones, `grant execute … to
  service_role` sólo, como la de alta. Nada a `anon` ni a `authenticated`.
- Revisar que retirar el equipo otra vez (`admin-revoke-device`) siga
  anulando los códigos sin usar, incluidos los de reconexión. Ya lo hace.

### Edge Functions

- **`admin-autorizar-reconexion`** (sólo administradores,
  `correoAdminAutorizado`):
  - exige que el equipo esté retirado y que exista la unidad destino;
  - anula los códigos anteriores sin usar de ese equipo y emite uno nuevo
    (mismo largo y formato que el de alta, vigencia de 10 minutos);
  - registra `reconexion_autorizada` con el correo del administrador.
- **`device-reconectar`** (pública, como `device-vincular`, porque el equipo
  retirado no tiene credencial vigente):
  - `confirmar: false` → `consultar_codigo_reconexion`;
  - `confirmar: true` → `canjear_codigo_reconexion` y emite el primer token,
    con la misma forma que `device-auth`;
  - cualquier rechazo responde lo mismo (`codigo_invalido`) y el motivo real
    va a `eventos_seguridad_dispositivos` con la IP.
- Pruebas Deno de las dos.

### Núcleo (`src/`)

- **Estado "retirado" guardado**: cuando `device-auth` responde que no
  reconoce la clave (`CredencialesInvalidas`), el núcleo lo anota en la base
  local. Así el botón aparece también después de reiniciar la app. La marca
  se borra al reconectar.
- `AppCore::equipo_retirado() -> bool`, para que la pantalla sepa si mostrar
  el botón. La regla es del núcleo.
- `AppCore::reconectar_equipo(codigo, confirmacion: Option<…>) ->
  ResultadoReconexion`, con tres resultados:
  - `Reconectado`, si es la misma unidad;
  - `NecesitaConfirmacion { pendientes, unidad }`;
  - `ReconectadoEnOtraUnidad`, si se borró y se bajó la base nueva.
- El núcleo cuenta los registros sin subir (la cola de sincronización) y
  decide cuándo hace falta confirmar. Se niega a reconectar si el equipo no
  está retirado.
- Borrado al cambiar de unidad: vaciar la base local y conservar sólo la
  clave nueva del equipo, por el mismo camino que un equipo recién vinculado
  (`requiere_configuracion_inicial` → catálogo de la unidad).
- Mensajes en `src/mensajes.rs`, en "usted".
- **Pruebas**:
  - misma unidad: conserva todo y lo pendiente queda para subir;
  - otra unidad sin pendientes: borra sin preguntar;
  - otra unidad con pendientes: pide confirmación y no borra hasta tenerla;
  - equipo no retirado: se niega;
  - código inválido: no toca nada.

### Escritorio

- `MENSAJE_EXPULSION` pasa a decir: "…Para volver a conectarlo, pida a un
  administrador un código de reconexión."
- Junto al aviso, el botón **Reconectar este equipo** (sólo si
  `equipo_retirado()`), que abre una ventana para el código, con el mismo
  formato de grupos que **Vincular este equipo**.
- Si el núcleo pide confirmación, la ventana muestra: "Hay N registros sin
  subir que se van a perder. ¿Reconectar en (unidad)?" con **Cancelar** y
  **Reconectar y borrar**.

### Móvil

- Lo mismo, por UniFFI: `equipo_retirado`, `reconectar_equipo` y el
  resultado. Kotlin sólo dibuja el botón, la entrada del código y la
  confirmación.
- `ExpulsionNube.MENSAJE` con el texto nuevo.

### Panel web

- **Dispositivos**: en la fila de un equipo **Retirado**, la acción
  **Autorizar reconexión**. Abre una ventana con unidad y etiqueta, pasa por
  `ConfirmacionSensible` y muestra el código emitido.

### Manual

- `escritorio/equipo-retirado-o-sesion-cerrada.md`,
  `movil/telefono-retirado-o-sesion-cerrada.md` y `panel/dispositivos.md`:
  pasos de la reconexión y sus mensajes.
- `README.md`: la mejora 8, marcada como resuelta.

## Despliegue

1. Migración y Edge Functions. La migración se aplica sola al mergear a
   `main`; antes se prueba en staging contra el esquema de producción.
2. Panel (`wrangler deploy`).
3. Escritorio y móvil en su próximo release. Una versión vieja nunca
   muestra el botón, así que convivir no tiene riesgo.

## Fuera de este plan

- Reconectar un equipo que **no** fue retirado: no hace falta, sigue
  conectado.
- Mover un equipo **vigente** a otra unidad: primero se retira y después se
  reconecta en la unidad nueva.
- Cambiar el tipo de equipo (PC, teléfono, visor).
