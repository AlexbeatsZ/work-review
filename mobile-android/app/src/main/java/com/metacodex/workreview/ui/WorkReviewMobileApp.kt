@file:OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)

package com.metacodex.workreview.ui

import android.content.Intent
import android.graphics.Bitmap
import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.selection.toggleable
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Analytics
import androidx.compose.material.icons.outlined.BugReport
import androidx.compose.material.icons.outlined.CalendarToday
import androidx.compose.material.icons.outlined.Check
import androidx.compose.material.icons.outlined.ChevronLeft
import androidx.compose.material.icons.outlined.ChevronRight
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.DeleteOutline
import androidx.compose.material.icons.outlined.FolderOpen
import androidx.compose.material.icons.outlined.History
import androidx.compose.material.icons.outlined.IosShare
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material.icons.outlined.Save
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material.icons.outlined.Shield
import androidx.compose.material.icons.outlined.Tune
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.CheckboxDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationBarItemDefaults
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.metacodex.workreview.WorkReviewRepository
import com.metacodex.workreview.data.ExportWriter
import com.metacodex.workreview.data.db.AppEventEntity
import com.metacodex.workreview.data.db.AppSessionEntity
import com.metacodex.workreview.data.usage.AppVisual
import com.metacodex.workreview.data.usage.InstalledApp
import com.metacodex.workreview.data.usage.resolveAppDisplayName
import com.metacodex.workreview.permissions.UsageAccess
import kotlinx.coroutines.launch
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Locale

private val Paper = Color(0xFFF7F3EC)
private val PaperRaised = Color(0xFFFFFCF7)
private val Ink = Color(0xFF292521)
private val Ash = Color(0xFF69625B)
private val Rule = Color(0xFFD9D0C5)
private val Copper = Color(0xFFC5663D)
private val Sage = Color(0xFF65705C)
private val RoastedPaper = Color(0xFF201D1A)
private val RoastedRaised = Color(0xFF292521)
private val WarmInk = Color(0xFFF3ECE2)
private val WarmAsh = Color(0xFFBDB2A7)
private val DarkRule = Color(0xFF49413A)
private val BrightCopper = Color(0xFFDF7E54)

private val WorkReviewTypography = Typography(
    headlineLarge = TextStyle(
        fontFamily = FontFamily.Serif,
        fontSize = 39.sp,
        lineHeight = 43.sp,
        fontWeight = FontWeight.Normal,
        letterSpacing = (-0.8).sp
    ),
    headlineSmall = TextStyle(
        fontFamily = FontFamily.Serif,
        fontSize = 27.sp,
        lineHeight = 32.sp,
        fontWeight = FontWeight.Normal
    ),
    titleLarge = TextStyle(
        fontFamily = FontFamily.Serif,
        fontSize = 21.sp,
        lineHeight = 27.sp,
        fontWeight = FontWeight.Medium
    ),
    titleMedium = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontSize = 16.sp,
        lineHeight = 22.sp,
        fontWeight = FontWeight.SemiBold
    ),
    bodyLarge = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontSize = 16.sp,
        lineHeight = 24.sp
    ),
    bodyMedium = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontSize = 14.sp,
        lineHeight = 21.sp
    ),
    bodySmall = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontSize = 12.sp,
        lineHeight = 18.sp
    ),
    labelLarge = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontSize = 14.sp,
        lineHeight = 18.sp,
        fontWeight = FontWeight.SemiBold
    ),
    labelSmall = TextStyle(
        fontFamily = FontFamily.Monospace,
        fontSize = 11.sp,
        lineHeight = 15.sp,
        fontWeight = FontWeight.Medium,
        letterSpacing = 0.5.sp
    )
)

private val LightColors = lightColorScheme(
    primary = Copper,
    onPrimary = Color.White,
    primaryContainer = Color(0xFFF1D8CA),
    onPrimaryContainer = Color(0xFF542612),
    secondary = Sage,
    onSecondary = Color.White,
    secondaryContainer = Color(0xFFDDE4D7),
    onSecondaryContainer = Color(0xFF252D20),
    background = Paper,
    onBackground = Ink,
    surface = PaperRaised,
    onSurface = Ink,
    surfaceVariant = Color(0xFFEDE6DD),
    onSurfaceVariant = Ash,
    outline = Rule,
    outlineVariant = Color(0xFFE8E0D6),
    error = Color(0xFFA43E32),
    errorContainer = Color(0xFFF7DDD7),
    onErrorContainer = Color(0xFF5C1712)
)

