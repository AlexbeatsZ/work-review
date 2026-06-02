package com.metacodex.workreview.server

import com.metacodex.workreview.WorkReviewRepository
import com.metacodex.workreview.data.browser.BrowserLogParser
import fi.iki.elonen.NanoHTTPD
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

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
        if (session.method != Method.POST || session.uri != "/log") {
            return cors(newFixedLengthResponse(Response.Status.NOT_FOUND, "text/plain", "not found"))
        }

        val files = mutableMapOf<String, String>()
        return runCatching {
            session.parseBody(files)
            val body = files["postData"] ?: ""
            val parsed = parser.parse(body)
            val event = parsed.event ?: return cors(
                newFixedLengthResponse(Response.Status.BAD_REQUEST, "application/json", """{"ok":false,"error":"${parsed.error}"}""")
            )
            scope.launch { repository.insertBrowserEvent(event) }
            cors(newFixedLengthResponse(Response.Status.OK, "application/json", """{"ok":true}"""))
        }.getOrElse {
            cors(newFixedLengthResponse(Response.Status.INTERNAL_ERROR, "application/json", """{"ok":false,"error":"server error"}"""))
        }
    }

    private fun cors(response: Response): Response =
        response.apply {
            addHeader("Access-Control-Allow-Origin", "*")
            addHeader("Access-Control-Allow-Methods", "POST, OPTIONS")
            addHeader("Access-Control-Allow-Headers", "Content-Type")
        }
}
