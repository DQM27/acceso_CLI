# Sesión única y presencia de dispositivos — plan (borrador, sin código todavía)

> Documento de continuidad para retomar esta conversación en otra sesión.
> Nace de una prueba manual del usuario: el mismo secreto de dispositivo
> funciona en más de un dispositivo a la vez. Nada de esto está
> implementado — es la base para decidir, sesión por sesión, igual que
> `plan-persistencia-nube.md` y `plan-panel-administrativo-web.md`.

## Por qué existe este documento

Sesión de planificación (sin tocar código) donde se discutieron tres
huecos relacionados en el flujo actual de activación de dispositivos:

1. Hoy no hay forma de saber qué dispositivos están conectados en este
   momento ni cuántas conexiones hay activas.
2. El secreto que se envía al dispositivo para activarlo es reutilizable:
   el mismo secreto sirve para activar más de un dispositivo, sin límite.
3. No hay una regla definida para cuando dos dispositivos, tras operar
   offline, reclaman la misma identidad al reconectar.

## Decisiones ya tomadas (explícitas del usuario)

### 1. El secreto de dispositivo es de un solo uso, sin excepción

No es una credencial reutilizable. Se consume en el primer login exitoso
y queda atado a ese dispositivo para siempre. No hay "reutilización
legítima" — si hay que re-vincular un dispositivo, se emite un secreto
nuevo.

### 2. Reutilización en línea → bloquear, no expulsar, y guardar evidencia

Cuando alguien intenta usar un secreto ya consumido mientras el sistema
puede verificarlo en el momento (hay conectividad):

- Se **bloquea el segundo intento**. El dispositivo original (dueño
  legítimo del secreto) no se toca.
- Se **notifica al administrador**, incluyendo el nombre/identidad del
  dispositivo dueño legítimo del secreto.
- Se **captura toda la metadata posible** del intento (IP, timestamp,
  cualquier dato del dispositivo que intenta el registro) en un registro
  persistente — no solo una alerta efímera. El objetivo explícito es
  poder usar esto como evidencia para denunciar por intento de fraude si
  hace falta.
- Canal de la alerta: por correo si el SMTP de Supabase ya está operando
  (el usuario cree que sí; queda pendiente de confirmar — ver
  `docs/pendientes.md`, sección Seguridad y nube). Si no, alerta dentro
  de la app como plan mínimo de V1.

### 3. Identidad canónica del dispositivo vive en Supabase

No hay alta previa del dispositivo por parte del admin — el admin genera
el secreto sin saber todavía a qué dispositivo va a llegar. El
intercambio real es: **la nube envía el secreto (código), el dispositivo
lo ingresa y a cambio manda sus propios datos** (identificador único de
hardware + metadata). Ahí, en ese intercambio, nace la identidad.

Esos datos quedan en una **tabla maestra de identidad** en Supabase —
separada del secreto y de la sesión — con:

- Fecha de alta = fecha del primer consumo exitoso del secreto (no hace
  falta un registro previo aparte), para poder auditar "cuándo revisar".
- Toda la metadata posible capturada del dispositivo en ese intercambio.
- Esta tabla es la fuente de verdad de "quién es quién".

**Mecanismo de reconocimiento de secretos usados:** una tabla de
"secretos usados" (secreto + qué dispositivo lo consumió + fecha/hora)
es el mecanismo en sí — reconocer un reuso es solo comparar si el
secreto ya existe ahí. Si no está, es la primera vez (se inserta y nace
la identidad); si ya está, es un intento de fraude (ver punto 2). Como
el dispositivo ya mandó sus datos en el intercambio antes de ser
rechazado, esos datos igual se aprovechan para el registro forense del
intento.

### 4. El secreto NO se descarta — vive cifrado en el dispositivo como
credencial permanente, junto al ID

Corrección explícita del usuario a una primera versión de este punto
(que proponía descartar el secreto tras la activación): **el ID del
dispositivo por sí solo no es secreto** — viaja en cada request, se
puede observar o extraer. Si el sistema confiara solo en el ID, alguien
que lo robara podría plantarlo en otro dispositivo y suplantar al
original. El secreto es lo que hace que el ID no se pueda falsificar sin
también robar la credencial.

Modelo correcto: es un par permanente, no una activación de un solo
disparo.

- **El ID** es el "quién dice ser" (público, observable).
- **El secreto** es la prueba de que realmente es quien dice ser — vive
  **cifrado en el dispositivo para siempre** (el trabajo ya hecho con
  Android Keystore en `SecretoDispositivoStore.kt`, y el ítem ya cerrado
  en `pendientes.md` de "proteger el secreto con Keystore", siguen
  siendo exactamente correctos — no se tiran, no cambian).
