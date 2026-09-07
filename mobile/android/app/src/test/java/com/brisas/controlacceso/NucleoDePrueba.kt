package com.brisas.controlacceso

import java.io.File
import java.sql.DriverManager
import uniffi.control_acceso_mobile.Nucleo

/// Abre un [Nucleo] de prueba sobre un archivo `SQLite` temporal, con el
/// esquema real y fixtures sembradas — mismo patrón que los tests de
/// `mobile/rust-core/src/lib.rs` (`open_database` + `execute_batch` + recién
/// ahí abrir el `Nucleo` real), adaptado a Kotlin porque `Nucleo` no expone
/// ningún método para insertar datos sin autenticarse primero — y en una
/// base recién creada todavía no existe ningún usuario con el cual
/// autenticarse.
///
/// Requiere `mobile/rust-core` compilado para el host (no para Android) —
/// el propio `build.gradle.kts` de este módulo hace `cargo build --release`
/// antes de correr los tests, así que `./gradlew test` alcanza solo.
object NucleoDePrueba {
    /// Hash Argon2 real de la contraseña `"clave_prueba_123"` — mismo valor
    /// que usan los tests de `mobile/rust-core/src/lib.rs`, nunca texto
    /// plano.
    const val HASH_CLAVE_PRUEBA =
        "\$argon2id\$v=19\$m=19456,t=2,p=1\$pO+/qvY8ieaUA97ME2LUPQ\$OfE/070ufOj4TtL2SzVyW3sefnJjrMJq32APEHrM/wI"

    const val CLAVE_PRUEBA = "clave_prueba_123"

    /// `seedSql`: sentencias `INSERT` crudas para sembrar fixtures (mismo
    /// esquema que usa la app real) — se ejecutan en orden, después de que
    /// el propio `Nucleo.abrir` haya creado las tablas.
    fun abrir(archivoTemporal: File, vararg seedSql: String): Nucleo {
        val ruta = archivoTemporal.absolutePath

        // Abrir y cerrar de una crea el esquema (mismas migraciones que
        // AppCore::abrir en Rust) sin dejar la conexión tomada — hace falta
        // liberarla antes de que JDBC abra la suya para sembrar.
        Nucleo.abrir(ruta).close()

        DriverManager.getConnection("jdbc:sqlite:$ruta").use { conexion ->
            conexion.createStatement().use { sentencia ->
                seedSql.forEach { sentencia.execute(it) }
            }
        }

        return Nucleo.abrir(ruta)
    }

    /// Sentencia lista para sembrar un usuario Root con [CLAVE_PRUEBA] —
    /// el caso más común, la mayoría de los tests necesitan autenticarse
    /// como alguien.
    fun sqlUsuarioRoot(cedula: String = "999999999", nombre: String = "Actor Test"): String =
        """
        INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES (
            '$cedula', '$nombre', '$HASH_CLAVE_PRUEBA', 'ROOT', 1
        );
        """.trimIndent()
}
