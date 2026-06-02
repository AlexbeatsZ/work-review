package com.metacodex.workreview

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.metacodex.workreview.data.ExportWriter
import com.metacodex.workreview.data.db.AppEventEntity
import com.metacodex.workreview.data.db.AppSessionEntity
import com.metacodex.workreview.data.usage.InstalledApp
import com.metacodex.workreview.permissions.UsageAccess
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.launch
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.format.DateTimeFormatter

class MainActivity : ComponentActivity() {
    private lateinit var repository: WorkReviewRepository

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        repository = (application as WorkReviewApp).repository
        setContent { WorkReviewMobileApp(repository) }
    }

    override fun onResume() {
        super.onResume()
        if (::repository.isInitialized) {
            MainScope().launch { repository.collectUsageNow() }
        }
    }
}

@Composable
private fun WorkReviewMobileApp(repository: WorkReviewRepository) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var selectedDate by remember { mutableStateOf(LocalDate.now()) }
    var tab by remember { mutableStateOf("overview") }
    var hasUsageAccess by remember { mutableStateOf(UsageAccess.hasUsageAccess(context)) }
    var status by remember { mutableStateOf("") }
    var lastCollectionAt by remember { mutableStateOf<Long?>(null) }
    var lastExportAt by remember { mutableStateOf<Long?>(null) }

    suspend fun refreshStatus() {
        hasUsageAccess = UsageAccess.hasUsageAccess(context)
        lastCollectionAt = repository.lastCollectionAt()
        lastExportAt = repository.lastExportAt()
    }

    LaunchedEffect(selectedDate) {
        repository.collectUsageNow()
        refreshStatus()
    }

    MaterialTheme(colorScheme = if (isSystemInDarkTheme()) darkColorScheme() else lightColorScheme()) {
        Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
            Scaffold(
                bottomBar = {
                    NavigationBar {
                        listOf("overview" to "总览", "timeline" to "时间线", "settings" to "设置", "debug" to "调试").forEach { (id, label) ->
                            NavigationBarItem(
                                selected = tab == id,
                                onClick = { tab = id },
                                label = { Text(label) },
                                icon = { Text(label.first().toString()) }
                            )
                        }
                    }
                }
            ) { padding ->
                Column(
                    modifier = Modifier
                        .padding(padding)
                        .padding(14.dp)
                        .verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    Header(
                        selectedDate = selectedDate,
                        lastCollectionAt = lastCollectionAt,
                        onPrev = { selectedDate = selectedDate.minusDays(1) },
                        onToday = { selectedDate = LocalDate.now() },
                        onNext = { selectedDate = selectedDate.plusDays(1) }
                    )
                    if (!hasUsageAccess) {
                        PermissionCard { context.startActivity(UsageAccess.settingsIntent()) }
                    }
                    if (status.isNotBlank()) {
                        Text(status, color = MaterialTheme.colorScheme.primary)
                    }
                    when (tab) {
                        "overview" -> OverviewScreen(repository, selectedDate)
                        "timeline" -> TimelineScreen(repository, selectedDate)
                        "settings" -> SettingsScreen(
                            repository = repository,
                            lastExportAt = lastExportAt,
                            onStatus = { status = it },
                            onRefreshStatus = { scope.launch { refreshStatus() } }
                        )
                        "debug" -> DebugScreen(
                            repository = repository,
                            hasUsageAccess = hasUsageAccess,
                            lastCollectionAt = lastCollectionAt,
                            onStatus = { status = it },
                            onRefreshStatus = { scope.launch { refreshStatus() } }
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun Header(
    selectedDate: LocalDate,
    lastCollectionAt: Long?,
    onPrev: () -> Unit,
    onToday: () -> Unit,
    onNext: () -> Unit
) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("Work Review Lite", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
        Text("最近采集：${lastCollectionAt?.let(::formatDateTime) ?: "尚未采集"}", color = MaterialTheme.colorScheme.onSurfaceVariant)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            TextButton(onClick = onPrev) { Text("前一天") }
            TextButton(onClick = onToday) { Text(selectedDate.toString()) }
            TextButton(onClick = onNext) { Text("后一天") }
        }
    }
}

@Composable
private fun PermissionCard(onOpenSettings: () -> Unit) {
    Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.errorContainer)) {
        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text("需要开启使用情况访问权限", fontWeight = FontWeight.Bold)
            Text("开启后会自动补采 app 使用记录。")
            Button(onClick = onOpenSettings) { Text("打开系统 Usage Access 设置") }
        }
    }
}