private val DarkColors = darkColorScheme(
    primary = BrightCopper,
    onPrimary = Color(0xFF3B1607),
    primaryContainer = Color(0xFF663019),
    onPrimaryContainer = Color(0xFFFFDCCB),
    secondary = Color(0xFFA9B59F),
    onSecondary = Color(0xFF263021),
    secondaryContainer = Color(0xFF3C4936),
    onSecondaryContainer = Color(0xFFDDE8D6),
    background = RoastedPaper,
    onBackground = WarmInk,
    surface = RoastedRaised,
    onSurface = WarmInk,
    surfaceVariant = Color(0xFF37322E),
    onSurfaceVariant = WarmAsh,
    outline = DarkRule,
    outlineVariant = Color(0xFF39332E),
    error = Color(0xFFFFB4A8),
    errorContainer = Color(0xFF6F251E),
    onErrorContainer = Color(0xFFFFDAD4)
)

private data class MobileTab(val id: String, val label: String, val icon: ImageVector)

private val MobileTabs = listOf(
    MobileTab("overview", "总览", Icons.Outlined.Analytics),
    MobileTab("timeline", "时间线", Icons.Outlined.History),
    MobileTab("settings", "设置", Icons.Outlined.Tune),
    MobileTab("debug", "状态", Icons.Outlined.BugReport)
)

@Composable
fun WorkReviewMobileApp(repository: WorkReviewRepository) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var selectedDate by remember { mutableStateOf(LocalDate.now()) }
    var selectedTab by remember { mutableStateOf("overview") }
    var hasUsageAccess by remember { mutableStateOf(UsageAccess.hasUsageAccess(context)) }
    var status by remember { mutableStateOf("") }
    var lastCollectionAt by remember { mutableStateOf<Long?>(null) }
    var lastExportAt by remember { mutableStateOf<Long?>(null) }

    suspend fun refreshStatus() {
        hasUsageAccess = UsageAccess.hasUsageAccess(context)
        lastCollectionAt = repository.lastCollectionAt()
        lastExportAt = repository.lastExportAt()
    }

    LaunchedEffect(selectedDate, selectedTab) {
        repository.collectUsageNow()
        refreshStatus()
    }

    WorkReviewTheme {
        Scaffold(
            containerColor = MaterialTheme.colorScheme.background,
            bottomBar = {
                WorkReviewNavigation(
                    selectedTab = selectedTab,
                    onSelect = { selectedTab = it }
                )
            }
        ) { scaffoldPadding ->
            when (selectedTab) {
                "overview" -> OverviewScreen(
                    repository = repository,
                    date = selectedDate,
                    scaffoldPadding = scaffoldPadding,
                    hasUsageAccess = hasUsageAccess,
                    status = status,
                    lastCollectionAt = lastCollectionAt,
                    onPrev = { selectedDate = selectedDate.minusDays(1) },
                    onToday = { selectedDate = LocalDate.now() },
                    onNext = { selectedDate = selectedDate.plusDays(1) },
                    onOpenPermission = { context.startActivity(UsageAccess.settingsIntent()) }
                )

                "timeline" -> TimelineScreen(
                    repository = repository,
                    date = selectedDate,
                    scaffoldPadding = scaffoldPadding,
                    hasUsageAccess = hasUsageAccess,
                    status = status,
                    lastCollectionAt = lastCollectionAt,
                    onPrev = { selectedDate = selectedDate.minusDays(1) },
                    onToday = { selectedDate = LocalDate.now() },
                    onNext = { selectedDate = selectedDate.plusDays(1) },
                    onOpenPermission = { context.startActivity(UsageAccess.settingsIntent()) }
                )

                "settings" -> SettingsScreen(
                    repository = repository,
                    scaffoldPadding = scaffoldPadding,
                    lastExportAt = lastExportAt,
                    status = status,
                    onStatus = { status = it },
                    onRefreshStatus = { scope.launch { refreshStatus() } }
                )

                else -> DebugScreen(
                    repository = repository,
                    scaffoldPadding = scaffoldPadding,
                    hasUsageAccess = hasUsageAccess,
                    lastCollectionAt = lastCollectionAt,
                    status = status,
                    onStatus = { status = it },
                    onRefreshStatus = { scope.launch { refreshStatus() } }
                )
            }
        }
    }
}

@Composable
private fun WorkReviewTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = if (isSystemInDarkTheme()) DarkColors else LightColors,
        typography = WorkReviewTypography,
        content = content
    )
}

