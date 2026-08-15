package com.metacodex.workreview.data.usage

import android.content.Context
import com.metacodex.workreview.domain.sessionize.AppLabelResolver

class PackageLabelResolver(context: Context) : AppLabelResolver {
    private val visuals = PackageVisualResolver(context)

    override fun labelFor(packageName: String): String = visuals.resolve(packageName).label
}
