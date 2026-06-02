package com.metacodex.workreview.data.browser

import com.metacodex.workreview.data.db.BrowserEventEntity
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PrivacyFilterTest {
    private val event = BrowserEventEntity(
        ts = 1,
        url = "https://sub.example.com/path?q=secret",
        title = "Title",
        referrer = "https://ref.example/",
        eventType = "page_enter",
        durationMs = 0
    )

    @Test
    fun blocksConfiguredDomains() {
        val filter = PrivacyFilter(PrivacySettings(blockedDomains = setOf("example.com")))
        assertNull(filter.apply(event))
    }

    @Test
    fun redactsToDomainOnly() {
        val filter = PrivacyFilter(PrivacySettings(domainOnly = true))
        val redacted = filter.apply(event)
        assertEquals("https://sub.example.com/", redacted?.url)
        assertNull(redacted?.title)
        assertNull(redacted?.referrer)
    }
}