@Composable
private fun WorkReviewNavigation(selectedTab: String, onSelect: (String) -> Unit) {
    NavigationBar(
        containerColor = MaterialTheme.colorScheme.surface,
        tonalElevation = 0.dp,
        modifier = Modifier.border(
            width = 1.dp,
            color = MaterialTheme.colorScheme.outlineVariant,
            shape = RoundedCornerShape(topStart = 22.dp, topEnd = 22.dp)
        )
    ) {
        MobileTabs.forEach { tab ->
            NavigationBarItem(
                selected = selectedTab == tab.id,
                onClick = { onSelect(tab.id) },
                icon = { Icon(tab.icon, contentDescription = tab.label) },
                label = { Text(tab.label) },
                colors = NavigationBarItemDefaults.colors(
                    selectedIconColor = MaterialTheme.colorScheme.primary,
                    selectedTextColor = MaterialTheme.colorScheme.onSurface,
                    indicatorColor = MaterialTheme.colorScheme.primaryContainer,
                    unselectedIconColor = MaterialTheme.colorScheme.onSurfaceVariant,
                    unselectedTextColor = MaterialTheme.colorScheme.onSurfaceVariant
                )
            )
        }
    }
}

@Composable
private fun PageMasthead(
    title: String,
    subtitle: String,
    isRecording: Boolean = true
) {
    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = "WORK REVIEW",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Row(
                horizontalArrangement = Arrangement.spacedBy(7.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Box(
                    Modifier
                        .size(7.dp)
                        .background(
                            if (isRecording) MaterialTheme.colorScheme.secondary
                            else MaterialTheme.colorScheme.outline,
                            CircleShape
                        )
                )
                Text(
                    if (isRecording) "正在记录" else "记录已暂停",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }
        Text(title, style = MaterialTheme.typography.headlineSmall)
        Text(
            subtitle,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}

@Composable
private fun DateChronicle(
    date: LocalDate,
    onPrev: () -> Unit,
    onToday: () -> Unit,
    onNext: () -> Unit
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        IconButton(onClick = onPrev, modifier = Modifier.size(44.dp)) {
            Icon(Icons.Outlined.ChevronLeft, contentDescription = "前一天")
        }
        Box(
            Modifier
                .weight(1f)
                .height(1.dp)
                .background(MaterialTheme.colorScheme.outline)
        )
        Box(
            Modifier
                .size(8.dp)
                .background(MaterialTheme.colorScheme.primary, CircleShape)
        )
        TextButton(onClick = onToday, contentPadding = PaddingValues(horizontal = 13.dp, vertical = 10.dp)) {
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                Text(
                    formatDate(date),
                    style = MaterialTheme.typography.titleMedium,
                    color = MaterialTheme.colorScheme.onSurface
                )
                Text(
                    if (date == LocalDate.now()) "今天" else date.year.toString(),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.primary
                )
            }
        }
        Box(
            Modifier
                .size(8.dp)
                .background(MaterialTheme.colorScheme.primary, CircleShape)
        )
        Box(
            Modifier
                .weight(1f)
                .height(1.dp)
                .background(MaterialTheme.colorScheme.outline)
        )
        IconButton(onClick = onNext, modifier = Modifier.size(44.dp)) {
            Icon(Icons.Outlined.ChevronRight, contentDescription = "后一天")
        }
    }
}

@Composable
private fun PermissionNote(onOpenSettings: () -> Unit) {
    Surface(
        color = MaterialTheme.colorScheme.errorContainer,
        contentColor = MaterialTheme.colorScheme.onErrorContainer,
        shape = RoundedCornerShape(18.dp)
    ) {
        Column(
            Modifier.padding(17.dp),
            verticalArrangement = Arrangement.spacedBy(9.dp)
        ) {
            Text("需要使用情况访问权限", style = MaterialTheme.typography.titleMedium)
            Text("开启后，Work Review 才能补全应用使用记录。", style = MaterialTheme.typography.bodyMedium)
            FilledTonalButton(onClick = onOpenSettings) {
                Text("打开系统设置")
            }
        }
    }
}

@Composable
private fun StatusNote(message: String) {
    AnimatedVisibility(visible = message.isNotBlank()) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .background(MaterialTheme.colorScheme.secondaryContainer, RoundedCornerShape(14.dp))
                .padding(horizontal = 14.dp, vertical = 11.dp),
            horizontalArrangement = Arrangement.spacedBy(9.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                Icons.Outlined.Check,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.secondary,
                modifier = Modifier.size(18.dp)
            )
            Text(message, style = MaterialTheme.typography.bodyMedium)
        }
    }
}

