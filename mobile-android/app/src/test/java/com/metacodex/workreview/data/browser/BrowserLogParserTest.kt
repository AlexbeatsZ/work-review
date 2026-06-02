package com.metacodex.workreview.data.browser

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Test

class BrowserLogParserTest {
    @Test
    fun parsesValidPayload() {
        val result = BrowserLogParser().parse(
            """
            {
              "ts": 1780290000000,
              "event_type": "page_enter",
              "url": "https://example.com/page",
              "title": "Example",
              "duration_ms": 0,
              "source": "via_userscript"
            }
            """.trimIndent()
        )
        assertNull(result.error)
        assertNotNull(result.event)
        assertEquals("page_enter", result.event?.eventType)
        assertEquals("https://example.com/page", result.event?.url)
    }

    @Test
    fun rejectsMissingUrlWithoutThrowing() {
        val result = BrowserLogParser().parse("""{"ts":1,"event_type":"page_enter"}""")
        assertNull(result.event)
        assertEquals("missing or unsupported url", result.error)
    }
}