- Toda sesión y toda renovación de sesión se valida presentando el
  **par (ID + secreto)**, no solo el ID. En la práctica esto puede
  seguir emitiendo un token de sesión de corta duración para no mandar
  el secreto crudo en cada llamada — pero ese token nace de validar el
  par, y se puede volver a pedir en cualquier momento presentando el
  mismo par (esto es justo lo que pasa al reconectar tras estar
  offline, punto 5).

"Un solo uso" no significa "se descarta después de usarlo" — significa
**"un solo dispositivo puede quedar vinculado a ese secreto para
siempre"**. La primera vez que se presenta, se ata permanentemente a ese
dispositivo (nace la identidad, punto 3). De ahí en adelante ese mismo
par es la credencial recurrente de sesión. Si alguien intenta vincular
ese secreto a un dispositivo *distinto* (una activación nueva), ahí sí
sigue aplicando el bloqueo + evidencia + alerta del punto 2 — eso no
cambia.

**Rotación del secreto en cada uso (patrón estándar, tipo refresh token
de OAuth):** si el secreto nunca cambiara, un flag de "ya usado" no
serviría de nada — bloquearía también al dueño legítimo la próxima vez
que lo presente, porque el servidor no puede distinguir su uso legítimo
repetido del uso de alguien que robó el par completo. La solución es que
**cada vez que el par (ID + secreto) se valida con éxito, el servidor
entrega un secreto nuevo que reemplaza al anterior**, y el viejo queda
invalidado para siempre. Así:

- El dispositivo legítimo siempre tiene el secreto más reciente — nunca
  choca con su propio historial, porque cada uso se lo renueva.
- Si alguien robó el par y lo usa **después** de que el legítimo ya
  rotó (lo normal, dado el uso continuo), el secreto robado ya quedó
  viejo/invalidado → se detecta y bloquea igual que el punto 2.
- Si lo usa **antes** de que el legítimo alcance a rotar (carrera casi
  simultánea), aplica el mecanismo de conflicto ya definido (bloqueo en
  línea o desempate por fecha si fue offline, punto 5).

### 5. Caso offline → desempate por fecha de alta, expulsión automática

Cuando el conflicto **no** se puede resolver en el momento (sin
conectividad) y dos dispositivos terminan reclamando la misma identidad,
al reconectar:

- Gana el dispositivo con la **fecha de alta más temprana** en la tabla
  maestra (criterio simple, a prueba de manipulación — no "más usado",
  solo "más viejo por fecha de registro").
- El dispositivo perdedor es **expulsado del grupo de inmediato y de
  forma automática**, sin esperar revisión manual.

Esto **no contradice** la regla del punto 2 — son dos escenarios
distintos: en línea el conflicto es solo sospecha y se bloquea sin
expulsar; offline el conflicto ya llegó confirmado (dos dispositivos
operaron creyéndose legítimos) y ahí sí corresponde expulsión automática.

### 6. Panel de presencia en tiempo real

Con sesión propia por dispositivo (punto 4), se puede construir un panel
que muestre qué dispositivos están conectados ahora y cuántas conexiones
hay activas — apoyado en presence de Realtime o un heartbeat simple.
Esto depende de que el punto 4 exista primero; construirlo antes sería
maquillaje sobre un problema de seguridad sin resolver.

## Orden de implementación

0. Identidad canónica del dispositivo (tabla maestra en Supabase).
1. Secreto de un solo uso + registro forense de intentos + alerta al
   admin (caso en línea).
2. Emisión de sesión (token de corta duración) validando el par
   ID + secreto — el secreto no se descarta, sigue viviendo cifrado en
   el dispositivo para poder renovar sesión cuando haga falta.
3. Panel de presencia en tiempo real (depende de 0 y 2).
4. Regla de desempate offline + expulsión automática (depende de 0；
   puede ir en paralelo con 1-3 una vez que la tabla maestra exista).

## Preguntas abiertas

- ¿El SMTP de Supabase ya está operando de verdad? No se pudo confirmar
  por las herramientas MCP disponibles (la config de Auth/SMTP vive en
  el dashboard, no expuesta por esas herramientas). Falta verificación
  directa en el dashboard de Supabase.
- ¿Qué metadata exacta del dispositivo es capturable hoy desde el
  cliente Android/rust-core? Falta inventariar qué campos ya viajan y
  cuáles habría que agregar para que el registro forense sea útil como
  evidencia real.
