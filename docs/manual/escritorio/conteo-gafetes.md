# Conteo de gafetes

## Para qué sirve

Comprobar rápido, por ejemplo al recibir el turno, que estén todos los
gafetes que deberían estar. No hace falta ordenarlos: se van tomando en el
orden en que estén.

## Quién puede hacerlo

Cualquier usuario con sesión abierta en el escritorio.

## Dónde

**Gafetes**, en el menú lateral: elija el tipo (Contratista, Visita,
Provisional KOF o Proveedor) y, arriba a la izquierda, cambie de **Lista** a
**Conteo** (ícono de lista con marcas).

## Cómo se lee el tablero

Cada gafete del tipo es una ficha con su número:

| Ficha | Qué significa |
|---|---|
| Gris | Se espera y falta contarlo. |
| Verde | Contado. |
| Borde punteado | Asignado a alguien que está adentro: no se espera. Al pasar el mouse dice a quién. |
| Rayada | Perdido: no se espera. |
| Ámbar | Se contó, pero figura asignado o perdido: hay que revisarlo. |

Arriba se ve cuántos van contados, cuántos faltan y una barra de avance.

## Pasos

1. Tome un gafete, escriba su número y pulse **Enter**. Póngalo aparte para
   no volver a contarlo. El campo se limpia y espera el siguiente; la ficha
   se pone verde.
2. Al escribir cada número, debajo del campo aparece un aviso:
   - **"07 anotado."**: todo bien.
   - **"El 07 ya está anotado."**: ese número ya estaba anotado; no se
     cuenta dos veces.
   - **"El 99 no existe en el catálogo."**: no se anota. Revise el número.
   - **"El 05 figura perdido…"**: se anota igual (ficha ámbar). Revise que
     esté bien digitado; si lo está, el gafete apareció y hay que
     recuperarlo.
   - **"El 04 figura asignado a Fulano (adentro)…"**: se anota igual (ficha
     ámbar). Revise que esté bien digitado; si lo está, falta registrar la
     salida de esa persona.
3. Si anotó uno por error, haga clic en su ficha para quitarlo.
4. Si hay algo que mirar, a la derecha aparece **Revisar**. Ahí sale, por
   ejemplo, un **posible error de digitación al entregar**: el 06 figura
   asignado a Fulano y se contó, y falta el 09. Lo más probable es que al
   registrar a Fulano se digitara 06 pero se le entregara el 09. Revise qué
   gafete tiene puesto. Los registros no se corrigen; el aviso sirve para
   saber dónde está cada gafete.
5. Con el filtro de arriba a la derecha puede ver **Todos**, solo los que
   **Faltan** o solo los **Contados**.
6. Al terminar, pulse **Terminar conteo**. Lo que falta queda en rojo y el
   tablero muestra solo los faltantes. Si se le pasó uno, pulse **Seguir
   contando**, escríbalo y vuelva a terminar.

Con muchos gafetes, el tablero los agrupa por centenas (001–100, 101–200…).

El conteo **no guarda nada**: lo que haya que corregir se hace desde la
**Lista** (reportar perdido, recuperar), con su reporte.

## Relacionado

- [Gafete perdido, pagado o recuperado](gafete-perdido.md)
- [Gafetes: dar de alta y consultar](gafetes-alta-y-consulta.md)