@Composable
private fun OverviewScreen(repository: WorkReviewRepository, date: LocalDate) {
    val summary by repository.usageSummary(date).collectAsState(initial = emptyList())
    Section("今日概览") {
        val total = summary.sumOf { it.totalDurationMs }
        MetricCard("总使用时长", "按 UsageStats 自动汇总", formatDuration(total))
    }
    Section("App 使用排行") {
        if (summary.isEmpty()) EmptyText()
        summary.forEachIndexed { index, row ->
            MetricCard(
                title = "${index + 1}. ${displayAppName(row.appLabel, row.packageName)}",
                subtitle = row.packageName,
                value = formatDuration(row.totalDurationMs)
            )
        }
    }
}

@Composable
private fun TimelineScreen(repository: WorkReviewRepository, date: LocalDate) {
    val sessions by repository.sessions(date).collectAsState(initial = emptyList())
    val scope = rememberCoroutineScope()
    Section("App 时间线") {
        if (sessions.isEmpty()) EmptyText()
        sessions.sortedByDescending { it.startTs }.forEach { session ->
            SessionRow(
                session = session,
                onDelete = { scope.launch { repository.deleteAppSession(session.id) } }
            )
        }
    }
}

@Composable
private fun SettingsScreen(
    repository: WorkReviewRepository,
    lastExportAt: Long?,
    onStatus: (String) -> Unit,
    onRefreshStatus: () -> Unit
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var minSessionSeconds by remember { mutableStateOf(60) }
    var autoExportEnabled by remember { mutableStateOf(true) }
    var autoExportTimes by remember { mutableStateOf("11:30") }
    var exportDirectoryUri by remember { mutableStateOf<String?>(null) }
    var installedApps by remember { mutableStateOf<List<InstalledApp>>(emptyList()) }
    var ignoredPackages by remember { mutableStateOf<Set<String>>(emptySet()) }
    var showIgnoreDialog by remember { mutableStateOf(false) }

    val directoryLauncher = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocumentTree()) { uri: Uri? ->
        if (uri != null) {
            context.contentResolver.takePersistableUriPermission(
                uri,
                Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION
            )
            scope.launch {
                repository.setExportDirectoryUri(uri.toString())
                exportDirectoryUri = repository.exportDirectoryUri()
                onStatus("已设置导出目录")
            }
        }
    }

    LaunchedEffect(Unit) {
        minSessionSeconds = repository.minSessionSeconds()
        autoExportEnabled = repository.autoExportEnabled()
        autoExportTimes = repository.autoExportTimesText()
        exportDirectoryUri = repository.exportDirectoryUri()
        installedApps = repository.installedApps()
        ignoredPackages = installedApps.filter { it.isIgnored }.map { it.packageName }.toSet()
    }

    Section("采集") {
        Text("已启用自动采集：app 启动、切回前台和后台定时任务都会补采。")
        Text("最小显示/记录刻度：$minSessionSeconds 秒")
        SingleChoiceSegmentedButtonRow {
            listOf(15, 30, 60, 120).forEachIndexed { index, seconds ->
                SegmentedButton(
                    selected = minSessionSeconds == seconds,
                    onClick = {
                        minSessionSeconds = seconds
                        scope.launch {
                            repository.setMinSessionSeconds(seconds)
                            repository.recollectUsageForDate(LocalDate.now())
                            onStatus("已按 $seconds 秒刻度重新整理今天")
                        }
                    },
                    shape = SegmentedButtonDefaults.itemShape(index, 4)
                ) {
                    Text("${seconds}s")
                }
            }
        }
    }

    Section("屏蔽程序") {
        Text("已屏蔽 ${ignoredPackages.size} 个程序")
        Button(onClick = { showIgnoreDialog = true }, modifier = Modifier.fillMaxWidth()) {
            Text("管理屏蔽列表")
        }
    }

    Section("导出") {
        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Column(modifier = Modifier.weight(1f)) {
                Text("自动导出 SQLite/CSV", fontWeight = FontWeight.SemiBold)
                Text("最近导出：${lastExportAt?.let(::formatDateTime) ?: "尚未导出"}", style = MaterialTheme.typography.bodySmall)
            }
            Switch(
                checked = autoExportEnabled,
                onCheckedChange = {
                    autoExportEnabled = it
                    scope.launch {
                        repository.setAutoExportEnabled(it)
                        onStatus(if (it) "已开启自动导出" else "已关闭自动导出")
                    }
                }
            )
        }
        Text("导出目录：${exportDirectoryUri ?: "未选择，使用应用默认目录"}", style = MaterialTheme.typography.bodySmall)
        Button(onClick = { directoryLauncher.launch(null) }, modifier = Modifier.fillMaxWidth()) {
            Text("选择导出目录")
        }
        OutlinedTextField(
            value = autoExportTimes,
            onValueChange = { autoExportTimes = it },
            modifier = Modifier.fillMaxWidth(),
            label = { Text("每天导出时间") },
            supportingText = { Text("一行一个或用逗号分隔，例如 08:30、11:30、18:00") },
            minLines = 3
        )
        Button(
            modifier = Modifier.fillMaxWidth(),
            onClick = {
                scope.launch {
                    repository.setAutoExportTimes(autoExportTimes)
                    autoExportTimes = repository.autoExportTimesText()
                    onStatus("已保存自动导出时间")
                }
            }
        ) { Text("保存导出时间") }
        Button(
            modifier = Modifier.fillMaxWidth(),
            onClick = {
                scope.launch {
                    val result = ExportWriter(context).exportAll(repository.exportDirectoryUri())
                    repository.setLastExportAt(System.currentTimeMillis())
                    onRefreshStatus()
                    onStatus("已导出到 ${result.destination}")
                }
            }
        ) { Text("立即导出") }
    }

    Section("数据清理") {
        Button(onClick = { scope.launch { repository.clearAppUsage(); onStatus("已清空 app 使用记录") } }) {
            Text("清空 app 记录")
        }
    }

    if (showIgnoreDialog) {
        IgnoreAppsDialog(
            apps = installedApps,
            ignoredPackages = ignoredPackages,
            onIgnoredPackagesChange = { ignoredPackages = it },
            onDismiss = { showIgnoreDialog = false },
            onSave = {
                scope.launch {
                    repository.setIgnoredPackages(ignoredPackages)
                    repository.recollectUsageForDate(LocalDate.now())
                    installedApps = repository.installedApps()
                    showIgnoreDialog = false
                    onStatus("已保存屏蔽列表并重新整理今天")
                }
            }
        )
    }
}

