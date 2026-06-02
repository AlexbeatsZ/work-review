package com.metacodex.workreview.data

import android.content.Context
import com.metacodex.workreview.data.db.WorkReviewDatabase
import java.io.File

class ExportWriter(private val context: Context) {
    suspend fun exportAll(): File {
        val dir = File(context.getExternalFilesDir(null), "exports").apply { mkdirs() }
        val dao = WorkReviewDatabase.get(context).dao()
        File(dir, "app_sessions.csv").writeText(
            buildString {
                appendLine("id,start_ts,end_ts,duration_ms,package_name,app_label,source,confidence")
                dao.allAppSessions().forEach {
                    appendLine(listOf(it.id, it.startTs, it.endTs, it.durationMs, it.packageName, it.appLabel ?: "", it.source, it.confidence).toCsv())
                }
            }
        )
        File(dir, "browser_events.csv").writeText(
            buildString {
                appendLine("id,ts,browser_package,url,title,referrer,event_type,duration_ms,source")
                dao.allBrowserEvents().forEach {
                    appendLine(listOf(it.id, it.ts, it.browserPackage, it.url, it.title ?: "", it.referrer ?: "", it.eventType, it.durationMs ?: "", it.source).toCsv())
                }
            }
        )
        context.getDatabasePath("work_review_mobile.db")
            .takeIf { it.exists() }
            ?.copyTo(File(dir, "work_review_mobile.db"), overwrite = true)
        return dir
    }

    private fun List<Any>.toCsv(): String =
        joinToString(",") { value ->
            val text = value.toString()
            if (text.any { it == ',' || it == '"' || it == '\n' }) {
                "\"${text.replace("\"", "\"\"")}\""
            } else {
                text
            }
        }
}
