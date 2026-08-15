package com.metacodex.workreview.data

import android.content.Context
import android.net.Uri
import androidx.documentfile.provider.DocumentFile
import com.metacodex.workreview.data.db.WorkReviewDatabase
import java.io.File

class ExportWriter(private val context: Context) {
    suspend fun exportAll(exportDirectoryUri: String? = null): ExportResult {
        val files = buildExportFiles()
        if (!exportDirectoryUri.isNullOrBlank()) {
            val uri = Uri.parse(exportDirectoryUri)
            val dir = DocumentFile.fromTreeUri(context, uri)
            if (dir != null && dir.canWrite()) {
                files.forEach { file ->
                    dir.findFile(file.name)?.delete()
                    val target = dir.createFile(file.mimeType, file.name)
                        ?: error("Cannot create ${file.name}")
                    context.contentResolver.openOutputStream(target.uri)?.use { output ->
                        output.write(file.bytes)
                    } ?: error("Cannot open ${file.name}")
                }
                return ExportResult("uri:${dir.uri}", files.map { it.name })
            }
        }

        val dir = File(context.getExternalFilesDir(null), "exports").apply { mkdirs() }
        files.forEach { file ->
            File(dir, file.name).writeBytes(file.bytes)
        }
        return ExportResult(dir.absolutePath, files.map { it.name })
    }

    private suspend fun buildExportFiles(): List<ExportFile> {
        val dao = WorkReviewDatabase.get(context).dao()
        val appSessions = buildString {
                appendLine("id,start_ts,end_ts,duration_ms,package_name,app_label,source,confidence")
                dao.allAppSessions().forEach {
                    appendLine(listOf(it.id, it.startTs, it.endTs, it.durationMs, it.packageName, it.appLabel ?: "", it.source, it.confidence).toCsv())
                }
            }.toByteArray()
        val activities = buildString {
                appendLine("id,timestamp,app_name,window_title,screenshot_path,ocr_text,category,duration,browser_url,executable_path,semantic_category,semantic_confidence,intent_purpose,intent_note,intent_start_timestamp,intent_end_timestamp,intent_completed_at")
                dao.allActivities().forEach {
                    appendLine(
                        listOf(
                            it.id,
                            it.timestamp,
                            it.appName,
                            it.windowTitle,
                            it.screenshotPath,
                            it.ocrText ?: "",
                            it.category,
                            it.duration,
                            it.browserUrl ?: "",
                            it.executablePath ?: "",
                            it.semanticCategory ?: "",
                            it.semanticConfidence ?: "",
                            it.intentPurpose ?: "",
                            it.intentNote ?: "",
                            it.intentStartTimestamp ?: "",
                            it.intentEndTimestamp ?: "",
                            it.intentCompletedAt ?: ""
                        ).toCsv()
                    )
                }
            }.toByteArray()
        val dbBytes = context.getDatabasePath(DB_FILE)
            .takeIf { it.exists() }
            ?.readBytes()
            ?: ByteArray(0)
        return listOf(
            ExportFile(APP_SESSIONS_CSV, "text/csv", appSessions),
            ExportFile(ACTIVITIES_CSV, "text/csv", activities),
            ExportFile(DB_FILE, "application/octet-stream", dbBytes)
        )
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

    companion object {
        const val APP_SESSIONS_CSV = "app_sessions.csv"
        const val ACTIVITIES_CSV = "activities.csv"
        const val DB_FILE = "work_review_mobile.db"
    }
}

data class ExportResult(
    val destination: String,
    val fileNames: List<String>
)

private data class ExportFile(
    val name: String,
    val mimeType: String,
    val bytes: ByteArray
)