@Composable
private fun OverviewScreen(
    repository: WorkReviewRepository,
    date: LocalDate,
    scaffoldPadding: PaddingValues,
    hasUsageAccess: Boolean,
    status: String,
    lastCollectionAt: Long?,
    onPrev: () -> Unit,
    onToday: () -> Unit,
    onNext: () -> Unit,
    onOpenPermission: () -> Unit
) {
    val summary by repository.usageSummary(date).collectAsState(initial = emptyList())
    val total = summary.sumOf { it.totalDurationMs }

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .padding(scaffoldPadding),
        contentPadding = PaddingValues(start = 20.dp, end = 20.dp, top = 22.dp, bottom = 28.dp),
        verticalArrangement = Arrangement.spacedBy(18.dp)
    ) {
        item {
            PageMasthead(
                title = if (date == LocalDate.now()) "今日记录" else "每日记录",
                subtitle = "最近采集：${lastCollectionAt?.let(::formatDateTime) ?: "尚未采集"}"
            )
        }
        item { DateChronicle(date, onPrev, onToday, onNext) }
        if (!hasUsageAccess) item { PermissionNote(onOpenPermission) }
        if (status.isNotBlank()) item { StatusNote(status) }
        item { TimeHero(total = total, appCount = summary.size) }
        item { SectionHeading("应用排行", "按当天累计使用时长排序") }
        if (summary.isEmpty()) {
            item { EmptyLedger("这一天还没有应用记录", "开启权限或稍后刷新，记录会出现在这里。") }
        } else {
            items(summary, key = { it.packageName }) { row ->
                val visual = remember(row.packageName, row.appLabel) {
                    repository.appVisual(row.packageName, row.appLabel)
                }
                AppUsageRow(
                    visual = visual,
                    packageName = row.packageName,
                    duration = formatDurationCompact(row.totalDurationMs)
                )
            }
        }
    }
}

