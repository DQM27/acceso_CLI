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

            tiene_acceso INTEGER NOT NULL CHECK (
                tiene_acceso IN (0, 1)
            ),

            FOREIGN KEY (empresa_id)
                REFERENCES empresas(id)
        );

        CREATE INDEX IF NOT EXISTS idx_contratistas_empresa
        ON contratistas(empresa_id);
        ",
    )?;

    Ok(())
}