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

## Caso 2 — "Tu usuario fue desactivado"

**Mensaje**: "Tu usuario fue desactivado — se cerró la sesión."

**Qué pasó**: un administrador desactivó su usuario en el panel web.

**Qué hacer**: hable con un administrador. Hasta que lo reactiven, no
podrá iniciar sesión ("Usuario inactivo" o "Este usuario fue
desactivado").

## Caso 3 — "Este equipo fue retirado en el panel"

**Mensaje** (queda fijo en pantalla hasta cerrarlo): "Este equipo fue
retirado en el panel y ya no sincroniza. El trabajo local sigue
disponible; para volver a usar la nube hay que registrarlo como
dispositivo nuevo."

También puede aparecer al sincronizar: "La nube ya no reconoce a este
dispositivo (fue retirado en el panel) -- registralo como dispositivo
nuevo con un código del panel".

**Qué pasó**: un administrador quitó este equipo de la lista de equipos
del sitio en el panel web.

**Qué hacer**:

1. Puede seguir registrando en este equipo, pero **nada se envía**: el
   otro equipo del sitio y el panel no lo ven.
2. Avise **de inmediato** a un administrador. Si fue un error, hay que
   volver a registrar el equipo con un código nuevo.

Cómo volver a registrar un equipo ya usado **todavía está por definirse**:
lo resuelve un administrador. Este apartado se completa cuando esté
listo.

## Caso 4 — La sesión se cerró sin ningún mensaje

Si el programa vuelve a la pantalla de inicio sin aviso, probablemente
alguien pulsó **Ctrl+Q** o **Cerrar sesión**. Vuelva a iniciar sesión.

## Relacionado

- [Iniciar sesión](iniciar-sesion.md)
- [Cambiar mi contraseña](cambiar-contrasena.md)
- [Vincular el equipo por primera vez](vincular-equipo.md)