@Composable
private fun TimeHero(total: Long, appCount: Int) {
    Surface(
        modifier = Modifier.fillMaxWidth(),
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(24.dp),
        border = androidx.compose.foundation.BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant)
    ) {
        Column(
            Modifier.padding(horizontal = 21.dp, vertical = 22.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Text("记录到的使用时长", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Text(formatDurationLong(total), style = MaterialTheme.typography.headlineLarge)
            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
            Text(
                if (appCount == 0) "等待第一条记录" else "来自 $appCount 个应用",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.primary
            )
        }
    }
}

@Composable
private fun AppUsageRow(visual: AppVisual, packageName: String, duration: String) {
    Column {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(vertical = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            AppMark(visual.label, visual.icon)
            Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text(visual.label, style = MaterialTheme.typography.titleMedium, maxLines = 1, overflow = TextOverflow.Ellipsis)
                Text(
                    packageName,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
            }
            Text(
                duration,
                style = MaterialTheme.typography.labelLarge.copy(fontFamily = FontFamily.Monospace),
                color = MaterialTheme.colorScheme.primary
            )
        }
        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
    }
}

@Composable
private fun TimelineScreen(
    repository: WorkReviewRepository,
    date: LocalDate,
    scaffoldPadding: PaddingValues,
    hasUsageAccess: Boolean,
    status: String,
    lastCollectionAt: Long?,
    onPrev: () -> Unit,
    onToday: () -> Unit,
    onNext: () -> Unit,
    onOpenPermission: () -> Unit
) {
    val sessions by repository.sessions(date).collectAsState(initial = emptyList())
    val scope = rememberCoroutineScope()
    val ordered = remember(sessions) { sessions.sortedByDescending { it.startTs } }

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .padding(scaffoldPadding),
        contentPadding = PaddingValues(start = 20.dp, end = 20.dp, top = 22.dp, bottom = 28.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        item {
            PageMasthead(
                title = "时间线",
                subtitle = "最近采集：${lastCollectionAt?.let(::formatDateTime) ?: "尚未采集"}"
            )
        }
        item { DateChronicle(date, onPrev, onToday, onNext) }
        if (!hasUsageAccess) item { PermissionNote(onOpenPermission) }
        if (status.isNotBlank()) item { StatusNote(status) }
        item { SectionHeading("当天轨迹", if (ordered.isEmpty()) "暂无会话" else "${ordered.size} 段应用会话") }
        if (ordered.isEmpty()) {
            item { EmptyLedger("时间线上还没有记录", "使用应用后再回来，这里会按时间排列当天轨迹。") }
        } else {
            items(ordered, key = { it.id }) { session ->
                val visual = remember(session.packageName, session.appLabel) {
                    repository.appVisual(session.packageName, session.appLabel)
                }
                TimelineEntry(
                    session = session,
                    visual = visual,
                    onDelete = { scope.launch { repository.deleteAppSession(session.id) } }
                )
            }
        }
    }
}

@Composable
private fun TimelineEntry(session: AppSessionEntity, visual: AppVisual, onDelete: (() -> Unit)?) {
    Row(modifier = Modifier.fillMaxWidth()) {
        Text(
            formatTime(session.startTs),
            modifier = Modifier.width(48.dp).padding(top = 5.dp),
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
        Box(
            modifier = Modifier
                .width(24.dp)
                .height(86.dp),
            contentAlignment = Alignment.TopCenter
        ) {
            Box(
                Modifier
                    .fillMaxHeight()
                    .width(1.dp)
                    .background(MaterialTheme.colorScheme.outline)
            )
            Box(
                Modifier
                    .padding(top = 9.dp)
                    .size(9.dp)
                    .background(MaterialTheme.colorScheme.primary, CircleShape)
                    .border(2.dp, MaterialTheme.colorScheme.background, CircleShape)
            )
        }
        Row(
            modifier = Modifier
                .weight(1f)
                .padding(start = 8.dp, bottom = 13.dp),
            horizontalArrangement = Arrangement.spacedBy(11.dp),
            verticalAlignment = Alignment.Top
        ) {
            AppMark(visual.label, visual.icon, size = 40.dp)
            Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(3.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        visual.label,
                        style = MaterialTheme.typography.titleMedium,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                        modifier = Modifier.weight(1f)
                    )
                    Text(
                        formatDurationCompact(session.durationMs),
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.primary
                    )
                }
                Text(
                    "${formatTime(session.startTs)}–${formatTime(session.endTs)} · ${session.category}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
                Text(
                    session.packageName,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
            }
            if (onDelete != null) {
                IconButton(onClick = onDelete, modifier = Modifier.size(40.dp)) {
                    Icon(
                        Icons.Outlined.DeleteOutline,
                        contentDescription = "删除这段记录",
                        tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.size(19.dp)
                    )
                }
            }
        }
    }
}

@Composable
private fun SettingsScreen(
    repository: WorkReviewRepository,
    scaffoldPadding: PaddingValues,
    lastExportAt: Long?,
    status: String,
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
    var appsLoading by remember { mutableStateOf(true) }
    var showIgnoreSheet by remember { mutableStateOf(false) }

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
        appsLoading = false
    }

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .padding(scaffoldPadding),
        contentPadding = PaddingValues(start = 20.dp, end = 20.dp, top = 22.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(24.dp)
    ) {
        item { PageMasthead("设置", "只保留影响记录、隐私和导出的选项") }
        if (status.isNotBlank()) item { StatusNote(status) }
        item {
            LedgerSection("采集", "决定多短的应用会话会被保留") {
                Text("最小记录刻度", style = MaterialTheme.typography.titleMedium)
                Text(
                    "当前为 $minSessionSeconds 秒。修改后会重新整理今天的记录。",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                SingleChoiceSegmentedButtonRow(modifier = Modifier.fillMaxWidth()) {
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
                            shape = SegmentedButtonDefaults.itemShape(index, 4),
                            modifier = Modifier.weight(1f)
                        ) { Text("${seconds}s") }
                    }
                }
            }
        }
        item {
            LedgerSection("屏蔽程序", "屏蔽后不记录、不统计，也不会出现在时间线上") {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                        Text("${ignoredPackages.size} 个程序", style = MaterialTheme.typography.titleMedium)
                        Text(
                            if (appsLoading) "正在读取应用名称和图标…" else "从应用列表中选择",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                    Icon(
                        Icons.Outlined.Shield,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.primary
                    )
                }
                installedApps.filter { ignoredPackages.contains(it.packageName) }.take(3).forEach { app ->
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(11.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        AppMark(app.label, app.icon, size = 36.dp)
                        Text(app.label, modifier = Modifier.weight(1f), maxLines = 1, overflow = TextOverflow.Ellipsis)
                        Text("已屏蔽", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.primary)
                    }
                }
                Button(
                    onClick = { showIgnoreSheet = true },
                    modifier = Modifier
                        .fillMaxWidth()
                        .heightIn(min = 48.dp),
                    enabled = !appsLoading
                ) {
                    Text("管理屏蔽列表")
                }
            }
        }
        item {
            LedgerSection("导出", "在你选择的目录保留 SQLite 和 CSV 副本") {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                        Text("自动导出", style = MaterialTheme.typography.titleMedium)
                        Text(
                            "最近：${lastExportAt?.let(::formatDateTime) ?: "尚未导出"}",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                    Switch(
                        checked = autoExportEnabled,
                        onCheckedChange = {
                            autoExportEnabled = it
                            scope.launch {
                                repository.setAutoExportEnabled(it)
                                onStatus(if (it) "已开启自动导出" else "已关闭自动导出")
                            }
                        },
                        colors = SwitchDefaults.colors(checkedTrackColor = MaterialTheme.colorScheme.secondary)
                    )
                }
                HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                Text(
                    exportDirectoryUri ?: "未选择目录，将使用应用默认目录",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis
                )
                OutlinedButton(
                    onClick = { directoryLauncher.launch(null) },
                    modifier = Modifier.fillMaxWidth().heightIn(min = 48.dp)
                ) {
                    Icon(Icons.Outlined.FolderOpen, contentDescription = null)
                    Spacer(Modifier.width(8.dp))
                    Text("选择导出目录")
                }
                OutlinedTextField(
                    value = autoExportTimes,
                    onValueChange = { autoExportTimes = it },
                    modifier = Modifier.fillMaxWidth(),
                    label = { Text("每天导出时间") },
                    supportingText = { Text("一行一个，或用逗号分隔：08:30、11:30、18:00") },
                    minLines = 2
                )
                Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                    OutlinedButton(
                        modifier = Modifier.weight(1f).heightIn(min = 48.dp),
                        onClick = {
                            scope.launch {
                                repository.setAutoExportTimes(autoExportTimes)
                                autoExportTimes = repository.autoExportTimesText()
                                onStatus("已保存自动导出时间")
                            }
                        }
                    ) {
                        Icon(Icons.Outlined.Save, contentDescription = null)
                        Spacer(Modifier.width(7.dp))
                        Text("保存时间")
                    }
                    Button(
                        modifier = Modifier.weight(1f).heightIn(min = 48.dp),
                        onClick = {
                            scope.launch {
                                val result = ExportWriter(context).exportAll(repository.exportDirectoryUri())
                                repository.setLastExportAt(System.currentTimeMillis())
                                onRefreshStatus()
                                onStatus("已导出到 ${result.destination}")
                            }
                        }
                    ) {
                        Icon(Icons.Outlined.IosShare, contentDescription = null)
                        Spacer(Modifier.width(7.dp))
                        Text("立即导出")
                    }
                }
            }
        }
        item {
            LedgerSection("数据清理", "只清空手机端采集的应用使用记录") {
                OutlinedButton(
                    onClick = {
                        scope.launch {
                            repository.clearAppUsage()
                            onStatus("已清空应用使用记录")
                        }
                    },
                    modifier = Modifier.fillMaxWidth().heightIn(min = 48.dp),
                    colors = ButtonDefaults.outlinedButtonColors(contentColor = MaterialTheme.colorScheme.error)
                ) {
                    Icon(Icons.Outlined.DeleteOutline, contentDescription = null)
                    Spacer(Modifier.width(8.dp))
                    Text("清空应用记录")
                }
            }
        }
    }

    if (showIgnoreSheet) {
        IgnoreAppsSheet(
            apps = installedApps,
            ignoredPackages = ignoredPackages,
            onIgnoredPackagesChange = { ignoredPackages = it },
            onDismiss = { showIgnoreSheet = false },
            onSave = {
                scope.launch {
                    repository.setIgnoredPackages(ignoredPackages)
                    repository.recollectUsageForDate(LocalDate.now())
                    installedApps = repository.installedApps()
                    showIgnoreSheet = false
                    onStatus("已保存屏蔽列表并重新整理今天")
                }
            }
        )
    }
}

