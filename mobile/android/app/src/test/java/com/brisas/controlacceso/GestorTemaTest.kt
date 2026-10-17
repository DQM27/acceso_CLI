package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class GestorTemaTest {
    @Test
    fun `solo hay dos temas y se alternan`() {
        assertEquals(2, TemaApp.entries.size)
        assertEquals(TemaApp.TOKYO_NIGHT, TemaApp.TOKYO_DAY.siguiente())
        assertEquals(TemaApp.TOKYO_DAY, TemaApp.TOKYO_NIGHT.siguiente())
    }

    @Test
    fun `lo guardado con los temas actuales se respeta`() {
        assertEquals(TemaApp.TOKYO_DAY, temaGuardado("TOKYO_DAY", oscuroViejo = null))
        assertEquals(TemaApp.TOKYO_NIGHT, temaGuardado("TOKYO_NIGHT", oscuroViejo = null))
    }

    @Test
    fun `claro y oscuro de antes pasan a su equivalente Tokyo`() {
        assertEquals(TemaApp.TOKYO_DAY, temaGuardado("CLARO", oscuroViejo = null))
        assertEquals(TemaApp.TOKYO_NIGHT, temaGuardado("OSCURO", oscuroViejo = null))
    }

    @Test
    fun `el booleano mas viejo tambien se conserva`() {
        assertEquals(TemaApp.TOKYO_NIGHT, temaGuardado(null, oscuroViejo = true))
        assertEquals(TemaApp.TOKYO_DAY, temaGuardado(null, oscuroViejo = false))
    }

    @Test
    fun `sin nada guardado se sigue al sistema`() {
        assertNull(temaGuardado(null, oscuroViejo = null))
        assertNull(temaGuardado("otra cosa", oscuroViejo = null))
    }
}
