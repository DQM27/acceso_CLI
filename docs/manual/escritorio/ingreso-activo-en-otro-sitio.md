# "Tiene un ingreso activo acá Y en otro sitio"

## Para qué sirve

Entender y resolver el aviso que aparece cuando una misma persona figura
adentro **en este sitio y en otro sitio a la vez**. Una persona no puede
estar en dos lugares; uno de los dos registros está mal.

## Por qué pasa

Casi siempre, porque este equipo estuvo **sin internet**: se registró el
ingreso aquí sin poder confirmar que la persona seguía adentro en otro
sitio (por ejemplo, porque allá no le registraron la salida). Al volver la
conexión, el programa lo detecta y avisa.

Con internet esto no ocurre: el programa no deja registrar el ingreso y
muestra "El contratista ya tiene un ingreso activo en (sitio)." Ver
["El contratista ya tiene un ingreso activo"](contratista-ya-tiene-ingreso-activo.md).

## Quién puede hacerlo

Cualquier usuario con sesión abierta, coordinando con el otro sitio.

## Dónde

El aviso aparece en la esquina de la pantalla después de sincronizar. Se
repite en cada sincronización **hasta que se resuelva**.

## Pasos

1. Lea el aviso: dice el nombre de la persona y el otro sitio.
2. Confirme dónde está la persona realmente (¿está aquí, frente a usted, o
   ya se fue?).
3. Si la persona **está aquí**: el registro equivocado es el del otro
   sitio. Avise a ese sitio (o a un administrador) para que le registren la
   salida allá.
4. Si la persona **ya no está aquí**: registre su salida en este equipo
   con **Ctrl+S**.
5. Cuando uno de los dos ingresos se cierre, el aviso deja de aparecer.

Para avisar al otro sitio use el medio que tengan acordado (por ejemplo,
**WhatsApp**); si no tiene cómo contactarlo, avise a un administrador.

## Mensajes que pueden aparecer

| Mensaje | Clase |
|---|---|
| "(nombre) tiene un ingreso activo acá Y en (sitio) — hay que resolverlo." | Contratista |
| "(nombre) tiene un ingreso de proveedor activo acá Y en (sitio) — hay que resolverlo." | Proveedor |
| "(nombre) tiene un ingreso por correo activo acá Y en (sitio) — hay que resolverlo." | Visita por correo |
| "(nombre) tiene una visita activa acá Y en (sitio) — hay que resolverlo." | Visita agendada |

En todos los casos se resuelve igual: con los pasos de arriba.

## Gafete usado en el otro equipo del sitio

Parecido, pero con un gafete: este equipo, sin internet, entregó un gafete
que el otro equipo del sitio ya tenía entregado. Aquí no hay duda de cuál
está mal: **el de este equipo** no quedó registrado en la nube.

**Mensaje** (ejemplo con un contratista): "El ingreso de (nombre) con
gafete (número) (hora) no quedó registrado en la nube — otro dispositivo
de este sitio ya lo tiene asignado." Existe la misma forma para
proveedores, visitas por correo, visitas agendadas y gafetes provisionales
KOF.

**Qué hacer**: revise con el otro equipo quién tiene realmente ese gafete
y avise a un administrador.

El manejo de estos conflictos **todavía está en construcción**: por ahora
el aviso sólo informa, y el programa no ofrece una forma de corregirlo.
Este apartado se completa cuando esté listo.

## Relacionado

- [Trabajar sin internet y sincronizar a mano](trabajar-sin-internet.md)
- [Registrar una salida](registrar-salida.md)
- ["El contratista ya tiene un ingreso activo"](contratista-ya-tiene-ingreso-activo.md)