@Composable
private fun IgnoreAppsSheet(
    apps: List<InstalledApp>,
    ignoredPackages: Set<String>,
    onIgnoredPackagesChange: (Set<String>) -> Unit,
    onDismiss: () -> Unit,
    onSave: () -> Unit
) {
    var query by remember { mutableStateOf("") }
    val filtered = remember(apps, ignoredPackages, query) {
        apps
            .filter { query.isBlank() || it.label.contains(query, true) || it.packageName.contains(query, true) }
            .sortedWith(
                compareBy<InstalledApp> { !ignoredPackages.contains(it.packageName) }
                    .thenBy { it.label.lowercase() }
            )
    }
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheetState,
        containerColor = MaterialTheme.colorScheme.surface,
        contentColor = MaterialTheme.colorScheme.onSurface
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .fillMaxHeight(0.93f)
                .padding(horizontal = 20.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column(verticalArrangement = Arrangement.spacedBy(3.dp)) {
                    Text("管理屏蔽列表", style = MaterialTheme.typography.titleLarge)
                    Text(
                        "已选择 ${ignoredPackages.size} 个程序",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
                IconButton(onClick = onDismiss) {
                    Icon(Icons.Outlined.Close, contentDescription = "关闭")
                }
            }
            Spacer(Modifier.height(14.dp))
            OutlinedTextField(
                value = query,
                onValueChange = { query = it },
                modifier = Modifier.fillMaxWidth(),
                placeholder = { Text("按应用名称或包名搜索") },
                leadingIcon = { Icon(Icons.Outlined.Search, contentDescription = null) },
                singleLine = true
            )
            Spacer(Modifier.height(10.dp))
            Text(
                "名称是主要识别信息；包名仅用于区分同名应用。",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Spacer(Modifier.height(8.dp))
            LazyColumn(
                modifier = Modifier.weight(1f),
                contentPadding = PaddingValues(vertical = 6.dp)
            ) {
                if (filtered.isEmpty()) {
                    item { EmptyLedger("没有匹配的应用", "换一个名称或包名再试。") }
                } else {
                    items(filtered, key = { it.packageName }) { app ->
                        val checked = ignoredPackages.contains(app.packageName)
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .toggleable(
                                    value = checked,
                                    role = Role.Checkbox,
                                    onValueChange = { selected ->
                                        onIgnoredPackagesChange(
                                            if (selected) ignoredPackages + app.packageName
                                            else ignoredPackages - app.packageName
                                        )
                                    }
                                )
                                .padding(vertical = 10.dp),
                            horizontalArrangement = Arrangement.spacedBy(12.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            AppMark(app.label, app.icon)
                            Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                Text(
                                    app.label,
                                    style = MaterialTheme.typography.titleMedium,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis
                                )
                                Text(
                                    app.packageName,
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis
                                )
                            }
                            Checkbox(
                                checked = checked,
                                onCheckedChange = null,
                                colors = CheckboxDefaults.colors(checkedColor = MaterialTheme.colorScheme.primary)
                            )
                        }
                        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                    }
                }
            }
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(top = 12.dp, bottom = 18.dp),
                horizontalArrangement = Arrangement.spacedBy(10.dp)
            ) {
                OutlinedButton(
                    onClick = onDismiss,
                    modifier = Modifier.weight(1f).heightIn(min = 50.dp)
                ) { Text("取消") }
                Button(
                    onClick = onSave,
                    modifier = Modifier.weight(1f).heightIn(min = 50.dp)
                ) { Text("保存屏蔽列表") }
            }
        }
    }
}