@Composable
private fun IgnoreAppsDialog(
    apps: List<InstalledApp>,
    ignoredPackages: Set<String>,
    onIgnoredPackagesChange: (Set<String>) -> Unit,
    onDismiss: () -> Unit,
    onSave: () -> Unit
) {
    var query by remember { mutableStateOf("") }
    val filtered = apps
        .filter { query.isBlank() || it.label.contains(query, true) || it.packageName.contains(query, true) }
        .sortedWith(compareBy<InstalledApp> { !ignoredPackages.contains(it.packageName) }.thenBy { it.label.lowercase() })

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("管理屏蔽列表") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(
                    value = query,
                    onValueChange = { query = it },
                    modifier = Modifier.fillMaxWidth(),
                    label = { Text("搜索应用") },
                    singleLine = true
                )
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(4.dp)
                ) {
                    filtered.take(120).forEach { app ->
                        AppIgnoreRow(
                            app = app.copy(isIgnored = ignoredPackages.contains(app.packageName)),
                            onToggle = { checked ->
                                onIgnoredPackagesChange(
                                    if (checked) ignoredPackages + app.packageName else ignoredPackages - app.packageName
                                )
                            }
                        )
                    }
                }
            }
        },
        confirmButton = { Button(onClick = onSave) { Text("保存") } },
        dismissButton = { TextButton(onClick = onDismiss) { Text("取消") } }
    )
}

