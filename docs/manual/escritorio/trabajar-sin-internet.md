# Trabajar sin internet y sincronizar a mano

## Para qué sirve

Saber qué pasa cuando el equipo se queda sin internet y cómo enviar los
registros pendientes en cuanto vuelve la conexión. **El puesto de control
nunca se detiene por falta de internet**: los ingresos y salidas se
registran igual y se envían después.

## Quién puede hacerlo

Cualquier usuario con sesión abierta.

## Dónde

En la **barra de estado**, abajo a la derecha:

- Un **punto de color** con el estado de la conexión.
- El botón **Sincronizar**.

## Qué significa el estado de la conexión

| Indicador | Qué significa |
|---|---|
| Verde, **"En línea"** | Todo normal: lo que registra el otro equipo del sitio aparece aquí al instante. |
| Gris, **"Conectando…"** | El programa recién abrió y está conectándose. |
| Rojo parpadeante, **"Sin conexión — modo offline"** | El equipo no tiene internet. |
| Rojo parpadeante, **"Sin conexión en vivo"** | Hay internet, pero los avisos instantáneos no llegan (por ejemplo, el servidor está caído). Los datos se siguen enviando cada pocos minutos. |

Al perder la conexión aparece **"Sin conexión a internet — trabajando en
modo offline."**; al recuperarla, **"Conexión restablecida."**

## Qué se puede hacer sin internet

- Registrar **ingresos y salidas** de todas las clases, como siempre.
- Consultar **Activos**, **Historial**, contratistas, empresas y gafetes
  con lo último que se recibió.
- Iniciar sesión, si ya entró antes en este equipo con internet.

## Qué no se puede hacer sin internet

- Iniciar sesión por **primera vez** en este equipo.
- **Cambiar la contraseña**.
- **Vincular** el equipo.
- Ver al instante lo que registra el otro equipo del sitio (llega al
  volver la conexión).
- Confirmar con otros sitios si una persona ya está adentro allá. Por eso,
  al volver la conexión puede aparecer un aviso de ingreso duplicado: ver
  ["Tiene un ingreso activo aquí Y en otro sitio"](ingreso-activo-en-otro-sitio.md).

## Pasos — sincronizar a mano

El programa envía y recibe solo: en cuanto vuelve la conexión, y además
cada pocos minutos. Pulse **Sincronizar** si quiere asegurarse en ese
momento, por ejemplo antes de cerrar el turno o después de un corte largo.

1. Pulse **Sincronizar**, abajo a la derecha. Mientras trabaja dice
   **Sincronizando…**.
2. Verá **"Sincronizado — N enviados."**

## Mensajes que pueden aparecer

| Mensaje | Qué significa | Qué hacer |
|---|---|---|
| "Sincronizado — N enviados." | Todo se envió bien. | Nada. |
| "N enviados, M fallidos — reintenta más tarde." | Algunos registros no se pudieron enviar. Siguen guardados en el equipo. | Vuelva a pulsar **Sincronizar** más tarde. Si se repite durante horas, avise a un administrador. |
| "No se pudo conectar con la nube, intente de nuevo" | No hay internet o el servidor no responde. | Revise la conexión y vuelva a intentar. Mientras tanto, siga trabajando normal. |
| "El receptor rechazó el pedido, intente de nuevo más tarde" | El servidor no aceptó el envío en ese momento. | Intente más tarde. Si se repite, avise a un administrador. |
| "Este dispositivo todavía no está vinculado a la nube" | El equipo nunca se vinculó. | Ver [Vincular el equipo](vincular-equipo.md). |
| "Esta versión de la app ya no es compatible -- actualice la app para seguir sincronizando" | El programa es demasiado viejo para enviar datos. | Actualice: ver [Buscar e instalar una actualización](buscar-actualizacion.md). |

## Relacionado

- ["Tiene un ingreso activo aquí Y en otro sitio"](ingreso-activo-en-otro-sitio.md)
- [El equipo fue retirado o la sesión se cerró sola](equipo-retirado-o-sesion-cerrada.md)
- ["Revise la fecha y hora del equipo"](fecha-y-hora-del-equipo.md)