@Composable
private fun DebugScreen(
    repository: WorkReviewRepository,
    scaffoldPadding: PaddingValues,
    hasUsageAccess: Boolean,
    lastCollectionAt: Long?,
    status: String,
    onStatus: (String) -> Unit,
    onRefreshStatus: () -> Unit
) {
    val appEvents by repository.recentAppEvents().collectAsState(initial = emptyList())
    val appSessions by repository.recentAppSessions().collectAsState(initial = emptyList())
    val scope = rememberCoroutineScope()

    LazyColumn(
        modifier = Modifier.fillMaxSize().padding(scaffoldPadding),
        contentPadding = PaddingValues(start = 20.dp, end = 20.dp, top = 22.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(22.dp)
    ) {
        item { PageMasthead("采集状态", "确认权限、后台任务和最近写入是否正常") }
        if (status.isNotBlank()) item { StatusNote(status) }
        item {
            LedgerSection("运行状态", "后台采集实际执行时间受 Android 策略影响") {
                StatusLine("Usage Access", if (hasUsageAccess) "已开启" else "未开启")
                StatusLine("WorkManager", "每 15 分钟补采")
                StatusLine("最近采集", lastCollectionAt?.let(::formatDateTime) ?: "尚未采集")
                Button(
                    onClick = {
                        scope.launch {
                            repository.collectUsageNow()
                            onRefreshStatus()
                            onStatus("已手动触发补采")
                        }
                    },
                    modifier = Modifier.fillMaxWidth().heightIn(min = 48.dp)
                ) {
                    Icon(Icons.Outlined.Refresh, contentDescription = null)
                    Spacer(Modifier.width(8.dp))
                    Text("立即补采")
                }
            }
        }
        item { SectionHeading("最近事件", "app_events") }
        if (appEvents.isEmpty()) item { EmptyLedger("还没有应用事件", "完成一次采集后会在这里显示。") }
        items(appEvents, key = { it.id }) { AppEventRow(it) }
        item { SectionHeading("最近会话", "app_sessions") }
        if (appSessions.isEmpty()) item { EmptyLedger("还没有应用会话", "前后台事件形成完整会话后会显示。") }
        items(appSessions, key = { it.id }) { session ->
            val visual = remember(session.packageName, session.appLabel) {
                repository.appVisual(session.packageName, session.appLabel)
            }
            TimelineEntry(session, visual, onDelete = null)
        }
    }
}

@Composable
private fun StatusLine(label: String, value: String) {
    Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
        Text(label, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(value, style = MaterialTheme.typography.labelLarge)
    }
}

@Composable
private fun AppEventRow(event: AppEventEntity) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(vertical = 7.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text(formatTime(event.ts), style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.primary)
        Column(modifier = Modifier.weight(1f)) {
            Text(resolveAppDisplayName(event.packageName, null), style = MaterialTheme.typography.titleMedium)
            Text(
                "${event.packageName} · type=${event.eventType}",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis
            )
        }
    }
}

