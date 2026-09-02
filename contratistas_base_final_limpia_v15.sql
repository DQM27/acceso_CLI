-- Base limpia de contratistas para Control de Acceso / schema v15
-- Generada a partir de la depuración del histórico y del catálogo normalizado.
--
-- REGLAS FINALES:
--  * Solo se cargan PRAIND, IN_HOUSE y SWAT.
--  * Todo personal de BAC se clasifica como SWAT.
--  * PRAIND/IN_HOUSE sin fecha conocida usan 2027-05-30 y es_personal_ruta = 1.
--  * SWAT usa fecha_vencimiento_praind = NULL y es_personal_ruta = 0.
--  * No contiene historial, filas históricas, cédulas fuente ni banderas de auditoría.
--  * No inserta registros en registro_ingresos.

PRAGMA foreign_keys = ON;

-- ============================================================
-- EMPRESAS UTILIZADAS POR LOS CONTRATISTAS FINALES
-- ============================================================
INSERT INTO empresas (nombre, activo) VALUES ('ALDAMA', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('ARGUEDAS', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('ASOSI S.A', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('AVANTIA FIRE Y SECURITY', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('BAC', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('BELCA', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('BLUE SATELLITE', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('BROOMDAY', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('CANFER', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('CAPACITA', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('CAPACITACIÓN DE BRIGADA.', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('CIASA', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('COM EL ORBE', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('COSEI', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('DYG', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('EBS', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('ECOS', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('ENTREVISTA', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('EXPENIC', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('INTEGRACOM', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('K-9 INTERNACIONAL', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('KOF', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('LIMPIEZAS SRL', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('LLANTAS EXPRES', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('M BRISAS', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('MEDICO', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('MULTIPRO S.A.', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('NAVEGACION', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('ORBE', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('PROVISA', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('RAICES MERCADEO', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('RENTOKILL', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('SCR', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('SIGMA', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('SODEXO', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('STERICLEAN DE CENTRO AMERICA', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('SUPRA CONTINENTAL INC', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('TALLER GERSON', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('TECMAS GUATEMALA S.A', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('TRACTOMOTRIZ', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('TRAUMA STORE', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('TRUCKSLOGIC', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('VALERIA', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
INSERT INTO empresas (nombre, activo) VALUES ('WARDIAN', 1) ON CONFLICT(nombre) DO UPDATE SET activo = 1;
