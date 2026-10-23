# "Revise la fecha y hora del equipo"

## Para qué sirve

Entender por qué el programa no deja registrar ingresos ni salidas y
muestra **"Revise la fecha y hora del equipo antes de continuar"**.

## Por qué pasa

El reloj del equipo marca una hora **anterior** al último ingreso o salida
que este mismo equipo registró. Por ejemplo: a las 10:00 se registró una
salida, y ahora el reloj del equipo dice 9:30. Si se dejara registrar, el
historial quedaría con horas imposibles (salidas antes de entradas), así
que el programa lo bloquea.

El programa **no depende del reloj de Windows**: usa una hora neutra,
tomada del servidor, y se corrige solo cada vez que tiene internet. Por
eso el aviso casi siempre aparece en un equipo que estuvo **sin
conexión** mientras alguien **cambió la fecha u hora de Windows** (o el
reloj se reinició porque falló la batería interna del equipo).

## Quién puede hacerlo

Cualquier usuario. No hace falta tocar la configuración de Windows.

## Dónde

Aparece al registrar un ingreso o una salida de cualquier clase
(contratista, proveedor, visita por correo, gafete KOF).

## Pasos

1. Revise que el equipo tenga **internet** (abajo a la derecha debe decir
   **En línea**).
2. Pulse **Sincronizar**. Con la conexión, el programa toma la hora del
   servidor y se corrige solo.
3. Vuelva a intentar el registro.
4. Si sigue igual, avise a un administrador.

**No** cambie la fecha ni la hora de Windows para "arreglar" un registro:
no hace falta y puede empeorar el problema.

Mientras el reloj esté mal, **no se puede registrar nada** en este equipo.
Si hay gente esperando, use el otro equipo del sitio (la PC o el celular)
si lo hay.

## Mensajes que pueden aparecer

| Mensaje | Qué significa | Qué hacer |
|---|---|---|
| "Revise la fecha y hora del equipo antes de continuar" | La hora del equipo quedó antes del último registro. | Siga los pasos de arriba. |
| "La salida no puede ser anterior al ingreso" | La hora de la salida queda antes de la de entrada. Suele tener la misma causa. | Siga los pasos de arriba. |

## Relacionado

- [Trabajar sin internet y sincronizar a mano](trabajar-sin-internet.md)
- [Registrar una salida](registrar-salida.md)