@Composable
private fun LedgerSection(
    title: String,
    supporting: String,
    content: @Composable ColumnScope.() -> Unit
) {
    Column(verticalArrangement = Arrangement.spacedBy(13.dp)) {
        SectionHeading(title, supporting)
        content()
    }
}

@Composable
private fun SectionHeading(title: String, supporting: String) {
    Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Text(title, style = MaterialTheme.typography.titleLarge)
        Text(
            supporting,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
        HorizontalDivider(color = MaterialTheme.colorScheme.outline)
    }
}

@Composable
private fun EmptyLedger(title: String, supporting: String) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .border(1.dp, MaterialTheme.colorScheme.outlineVariant, RoundedCornerShape(18.dp))
            .padding(20.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        Icon(
            Icons.Outlined.CalendarToday,
            contentDescription = null,
            tint = MaterialTheme.colorScheme.primary
        )
        Text(title, style = MaterialTheme.typography.titleMedium)
        Text(
            supporting,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}

@Composable
private fun AppMark(label: String, bitmap: Bitmap?, size: androidx.compose.ui.unit.Dp = 44.dp) {
    if (bitmap != null) {
        Image(
            bitmap = bitmap.asImageBitmap(),
            contentDescription = "$label 图标",
            modifier = Modifier
                .size(size)
                .clip(RoundedCornerShape(size * 0.25f))
        )
    } else {
        val colors = listOf(
            MaterialTheme.colorScheme.primaryContainer,
            MaterialTheme.colorScheme.secondaryContainer,
            MaterialTheme.colorScheme.surfaceVariant
        )
        Box(
            modifier = Modifier
                .size(size)
                .background(colors[(label.hashCode() and Int.MAX_VALUE) % colors.size], RoundedCornerShape(size * 0.25f)),
            contentAlignment = Alignment.Center
        ) {
            Text(
                label.trim().firstOrNull()?.uppercase() ?: "?",
                style = MaterialTheme.typography.titleMedium,
                color = MaterialTheme.colorScheme.onSurface
            )
        }
    }
}

private fun formatDurationCompact(ms: Long): String {
    if (ms < 60_000) return "${(ms / 1000).coerceAtLeast(1)}s"
    val minutes = (ms + 30_000) / 60_000
    val hours = minutes / 60
    val remaining = minutes % 60
    return if (hours > 0) "${hours}h ${remaining}m" else "${remaining}m"
}

private fun formatDurationLong(ms: Long): String {
    if (ms <= 0) return "0 分钟"
    val minutes = (ms + 30_000) / 60_000
    val hours = minutes / 60
    val remaining = minutes % 60
    return when {
        hours > 0 && remaining > 0 -> "$hours 小时 $remaining 分"
        hours > 0 -> "$hours 小时"
        else -> "$remaining 分钟"
    }
}

private fun formatDate(date: LocalDate): String =
    DateTimeFormatter.ofPattern("M 月 d 日 · EEEE", Locale.SIMPLIFIED_CHINESE).format(date)

private fun formatTime(ms: Long): String =
    DateTimeFormatter.ofPattern("HH:mm")
        .format(Instant.ofEpochMilli(ms).atZone(ZoneId.systemDefault()))

private fun formatDateTime(ms: Long): String =
    DateTimeFormatter.ofPattern("M-d HH:mm")
        .format(Instant.ofEpochMilli(ms).atZone(ZoneId.systemDefault()))
