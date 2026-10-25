# El equipo fue retirado o la sesión se cerró sola

## Para qué sirve

Entender por qué el programa cerró la sesión sin que nadie lo pidiera, o
dejó de enviar datos, y qué hacer en cada caso.

## Quién puede hacerlo

Cualquier usuario. En algunos casos hace falta un administrador.

## Caso 1 — "Su usuario inició sesión en otra unidad"

**Mensaje**: "Su usuario inició sesión en otra unidad: se cerró la sesión
en este equipo."

**Qué pasó**: alguien inició sesión con **su** usuario en **otro sitio**.
Un usuario sólo puede tener la sesión abierta en un sitio a la vez (en
varios equipos del mismo sitio sí se puede). Gana el último que entró.

**Qué hacer**:

1. Si fue usted (por ejemplo, entró en otro sitio y se olvidó de cerrar
   aquí), no hay nada que hacer.
2. Si **no** fue usted, alguien conoce su contraseña: inicie sesión de
   nuevo, [cambie su contraseña](cambiar-contrasena.md) y avise a un
   administrador.

Lo registrado antes del cierre no se pierde.

## Caso 2 — "Su usuario fue desactivado"

**Mensaje**: "Su usuario fue desactivado — se cerró la sesión."

**Qué pasó**: un administrador desactivó su usuario en el panel web.

**Qué hacer**: hable con un administrador. Hasta que lo reactiven, no
podrá iniciar sesión ("Usuario inactivo" o "Este usuario fue
desactivado").

## Caso 3 — "Este equipo fue retirado en el panel"

**Mensaje** (queda fijo en pantalla, con el botón **Reconectar este
equipo**): "Este equipo fue retirado en el panel y ya no sincroniza. El
trabajo local sigue disponible; para volver a conectarlo, pida a un
administrador un código de reconexión."

También puede aparecer al sincronizar: "La nube ya no reconoce a este
dispositivo (fue retirado en el panel) -- pida a un administrador un
código de reconexión".

**Qué pasó**: un administrador retiró este equipo en el panel web.

**Qué hacer**:

1. Puede seguir registrando en este equipo, pero **nada se envía**: el
   otro equipo del sitio y el panel no lo ven.
2. Avise **de inmediato** a un administrador. Si hay que volver a usar el
   equipo, le da un **código de reconexión** desde el panel.
3. Pulse **Reconectar este equipo**, escriba el código y pulse
   **Reconectar**.
   - Si el equipo **sigue en la misma unidad**, verá "Equipo reconectado
     en (unidad)." y lo que registró mientras estuvo retirado se envía.
     No se borra nada.
   - Si el código lo pasa **a otra unidad**, se borran los datos de la
     unidad anterior. Si hay registros que nunca llegaron a enviarse,
     antes avisa cuántos se van a perder; pulse **Reconectar y borrar**
     para seguir o **Cancelar** para no hacer nada. Después pulse
     **Reiniciar ahora**: el programa se reinicia, baja los datos de la
     unidad nueva y vuelve al inicio de sesión.

El código sirve **una sola vez**, sólo para este equipo, y vence en pocos
minutos.

| Mensaje | Qué significa | Qué hacer |
|---|---|---|
| "El código de reconexión no es válido o ya venció -- pida uno nuevo en el panel" | El código está mal escrito, ya se usó, venció, o es un código de alta (de equipo nuevo) y no de reconexión. | Pida un código de reconexión nuevo a un administrador. |
| "Este equipo no está retirado: no hace falta reconectarlo" | El equipo ya está conectado. | Nada. |
| "Este equipo se está cambiando de unidad: reinicie la app para terminar" | Se reconectó en otra unidad y falta reiniciar. | Cierre el programa y vuelva a abrirlo. |

## Caso 4 — La sesión se cerró sin ningún mensaje

Si el programa vuelve a la pantalla de inicio sin aviso, probablemente
alguien pulsó **Ctrl+Q** o **Cerrar sesión**. Vuelva a iniciar sesión.

## Relacionado

- [Iniciar sesión](iniciar-sesion.md)
- [Cambiar mi contraseña](cambiar-contrasena.md)
- [Vincular el equipo por primera vez](vincular-equipo.md)
