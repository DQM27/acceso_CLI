use rusqlite::Connection;

pub fn initialize_database(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS empresas (
            id INTEGER PRIMARY KEY,
            nombre TEXT NOT NULL UNIQUE
        );

        CREATE TABLE IF NOT EXISTS usuarios (
            id INTEGER PRIMARY KEY,
            cedula TEXT NOT NULL UNIQUE,
            nombre TEXT NOT NULL,
            password_hash TEXT NOT NULL,

            rol TEXT NOT NULL CHECK (
                rol IN (
                    'ROOT',
                    'ADMINISTRADOR',
                    'OPERADOR'
                )
            ),

            activo INTEGER NOT NULL CHECK (
                activo IN (0, 1)
            )
        );

        CREATE TABLE IF NOT EXISTS contratistas (
            id INTEGER PRIMARY KEY,
            cedula TEXT NOT NULL UNIQUE,
            nombre TEXT NOT NULL,
            empresa_id INTEGER NOT NULL,

            tipo_ingreso TEXT NOT NULL CHECK (
                tipo_ingreso IN (
                    'PRAIND',
                    'IN_HOUSE',
                    'POR_CORREO',
                    'SWAT'
                )
            ),

            fecha_vencimiento_praind TEXT,

            es_personal_ruta INTEGER NOT NULL DEFAULT 0 CHECK (
                es_personal_ruta IN (0, 1)
            ),

            tiene_acceso INTEGER NOT NULL CHECK (
                tiene_acceso IN (0, 1)
            ),

            FOREIGN KEY (empresa_id)
                REFERENCES empresas(id)
        );

        CREATE INDEX IF NOT EXISTS idx_contratistas_empresa
        ON contratistas(empresa_id);

        CREATE TABLE IF NOT EXISTS registro_ingresos (
            id INTEGER PRIMARY KEY,

            contratista_id INTEGER NOT NULL,
            empresa_id INTEGER NOT NULL,

            fecha_hora_ingreso TEXT NOT NULL,

            medio_ingreso TEXT NOT NULL CHECK (
                medio_ingreso IN (
                    'CAMINANDO',
                    'VEHICULO'
                )
            ),

            tipo_ingreso TEXT NOT NULL CHECK (
                tipo_ingreso IN (
                    'PRAIND',
                    'IN_HOUSE',
                    'POR_CORREO',
                    'SWAT'
                )
            ),

            gafete_numero INTEGER,

            usuario_ingreso_id INTEGER NOT NULL,

            fecha_hora_salida TEXT,
            usuario_salida_id INTEGER,

            FOREIGN KEY (contratista_id)
                REFERENCES contratistas(id),

            FOREIGN KEY (empresa_id)
                REFERENCES empresas(id),

            FOREIGN KEY (usuario_ingreso_id)
                REFERENCES usuarios(id),

            FOREIGN KEY (usuario_salida_id)
                REFERENCES usuarios(id)
        );

        CREATE INDEX IF NOT EXISTS idx_registro_ingresos_contratista
        ON registro_ingresos(contratista_id);

        CREATE INDEX IF NOT EXISTS idx_registro_ingresos_empresa
        ON registro_ingresos(empresa_id);

        CREATE INDEX IF NOT EXISTS idx_registro_ingresos_fecha_ingreso
        ON registro_ingresos(fecha_hora_ingreso);

        CREATE INDEX IF NOT EXISTS idx_registro_ingresos_gafete
        ON registro_ingresos(gafete_numero);

        CREATE UNIQUE INDEX IF NOT EXISTS idx_registro_ingresos_gafete_activo
        ON registro_ingresos(gafete_numero)
        WHERE gafete_numero IS NOT NULL
          AND fecha_hora_salida IS NULL;
        ",
    )?;

    Ok(())
}