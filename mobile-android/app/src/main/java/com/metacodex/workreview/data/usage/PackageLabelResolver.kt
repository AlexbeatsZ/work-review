package com.metacodex.workreview.data.usage

import android.content.Context
import android.content.pm.PackageManager
import com.metacodex.workreview.domain.sessionize.AppLabelResolver

class PackageLabelResolver(context: Context) : AppLabelResolver {
    private val packageManager = context.packageManager
    private val cache = mutableMapOf<String, String?>()

    override fun labelFor(packageName: String): String? =
        cache.getOrPut(packageName) {
            runCatching {
                val info = packageManager.getApplicationInfo(packageName, 0)
                packageManager.getApplicationLabel(info).toString()
            }.getOrNull()
        }
}
