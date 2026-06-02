package com.metacodex.workreview

import android.content.Context
import androidx.work.CoroutineWorker
import androidx.work.WorkerParameters
import com.metacodex.workreview.data.ExportWriter

class AutoExportWorker(
    context: Context,
    params: WorkerParameters
) : CoroutineWorker(context, params) {
    override suspend fun doWork(): Result =
        runCatching {
            val repository = WorkReviewRepository(applicationContext)
            if (repository.autoExportEnabled()) {
                ExportWriter(applicationContext).exportAll()
                repository.scheduleNextTimedAutoExport()
            }
        }.fold(
            onSuccess = { Result.success() },
            onFailure = { Result.retry() }
        )
}
