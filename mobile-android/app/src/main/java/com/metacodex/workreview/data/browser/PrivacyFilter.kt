package com.metacodex.workreview.data.browser

import com.metacodex.workreview.data.db.BrowserEventEntity
import java.net.URI

data class PrivacySettings(
    val browserLoggingEnabled: Boolean = true,
    val domainOnly: Boolean = false,
    val blockedDomains: Set<String> = emptySet()
)

class PrivacyFilter(private val settings: PrivacySettings) {
    fun apply(event: BrowserEventEntity): BrowserEventEntity? {
        if (!settings.browserLoggingEnabled) return null
        val uri = runCatching { URI(event.url) }.getOrNull() ?: return null
        val host = uri.host?.lowercase() ?: return null
        if (settings.blockedDomains.any { host == it || host.endsWith(".$it") }) return null
        if (!settings.domainOnly) return event
        val redactedUrl = "${uri.scheme}://$host/"
        return event.copy(url = redactedUrl, title = null, referrer = null)
    }
}
