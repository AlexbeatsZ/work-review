package com.metacodex.workreview.server

import android.util.Base64
import com.metacodex.workreview.WorkReviewRepository
import com.metacodex.workreview.data.browser.BrowserLogParser
import fi.iki.elonen.NanoHTTPD
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import java.net.URLDecoder
import java.nio.charset.StandardCharsets

class BrowserLogServer(
    private val repository: WorkReviewRepository,
    port: Int = 17890,
    private val parser: BrowserLogParser = BrowserLogParser()
) : NanoHTTPD("127.0.0.1", port) {
    private val scope = CoroutineScope(Dispatchers.IO)

    override fun serve(session: IHTTPSession): Response {
        if (session.method == Method.OPTIONS) {
            return cors(newFixedLengthResponse(Response.Status.NO_CONTENT, "text/plain", ""))
        }
        if (session.method == Method.GET && (session.uri == "/log" || session.uri == "/log.gif")) {
            val encoded = session.parameters["d"]?.firstOrNull()
                ?: return cors(newFixedLengthResponse(Response.Status.BAD_REQUEST, "text/plain", "missing d"))
            val body = runCatching {
                String(
                    Base64.decode(URLDecoder.decode(encoded, StandardCharsets.UTF_8.name()), Base64.URL_SAFE or Base64.NO_WRAP),
                    StandardCharsets.UTF_8
                )
            }.getOrElse {
                return cors(newFixedLengthResponse(Response.Status.BAD_REQUEST, "text/plain", "bad payload"))
            }
            return handleBody(body)
        }

        if (session.method != Method.POST || session.uri != "/log") {
            return cors(newFixedLengthResponse(Response.Status.NOT_FOUND, "text/plain", "not found"))
        }

        val files = mutableMapOf<String, String>()
        return runCatching {
            session.parseBody(files)
            val body = files["postData"] ?: ""
            handleBody(body)
        }.getOrElse {
            cors(newFixedLengthResponse(Response.Status.INTERNAL_ERROR, "application/json", """{"ok":false,"error":"server error"}"""))
        }
    }

    private fun handleBody(body: String): Response {
        val parsed = parser.parse(body)
        val event = parsed.event ?: return cors(
            newFixedLengthResponse(Response.Status.BAD_REQUEST, "application/json", """{"ok":false,"error":"${parsed.error}"}""")
        )
        scope.launch { repository.insertBrowserEvent(event) }
        return cors(newFixedLengthResponse(Response.Status.OK, "application/json", """{"ok":true}"""))
    }

    private fun cors(response: Response): Response =
        response.apply {
            addHeader("Access-Control-Allow-Origin", "*")
            addHeader("Access-Control-Allow-Methods", "POST, OPTIONS")
            addHeader("Access-Control-Allow-Headers", "Content-Type")
        }
}
