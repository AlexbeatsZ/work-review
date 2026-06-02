package com.metacodex.workreview.data.browser

import com.metacodex.workreview.data.db.BrowserEventEntity
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

@Serializable
data class BrowserLogPayload(
    val ts: Long? = null,
    @SerialName("event_type") val eventType: String? = null,
    val url: String? = null,
    val title: String? = null,
    val referrer: String? = null,
    val visible: Boolean? = null,
    @SerialName("duration_ms") val durationMs: Long? = null,
    @SerialName("user_agent") val userAgent: String? = null,
    val source: String? = null
)

data class BrowserLogParseResult(
    val event: BrowserEventEntity?,
    val error: String?
)

class BrowserLogParser(
    private val json: Json = Json {
        ignoreUnknownKeys = true
        isLenient = true
    }
) {
    fun parse(body: String): BrowserLogParseResult {
        val payload = runCatching { json.decodeFromString<BrowserLogPayload>(body) }
            .getOrElse { return BrowserLogParseResult(null, "invalid JSON: ${it.message}") }

        val ts = payload.ts ?: return BrowserLogParseResult(null, "missing ts")
        val eventType = payload.eventType?.takeIf { it.isNotBlank() }
            ?: return BrowserLogParseResult(null, "missing event_type")
        val url = payload.url?.takeIf { it.startsWith("http://") || it.startsWith("https://") }
            ?: return BrowserLogParseResult(null, "missing or unsupported url")

        return BrowserLogParseResult(
            event = BrowserEventEntity(
                ts = ts,
                url = url,
                title = payload.title,
                referrer = payload.referrer,
                eventType = eventType,
                durationMs = payload.durationMs?.takeIf { it >= 0 },
                source = payload.source ?: "via_userscript"
            ),
            error = null
        )
    }
}
