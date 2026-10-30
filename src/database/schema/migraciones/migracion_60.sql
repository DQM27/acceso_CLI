-- Por qué quedó fallida una fila de `cola_salida` (incidente 2026-10-30: un
-- teléfono avisaba "28 registros no se pudieron subir" y casi todos eran
-- choques ya avisados en su momento, no datos perdidos):
--   conflicto  la nube la rechazó por una regla del negocio (el gafete o la
--              persona ya tenían algo abierto en la nube). Se avisa al
--              sincronizar como conflicto; no es una falla de sincronización.
--   error      agotó los reintentos o la nube la rechazó por otra causa:
--              eso sí no llegó y alguien tiene que mirarlo.
-- Nulo mientras la fila no esté fallida.
ALTER TABLE cola_salida ADD COLUMN motivo_fallo TEXT
    CHECK (motivo_fallo IN ('conflicto', 'error'));

-- Las fallidas de antes se clasifican por el texto que dejó la nube: un 409
-- de uno de los índices de "gafete activo" o "persona activa" (todos se
-- llaman `*_activo_*idx` o `*_activa_idx`, ver `gafete_activo_de` e
-- `indice_persona_activa_de` en src/nube/sincronizacion/cola.rs).
UPDATE cola_salida
SET motivo_fallo = CASE
    WHEN ultimo_error LIKE 'El receptor respondió 409:%'
         AND (ultimo_error LIKE '%\_activo\_%idx%' ESCAPE '\'
              OR ultimo_error LIKE '%\_activa\_idx%' ESCAPE '\')
        THEN 'conflicto'
    ELSE 'error'
END
WHERE estado = 'fallido';