@Composable
private fun DebugScreen(
    repository: WorkReviewRepository,
    hasUsageAccess: Boolean,
    lastCollectionAt: Long?,
    onStatus: (String) -> Unit,
    onRefreshStatus: () -> Unit
) {
    val appEvents by repository.recentAppEvents().collectAsState(initial = emptyList())
    val appSessions by repository.recentAppSessions().collectAsState(initial = emptyList())
    val scope = rememberCoroutineScope()
    Section("状态") {
        Text("Usage Access：${if (hasUsageAccess) "已开启" else "未开启"}")
        Text("WorkManager：每 15 分钟补采一次，实际执行受 Android 后台策略影响")
        Text("最近采集：${lastCollectionAt?.let(::formatDateTime) ?: "尚未采集"}")
        Button(onClick = {
            scope.launch {
                repository.collectUsageNow()
                onRefreshStatus()
                onStatus("已手动触发补采")
            }
        }) { Text("手动补采") }
    }
    Section("最近 app_events") { appEvents.forEach { AppEventRow(it) } }
    Section("最近 app_sessions") { appSessions.forEach { SessionRow(it, onDelete = null) } }
}

@Composable
private fun AppIgnoreRow(app: InstalledApp, onToggle: (Boolean) -> Unit) {
    Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Checkbox(checked = app.isIgnored, onCheckedChange = onToggle)
        Column(modifier = Modifier.weight(1f)) {
            Text(app.label, maxLines = 1, overflow = TextOverflow.Ellipsis)
            Text(
                app.packageName,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis
            )
        }
    }
}

@Composable
private fun Section(title: String, content: @Composable ColumnScope.() -> Unit) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp)
    ) {
        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(title, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
            HorizontalDivider()
            content()
        }
    }
}

@Composable private fun EmptyText() = Text("暂无数据")

@Composable
private fun MetricCard(title: String, subtitle: String, value: String) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)
    ) {
        Row(
            modifier = Modifier.padding(12.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Text(title, fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis)
                Text(subtitle, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
            Text(value, fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.primary)
        }
    }
}

@Composable
private fun SessionRow(session: AppSessionEntity, onDelete: (() -> Unit)?) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)
    ) {
        Row(
            modifier = Modifier.padding(12.dp),
            horizontalArrangement = Arrangement.spacedBy(10.dp)
        ) {
            Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text(displayAppName(session.appLabel, session.packageName), fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis)
                Text("${formatTime(session.startTs)}-${formatTime(session.endTs)} · ${session.packageName}", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 2, overflow = TextOverflow.Ellipsis)
                Text("${session.category}${session.semanticCategory?.let { " · $it" } ?: ""}", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.primary)
            }
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text(formatDuration(session.durationMs), fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.primary)
                if (onDelete != null) TextButton(onClick = onDelete) { Text("删除") }
            }
        }
    }
}

@Composable
private fun AppEventRow(event: AppEventEntity) {
    Text("${formatTime(event.ts)} ${readablePackageName(event.packageName)} type=${event.eventType}")
}

private fun formatDuration(ms: Long): String {
    if (ms < 60_000) return "${(ms / 1000).coerceAtLeast(1)}s"
    val minutes = (ms + 30_000) / 60_000
    val hours = minutes / 60
    val rem = minutes % 60
    return if (hours > 0) "${hours}h ${rem}m" else "${rem}m"
}

private fun displayAppName(appLabel: String?, packageName: String): String =
    appLabel?.takeIf { it.isNotBlank() && it != packageName }
        ?: readablePackageName(packageName)

private fun readablePackageName(packageName: String): String {
    val known = mapOf(
        "com.tencent.mobileqq" to "QQ",
        "com.tencent.mm" to "微信",
        "md.obsidian" to "Obsidian",
        "mark.via.gp" to "Via"
    )
    known[packageName]?.let { return it }
    return packageName
        .substringAfterLast('.')
        .replace('_', ' ')
        .replace('-', ' ')
        .split(' ')
        .filter { it.isNotBlank() }
        .joinToString(" ") { word -> word.replaceFirstChar { it.uppercase() } }
        .ifBlank { packageName }
}

private fun formatTime(ms: Long): String =
    DateTimeFormatter.ofPattern("HH:mm")
        .format(Instant.ofEpochMilli(ms).atZone(ZoneId.systemDefault()))

private fun formatDateTime(ms: Long): String =
    DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm")
        .format(Instant.ofEpochMilli(ms).atZone(ZoneId.systemDefault()))
