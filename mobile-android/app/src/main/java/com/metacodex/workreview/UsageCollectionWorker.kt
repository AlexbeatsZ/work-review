package com.metacodex.workreview

import android.content.Context
import androidx.work.CoroutineWorker
import androidx.work.WorkerParameters

class UsageCollectionWorker(
    context: Context,
    params: WorkerParameters
) : CoroutineWorker(context, params) {
    override suspend fun doWork(): Result =
        runCatching {
            WorkReviewRepository(applicationContext).collectUsageNow()
        }.fold(
            onSuccess = { Result.success() },
            onFailure = { Result.retry() }
        )
}
