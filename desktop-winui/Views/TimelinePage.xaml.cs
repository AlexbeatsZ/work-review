using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Shapes;
using Microsoft.UI.Xaml.Media.Imaging;
using WorkReview.Engine;
using WorkReview.Models;
using WorkReview.Services;

namespace WorkReview.Views;

/// <summary>时间线页：活动列表、日期导航、详情与分类编辑、实时更新。</summary>
public sealed partial class TimelinePage : UserControl
{
    private const int PageSize = 12;
    private const int FeaturedDurationThreshold = 20 * 60;
    private const int FeaturedContextThreshold = 10 * 60;
    private const int FeaturedMinGap = 2;
    private const int FeaturedMaxItems = 4;

    private static readonly string[] CategoryEmojis =
    {
        "💻", "🌐", "💬", "📝", "🎨", "🎮", "📁",
        "⚡", "📊", "🔧", "🛠️", "💡", "🎯", "📌",
        "🏷️", "🏠", "📚", "🎵", "📷", "🔬", "🧪",
        "💼", "🧑‍💻", "🧑‍🎨", "📱", "🚀", "⭐", "🔒",
    };

    private static readonly string[] CategoryPresetColors =
    {
        "#6366F1", "#0EA5E9", "#10B981", "#F59E0B",
        "#EF4444", "#8B5CF6", "#EC4899", "#14B8A6",
    };

    private readonly List<Activity> _activities = new();
    private List<HourlySummary> _hourlySummaries = new();
    private HashSet<long> _featuredIds = new();
    private DispatcherTimer? _clockTimer;
    private int _offset;
    private bool _hasMore = true;
    private bool _loading;
    private bool _loadingMore;
    private int _loadRequestId;
    private string _selectedDate = TodayString();
    private Activity? _selectedActivity;
    private bool _categorySaving;

    private readonly IconCache _icons = new();

    public TimelinePage()
    {
        InitializeComponent();
        ApplyLocale();

        _clockTimer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(1) };
        _clockTimer.Tick += (_, _) => UpdateLiveClock();
        _clockTimer.Start();
    }

    /// <summary>切换到本页时触发。</summary>
    public void OnActivated()
    {
        if (IsToday(_selectedDate))
        {
            _ = LoadTimelineAsync();
        }
    }

    /// <summary>主窗口重新激活时触发（等价于 document visibilitychange）。</summary>
    public void OnWindowActivated()
    {
        if (IsToday(_selectedDate) && !_loading)
        {
            _ = LoadTimelineAsync();
        }
    }

    /// <summary>引擎 screenshot-taken 事件（主窗口已过滤为 UI 线程）。</summary>
    public void OnActivityUpserted(Activity activity)
    {
        if (!IsToday(_selectedDate))
        {
            return;
        }

        if (!string.IsNullOrEmpty(activity.ScreenshotPath))
        {
            _ = _icons.GetAsync(activity.AppName, activity.ExecutablePath);
        }

        UpsertActivity(activity);
    }

    public void ApplyLocale()
    {
        PageTitle.Text = I18n.T("sidebar.nav.timeline");
        TodayButton.Content = I18n.T("datePicker.today");
        ToolTipService.SetToolTip(RefreshButton, I18n.T("timeline.refreshTitle"));
        ErrorTitle.Text = I18n.T("timeline.loadError");
        RetryButton.Content = I18n.T("timeline.retry");
        EmptyText.Text = I18n.T("timeline.empty");
        NoMoreText.Text = I18n.T("timeline.noMore");
        SummaryLinkText.Text = I18n.T("timeline.periodSummary");

        if (_activities.Count > 0)
        {
            UpdateSummaryStrip();
            RebuildRows();
        }
        else
        {
            UpdateSummaryStrip();
        }
    }

    // ---------- 数据加载 ----------

    private static string TodayString()
    {
        var now = DateTime.Now;
        return $"{now.Year:D4}-{now.Month:D2}-{now.Day:D2}";
    }

    private static bool IsToday(string date) => date == TodayString();

    private async Task LoadTimelineAsync()
    {
        if (_loading)
        {
            return;
        }

        var requestId = ++_loadRequestId;
        _loading = true;
        _offset = 0;
        _hasMore = true;

        ShowState(showLoading: true);

        try
        {
            var activitiesTask = EngineApi.InvokeAsync<List<Activity>>("get_timeline", new
            {
                date = _selectedDate,
                limit = PageSize,
                offset = 0,
            });
            var summariesTask = EngineApi.InvokeAsync<List<HourlySummary>>("get_hourly_summaries", new
            {
                date = _selectedDate,
            });
            await Task.WhenAll(activitiesTask, summariesTask);

            if (requestId != _loadRequestId)
            {
                return;
            }

            _activities.Clear();
            _activities.AddRange(PrepareActivities(activitiesTask.Result));
            _hourlySummaries = summariesTask.Result;

            _offset = _activities.Count;
            _hasMore = activitiesTask.Result.Count >= PageSize;
            _featuredIds = SelectFeaturedActivityIds(_activities);

            RebuildRows();
            UpdateSummaryStrip();
            ShowState(showList: true);

            // 后台预热缩略图与图标
            _ = Task.Run(() =>
            {
                foreach (var a in _activities.Where(a => !string.IsNullOrEmpty(a.ScreenshotPath)).Take(6).ToList())
                {
                    _ = LoadThumbnailAsync(a.ScreenshotPath);
                }
                foreach (var a in _activities.Take(12).ToList())
                {
                    _ = _icons.GetAsync(a.AppName, a.ExecutablePath);
                }
            });
        }
        catch (Exception e)
        {
            if (requestId != _loadRequestId)
            {
                return;
            }
            ErrorText.Text = e.Message;
            ShowState(showError: true);
        }
        finally
        {
            _loading = false;
        }
    }

    private async Task LoadMoreAsync()
    {
        if (_loadingMore || !_hasMore)
        {
            return;
        }
        _loadingMore = true;
        LoadMoreButton.IsEnabled = false;

        try
        {
            var more = await EngineApi.InvokeAsync<List<Activity>>("get_timeline", new
            {
                date = _selectedDate,
                limit = PageSize,
                offset = _offset,
            });

            if (more.Count > 0)
            {
                _activities.AddRange(PrepareActivities(more));
                _offset += more.Count;
                _featuredIds = SelectFeaturedActivityIds(_activities);
                RebuildRows();
            }

            if (more.Count < PageSize)
            {
                _hasMore = false;
                LoadMoreButton.Visibility = Visibility.Collapsed;
                NoMoreText.Visibility = Visibility.Visible;
            }
        }
        catch
        {
            // 加载更多失败保持当前列表
        }
        finally
        {
            _loadingMore = false;
            LoadMoreButton.IsEnabled = true;
        }
    }

    private void ShowState(
        bool showLoading = false,
        bool showList = false,
        bool showError = false,
        bool showEmpty = false)
    {
        LoadingRing.IsActive = showLoading;
        LoadingRing.Visibility = showLoading ? Visibility.Visible : Visibility.Collapsed;
        ListCard.Visibility = showList ? Visibility.Visible : Visibility.Collapsed;
        ErrorBanner.Visibility = showError ? Visibility.Visible : Visibility.Collapsed;
        EmptyState.Visibility = showEmpty ? Visibility.Visible : Visibility.Collapsed;
        LoadMoreButton.Visibility = Visibility.Collapsed;
        NoMoreText.Visibility = Visibility.Collapsed;
    }

    // ---------- 数据准备（与 timelineData.js 一致） ----------

    private static List<Activity> PrepareActivities(IEnumerable<Activity> data) =>
        data.OrderByDescending(a => a.Timestamp).ThenByDescending(a => a.Id ?? 0).ToList();

    private void UpsertActivity(Activity incoming)
    {
        if (incoming.Id is { } id)
        {
            var index = _activities.FindIndex(a => a.Id == id);
            if (index >= 0)
            {
                _activities[index] = incoming;
                _featuredIds = SelectFeaturedActivityIds(_activities);
                RefreshRowFor(incoming);
                UpdateSummaryStrip();
                return;
            }
        }

        _activities.Insert(0, incoming);
        _activities.Sort((a, b) =>
        {
            var cmp = b.Timestamp.CompareTo(a.Timestamp);
            return cmp != 0 ? cmp : (b.Id ?? 0).CompareTo(a.Id ?? 0);
        });
        _offset += 1;
        _featuredIds = SelectFeaturedActivityIds(_activities);
        RebuildRows();
        UpdateSummaryStrip();
    }

    private HashSet<long> SelectFeaturedActivityIds(IReadOnlyList<Activity> items)
    {
        var featured = new List<long>();
        var maxCount = Math.Min(FeaturedMaxItems, Math.Max(1, (items.Count + 3) / 4));
        var lastFeaturedIndex = -99;

        for (var index = 0; index < items.Count; index++)
        {
            var activity = items[index];
            var previous = index > 0 ? items[index - 1] : null;

            if (activity.Id is not { } id || string.IsNullOrEmpty(activity.ScreenshotPath))
            {
                continue;
            }

            var score = 0;
            if (activity.Duration >= FeaturedDurationThreshold)
            {
                score += 3;
            }
            else if (activity.Duration >= FeaturedContextThreshold)
            {
                score += 1;
            }
            if (!string.IsNullOrEmpty(activity.BrowserUrl))
            {
                score += 1;
            }
            if (previous is not null &&
                (!string.Equals(NormalizeAppKey(previous.AppName), NormalizeAppKey(activity.AppName), StringComparison.Ordinal) ||
                 !string.Equals(previous.Category ?? "other", activity.Category ?? "other", StringComparison.Ordinal)))
            {
                score += 1;
            }
            if (index == 0)
            {
                score += 1;
            }
            if (score < 3 || index - lastFeaturedIndex < FeaturedMinGap)
            {
                continue;
            }

            featured.Add(id);
            lastFeaturedIndex = index;

            if (featured.Count >= maxCount)
            {
                break;
            }
        }

        if (featured.Count == 0)
        {
            var fallback = items.FirstOrDefault(a => a.Id is not null && !string.IsNullOrEmpty(a.ScreenshotPath));
            if (fallback?.Id is { } fallbackId)
            {
                featured.Add(fallbackId);
            }
        }

        return featured.ToHashSet();
    }

    private static string NormalizeAppKey(string? value) =>
        (value ?? string.Empty).Trim().ToLowerInvariant();

    // ---------- 显示格式化 ----------

    private static readonly (string Suffix, string Replacement)[] TitleSuffixRules =
    {
        (" - Google Chrome", ""),
        (" - Chrome", ""),
        (" - Mozilla Firefox", ""),
        (" - Firefox", ""),
        (" - Safari", ""),
        (" - Microsoft Edge", ""),
        (" - Visual Studio Code", ""),
        (" · GitHub", ""),
        (" - YouTube", ""),
    };

    private string FormatWindowTitle(string? title, string appName, string? browserUrl)
    {
        if (!string.IsNullOrWhiteSpace(title))
        {
            var clean = title.Trim();
            foreach (var (suffix, replacement) in TitleSuffixRules)
            {
                if (clean.EndsWith(suffix, StringComparison.OrdinalIgnoreCase))
                {
                    clean = clean[..^suffix.Length] + replacement;
                    break;
                }
            }
            if (clean.Length > 60)
            {
                clean = clean[..57] + "...";
            }
            if (clean.Trim().Length > 0)
            {
                return clean;
            }
            return title;
        }

        if (!string.IsNullOrEmpty(browserUrl))
        {
            var display = FormatBrowserUrl(browserUrl);
            try
            {
                if (Uri.TryCreate(display, UriKind.Absolute, out var uri) &&
                    !string.IsNullOrEmpty(uri.Host))
                {
                    return uri.Host;
                }
            }
            catch
            {
                // fall through
            }
            return display.Length > 40 ? display[..40] : display;
        }

        return I18n.T("timeline.inUse", ("appName", appName));
    }

    private static string FormatBrowserUrl(string? url)
    {
        if (string.IsNullOrEmpty(url))
        {
            return string.Empty;
        }
        var text = url.Trim();
        try
        {
            if (Uri.TryCreate(text, UriKind.Absolute, out var uri))
            {
                var path = Uri.UnescapeDataString(uri.PathAndQuery);
                return uri.Host + path;
            }
        }
        catch
        {
            // fall through
        }
        return text.StartsWith("http", StringComparison.OrdinalIgnoreCase)
            ? text[(text.IndexOf("://", StringComparison.Ordinal) + 3)..]
            : text;
    }

    private string FormatTime(long timestamp) =>
        DateTimeOffset.FromUnixTimeSeconds(timestamp).ToLocalTime().DateTime.ToString("HH:mm:ss", CultureFormat());

    private string FormatAnchor(long timestamp) =>
        DateTimeOffset.FromUnixTimeSeconds(timestamp).ToLocalTime().DateTime.ToString("HH:mm", CultureFormat());

    private static IFormatProvider CultureFormat() =>
        System.Globalization.CultureInfo.InvariantCulture;

    // ---------- 应用名优选（appDisplay.js 移植） ----------

    private static string NormalizeComparable(string value) =>
        new string((value ?? string.Empty).Trim().ToLowerInvariant()
            .Where(c => char.IsAsciiLetterOrDigit(c) || c is >= '\u4e00' and <= '\u9fa5')
            .ToArray());

    private static bool IsGenericInstallerToken(string value) =>
        new[] { "setup", "install", "installer", "uninstall" }.Contains(NormalizeComparable(value));

    private static bool IsInstallerLikeName(string value)
    {
        var comparable = NormalizeComparable(value);
        return comparable.Contains("setup") || comparable.Contains("installer")
            || comparable.Contains("uninstall") || comparable.Contains("install");
    }

    private static bool IsCompactRawToken(string value) =>
        !string.IsNullOrEmpty(value)
        && value.All(c => char.IsAsciiLetterOrDigit(c) || c is '.' or '_' or '-' or '+')
        && !value.Any(char.IsUpper);

    public static string GetPreferredTimelineAppName(string? appName, string? windowTitle)
    {
        var rawAppName = (appName ?? string.Empty).Trim();
        var rawTitle = (windowTitle ?? string.Empty).Trim();
        if (rawTitle.Length == 0)
        {
            return rawAppName;
        }

        var appComparable = NormalizeComparable(rawAppName);
        var titleComparable = NormalizeComparable(rawTitle);

        if (IsGenericInstallerToken(rawAppName) && rawTitle.Length > rawAppName.Length)
        {
            return rawTitle;
        }

        if (IsInstallerLikeName(rawAppName)
            && titleComparable.Length > 0 && appComparable.Length > 0
            && (appComparable.Contains(titleComparable)
                || (appComparable.StartsWith("workreview") && titleComparable.StartsWith("workreview")
                    && appComparable.EndsWith("setup") && titleComparable.EndsWith("setup"))))
        {
            return rawTitle;
        }

        if (appComparable.Length > 0 && titleComparable == appComparable && rawTitle != rawAppName)
        {
            return rawTitle;
        }

        if (IsCompactRawToken(rawAppName)
            && titleComparable.Length > 0 && appComparable.Length > 0
            && (appComparable.Contains(titleComparable) || titleComparable.Contains(appComparable))
            && rawTitle.Length >= rawAppName.Length)
        {
            return rawTitle;
        }

        return rawAppName.Length > 0 ? rawAppName : rawTitle;
    }

    private static bool ShouldPreferFallbackIcon(string? appName, string? windowTitle)
    {
        var rawAppName = (appName ?? string.Empty).Trim();
        var preferred = GetPreferredTimelineAppName(rawAppName, windowTitle);
        if (preferred.Length == 0)
        {
            return false;
        }
        if (IsGenericInstallerToken(rawAppName))
        {
            return true;
        }
        if (IsInstallerLikeName(rawAppName) && (windowTitle ?? string.Empty).Trim().Length > 0)
        {
            return true;
        }
        if (!string.Equals(NormalizeComparable(preferred), NormalizeComparable(rawAppName), StringComparison.Ordinal))
        {
            return true;
        }
        if (IsCompactRawToken(rawAppName) && (windowTitle ?? string.Empty).Trim() is { } t && t.Length > 0 && t != rawAppName)
        {
            return true;
        }
        return false;
    }

    // ---------- 行渲染 ----------

    private (Brush CategoryBrush, string CategoryName, string CategoryIcon) CategoryMeta(string? category)
    {
        var (color, icon, name, _) = CategoryStoreSingleton.Instance.GetCategoryMeta(category);
        return (new SolidColorBrush(CategoryStore.ParseColor(color)), name, icon);
    }

    private void UpdateSummaryStrip()
    {
        if (_activities.Count == 0)
        {
            RecordSummaryText.Text = I18n.T("timeline.recordSummary",
                ("dateLabel", IsToday(_selectedDate) ? I18n.T("timeline.todayLabel") : _selectedDate),
                ("count", "0"));
            HourBadgeText.Text = "0";
            return;
        }

        RecordSummaryText.Text = I18n.T("timeline.recordSummary",
            ("dateLabel", IsToday(_selectedDate) ? I18n.T("timeline.todayLabel") : _selectedDate),
            ("count", _activities.Count.ToString()));

        var endLabel = _activities.Count > 0 ? FormatTime(_activities[0].Timestamp) : "--:--";
        HourBadgeText.Text = $"00:00 - {endLabel}";
    }

    private void RebuildRows()
    {
        RowsHost.Children.Clear();

        if (_activities.Count == 0)
        {
            return;
        }

        foreach (var activity in _activities)
        {
            var isFeatured = activity.Id is { } id && _featuredIds.Contains(id);
            RowsHost.Children.Add(BuildRow(activity, isFeatured));
        }

        LoadMoreButton.Visibility = _hasMore ? Visibility.Visible : Visibility.Collapsed;
        LoadMoreButton.Content = I18n.T("timeline.loadMore");
        NoMoreText.Visibility = _hasMore ? Visibility.Collapsed : Visibility.Visible;
    }

    private void RefreshRowFor(Activity updated)
    {
        for (var i = 0; i < RowsHost.Children.Count; i++)
        {
            if (RowsHost.Children[i] is Button { Tag: Activity rowActivity } &&
                rowActivity.Id == updated.Id)
            {
                var isFeatured = updated.Id is { } id && _featuredIds.Contains(id);
                RowsHost.Children[i] = BuildRow(updated, isFeatured);
                return;
            }
        }
    }

    private Button BuildRow(Activity activity, bool featured)
    {
        var (categoryBrush, categoryName, categoryIcon) = CategoryMeta(activity.Category);
        var appName = GetPreferredTimelineAppName(activity.AppName, activity.WindowTitle);
        var title = FormatWindowTitle(activity.WindowTitle, activity.AppName, activity.BrowserUrl);

        var root = new Button { Style = (Style)Application.Current.Resources["WaTimelineRow"], Tag = activity };

        var row = new Grid { ColumnSpacing = 10 };

        row.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto }); // time
        row.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto }); // marker
        if (featured)
        {
            row.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto }); // media
        }
        row.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto }); // icon
        row.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) }); // main
        row.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto }); // duration

        // 时间锚点
        var anchorStack = new StackPanel { VerticalAlignment = VerticalAlignment.Center };
        anchorStack.Children.Add(new TextBlock
        {
            Text = FormatAnchor(activity.Timestamp),
            Style = (Style)Application.Current.Resources["WaMono"],
            FontSize = 12,
            Foreground = (Brush)Application.Current.Resources["WaTextTertiaryBrush"],
        });
        Grid.SetColumn(anchorStack, 0);
        row.Children.Add(anchorStack);

        // 信号轨标记点
        var marker = new Ellipse
        {
            Width = 7,
            Height = 7,
            VerticalAlignment = VerticalAlignment.Center,
            Fill = featured
                ? (Brush)Application.Current.Resources["WaAccentBrush"]
                : new SolidColorBrush(CategoryStore.ParseColor(ColorHexOf(activity))),
        };
        Grid.SetColumn(marker, 1);
        row.Children.Add(marker);

        var mainColumn = 2;

        // 精选条目：缩略图
        if (featured)
        {
            var media = new Border
            {
                Width = 168,
                Height = 94,
                CornerRadius = new CornerRadius(6),
                Background = (Brush)Application.Current.Resources["WaLayerAltBrush"],
                BorderBrush = (Brush)Application.Current.Resources["WaStrokeBrush"],
                BorderThickness = new Thickness(1),
                VerticalAlignment = VerticalAlignment.Center,
                Child = BuildThumbHolder(activity),
            };
            Grid.SetColumn(media, mainColumn);
            row.Children.Add(media);
            mainColumn += 1;
        }

        // 应用图标
        var iconHolder = new Grid { Width = 30, Height = 30, VerticalAlignment = VerticalAlignment.Center };
        var iconBorder = new Border
        {
            Width = 28,
            Height = 28,
            CornerRadius = new CornerRadius(6),
            Background = categoryBrush,
            Child = new TextBlock
            {
                Text = categoryIcon,
                FontSize = 14,
                HorizontalAlignment = HorizontalAlignment.Center,
                VerticalAlignment = VerticalAlignment.Center,
            },
        };
        iconHolder.Children.Add(iconBorder);
        if (!ShouldPreferFallbackIcon(activity.AppName, activity.WindowTitle))
        {
            _ = LoadIconInto(iconHolder, iconBorder, activity);
        }
        Grid.SetColumn(iconHolder, mainColumn);
        row.Children.Add(iconHolder);
        mainColumn += 1;

        // 主区：应用名 + 分类 pill / 标题 / URL
        var main = new StackPanel { Spacing = 2, VerticalAlignment = VerticalAlignment.Center };

        var head = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 6 };
        head.Children.Add(new TextBlock
        {
            Text = appName,
            FontSize = 13,
            FontWeight = Microsoft.UI.Text.FontWeights.SemiBold,
            Foreground = (Brush)Application.Current.Resources["WaTextPrimaryBrush"],
            VerticalAlignment = VerticalAlignment.Center,
        });
        head.Children.Add(new Border
        {
            Background = (Brush)Application.Current.Resources["WaLayerAltBrush"],
            CornerRadius = new CornerRadius(4),
            Padding = new Thickness(6, 1, 6, 2),
            VerticalAlignment = VerticalAlignment.Center,
            Child = new TextBlock
            {
                Text = categoryName,
                FontSize = 11,
                Foreground = (Brush)Application.Current.Resources["WaTextSecondaryBrush"],
            },
        });
        main.Children.Add(head);

        main.Children.Add(new TextBlock
        {
            Text = title,
            FontSize = 13,
            Foreground = (Brush)Application.Current.Resources["WaTextSecondaryBrush"],
            TextTrimming = TextTrimming.CharacterEllipsis,
            MaxLines = 1,
        });

        if (!string.IsNullOrEmpty(activity.BrowserUrl))
        {
            main.Children.Add(new TextBlock
            {
                Text = FormatBrowserUrl(activity.BrowserUrl),
                FontSize = 11,
                Foreground = (Brush)Application.Current.Resources["WaAccentTextBrush"],
                TextTrimming = TextTrimming.CharacterEllipsis,
                MaxLines = 1,
            });
        }

        Grid.SetColumn(main, mainColumn);
        row.Children.Add(main);
        mainColumn += 1;

        // 时长
        var tail = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            Spacing = 6,
            VerticalAlignment = VerticalAlignment.Center,
        };
        tail.Children.Add(new TextBlock
        {
            Text = I18n.FormatDuration(activity.Duration),
            Style = (Style)Application.Current.Resources["WaMono"],
            FontSize = 12,
        });
        tail.Children.Add(new FontIcon
        {
            FontSize = 10,
            Glyph = "\uE76C",
            Foreground = (Brush)Application.Current.Resources["WaTextTertiaryBrush"],
        });
        Grid.SetColumn(tail, mainColumn);
        row.Children.Add(tail);

        root.Content = row;
        root.Click += (_, _) => _ = OpenDetailAsync(activity);
        return root;
    }

    private string ColorHexOf(Activity activity)
    {
        var (color, _, _, _) = CategoryStoreSingleton.Instance.GetCategoryMeta(activity.Category);
        return color;
    }

    private FrameworkElement BuildThumbHolder(Activity activity)
    {
        if (string.IsNullOrEmpty(activity.ScreenshotPath))
        {
            return new FontIcon
            {
                FontSize = 18,
                Glyph = "\uEB9F",
                Foreground = (Brush)Application.Current.Resources["WaTextTertiaryBrush"],
                HorizontalAlignment = HorizontalAlignment.Center,
                VerticalAlignment = VerticalAlignment.Center,
            };
        }

        var image = new Image
        {
            Stretch = Stretch.UniformToFill,
            MaxWidth = 168,
            MaxHeight = 94,
        };
        _ = LoadThumbnailInto(image, activity.ScreenshotPath);
        return image;
    }

    private async Task LoadIconInto(Grid holder, Border fallback, Activity activity)
    {
        var icon = await _icons.GetAsync(activity.AppName, activity.ExecutablePath);
        if (icon is null)
        {
            return;
        }
        if (holder.Children.FirstOrDefault() is Border border)
        {
            border.Child = new Image
            {
                Source = icon,
                Stretch = Stretch.Uniform,
                Width = 26,
                Height = 26,
            };
        }
    }

    private readonly Dictionary<string, BitmapImage> _thumbCache = new();
    private readonly List<string> _thumbOrder = new();
    private const int ThumbCacheLimit = 60;

    private async Task LoadThumbnailInto(Image image, string path)
    {
        var cached = await LoadThumbnailAsync(path);
        if (cached is not null)
        {
            image.Source = cached;
        }
    }

    private async Task<BitmapImage?> LoadThumbnailAsync(string path)
    {
        if (_thumbCache.TryGetValue(path, out var cached))
        {
            return cached;
        }

        try
        {
            var base64 = await EngineApi.InvokeAsync<string>("get_screenshot_thumbnail", new { path });
            if (string.IsNullOrWhiteSpace(base64))
            {
                return null;
            }
            var image = new BitmapImage();
            await image.SetSourceAsync(await Base64ToStream(base64));

            lock (_thumbCache)
            {
                _thumbCache[path] = image;
                _thumbOrder.Add(path);
                while (_thumbOrder.Count > ThumbCacheLimit)
                {
                    var evicted = _thumbOrder[0];
                    _thumbOrder.RemoveAt(0);
                    _thumbCache.Remove(evicted);
                }
            }
            return image;
        }
        catch
        {
            return null;
        }
    }

    private async Task<Windows.Storage.Streams.InMemoryRandomAccessStream> Base64ToStream(string base64)
    {
        var bytes = Convert.FromBase64String(base64);
        var stream = new Windows.Storage.Streams.InMemoryRandomAccessStream();
        using var writer = new Windows.Storage.Streams.DataWriter(stream.GetOutputStreamAt(0));
        writer.WriteBytes(bytes);
        await writer.StoreAsync();
        await writer.FlushAsync();
        writer.DetachStream();
        return stream;
    }

    // ---------- 详情弹窗 ----------

    private async Task OpenDetailAsync(Activity activity)
    {
        var preview = _thumbCache.TryGetValue(activity.ScreenshotPath ?? string.Empty, out var preThumb) ? preThumb : null;

        var detail = activity;
        Task<Activity?> freshTask = activity.Id is { } id
            ? EngineApi.InvokeAsync<Activity?>("get_activity", new { id })
            : Task.FromResult<Activity?>(null);

        Task<BitmapImage?> fullTask = string.IsNullOrEmpty(activity.ScreenshotPath)
            ? Task.FromResult<BitmapImage?>(preview)
            : LoadFullImageAsync(activity.ScreenshotPath);

        await Task.WhenAll(freshTask, fullTask);

        detail = freshTask.Result ?? activity;

        ShowDetailDialog(detail, fullTask.Result ?? preview);
    }

    private readonly Dictionary<string, BitmapImage> _fullCache = new();
    private const int FullCacheLimit = 20;

    private async Task<BitmapImage?> LoadFullImageAsync(string path)
    {
        if (_fullCache.TryGetValue(path, out var cached))
        {
            return cached;
        }
        try
        {
            var base64 = await EngineApi.InvokeAsync<string>("get_screenshot_full", new { path });
            if (string.IsNullOrWhiteSpace(base64))
            {
                return await LoadThumbnailAsync(path);
            }
            var image = new BitmapImage();
            await image.SetSourceAsync(await Base64ToStream(base64));
            lock (_fullCache)
            {
                _fullCache[path] = image;
                while (_fullCache.Count > FullCacheLimit)
                {
                    var oldest = _fullCache.Keys.First();
                    _fullCache.Remove(oldest);
                }
            }
            return image;
        }
        catch
        {
            return await LoadThumbnailAsync(path);
        }
    }

    private async void ShowDetailDialog(Activity activity, BitmapImage? screenshot)
    {
        _selectedActivity = activity;

        var (categoryBrush, categoryName, categoryIcon) = CategoryMeta(activity.Category);
        var appName = GetPreferredTimelineAppName(activity.AppName, activity.WindowTitle);

        var content = new StackPanel { Spacing = 14, MinWidth = 560 };

        // 分类网格
        var categorySection = new StackPanel { Spacing = 8 };
        categorySection.Children.Add(new TextBlock
        {
            Text = I18n.T("timeline.detail.appCategory"),
            FontSize = 13,
            Foreground = (Brush)Application.Current.Resources["WaTextTertiaryBrush"],
        });
        categorySection.Children.Add(new TextBlock
        {
            Text = I18n.T("timeline.detail.appCategoryHelp"),
            FontSize = 11,
            TextWrapping = TextWrapping.Wrap,
            Foreground = (Brush)Application.Current.Resources["WaTextTertiaryBrush"],
        });

        var grid = new Grid { ColumnSpacing = 8, RowSpacing = 8 };
        var categories = CategoryStoreSingleton.Instance.Categories.ToList();
        var columns = 4;
        for (var c = 0; c < Math.Min(columns, Math.Max(1, categories.Count)); c++)
        {
            grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        }
        for (var r = 0; r < (categories.Count + columns - 1) / columns; r++)
        {
            grid.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        }

        for (var i = 0; i < categories.Count; i++)
        {
            var cat = categories[i];
            var displayName = cat.IsCustom ? cat.Name : I18n.TranslateCategoryLabel(cat.Key);
            var isSelected = string.Equals(activity.Category ?? "other", cat.Key, StringComparison.Ordinal);

            var btn = new Button
            {
                Content = new StackPanel
                {
                    Orientation = Orientation.Horizontal,
                    Spacing = 6,
                    HorizontalAlignment = HorizontalAlignment.Center,
                    Children =
                    {
                        new TextBlock { Text = cat.Icon, FontSize = 12 },
                        new TextBlock { Text = displayName, FontSize = 12 },
                    },
                },
                Padding = new Thickness(10, 6, 10, 6),
                CornerRadius = new CornerRadius(4),
                HorizontalAlignment = HorizontalAlignment.Stretch,
                Background = isSelected
                    ? (Brush)Application.Current.Resources["WaAccentBrush"]
                    : (Brush)Application.Current.Resources["WaLayerAltBrush"],
                Foreground = isSelected
                    ? (Brush)Application.Current.Resources["WaTextPrimaryBrush"]
                    : (Brush)Application.Current.Resources["WaTextPrimaryBrush"],
            };
            btn.Click += async (_, _) => await ConfirmAndChangeCategoryAsync(activity, cat);
            Grid.SetRow(btn, i / columns);
            Grid.SetColumn(btn, i % columns);
            grid.Children.Add(btn);
        }

        categorySection.Children.Add(grid);

        // 新建分类
        var createRow = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8 };
        var createButton = new Button
        {
            Content = new StackPanel
            {
                Orientation = Orientation.Horizontal,
                Spacing = 6,
                Children =
                {
                    new TextBlock { Text = "+", FontSize = 12 },
                    new TextBlock { Text = I18n.T("timeline.createCategory"), FontSize = 12 },
                },
            },
            Padding = new Thickness(10, 6, 10, 6),
            CornerRadius = new CornerRadius(4),
            BorderBrush = (Brush)Application.Current.Resources["WaStrokeStrongBrush"],
            BorderThickness = new Thickness(1),
            Background = (Brush)Application.Current.Resources["WaLayerAltBrush"],
        };
        createRow.Children.Add(createButton);
        categorySection.Children.Add(createRow);

        var createPanel = new StackPanel { Spacing = 8, Visibility = Visibility.Collapsed };
        createPanel.Children.Add(new TextBlock
        {
            Text = I18n.T("timeline.createCategoryHint"),
            FontSize = 11,
            TextWrapping = TextWrapping.Wrap,
            Foreground = (Brush)Application.Current.Resources["WaTextTertiaryBrush"],
        });

        var nameBox = new TextBox
        {
            PlaceholderText = I18n.T("timeline.categoryNamePlaceholder"),
            Width = 240,
            CornerRadius = new CornerRadius(4),
        };
        var colorRow = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 6 };
        var selectedColor = CategoryPresetColors[0];
        foreach (var hex in CategoryPresetColors)
        {
            var swatch = new Button
            {
                Width = 28,
                Height = 28,
                CornerRadius = new CornerRadius(4),
                Background = new SolidColorBrush(CategoryStore.ParseColor(hex)),
                BorderThickness = new Thickness(2),
                BorderBrush = hex == selectedColor
                    ? (Brush)Application.Current.Resources["WaTextPrimaryBrush"]
                    : (Brush)Application.Current.Resources["WaStrokeBrush"],
                Tag = hex,
            };
            swatch.Click += (sender, _) =>
            {
                selectedColor = (string)((Button)sender!).Tag!;
                foreach (Button other in colorRow.Children)
                {
                    other.BorderBrush = (string)other.Tag == selectedColor
                        ? (Brush)Application.Current.Resources["WaTextPrimaryBrush"]
                        : (Brush)Application.Current.Resources["WaStrokeBrush"];
                }
            };
            colorRow.Children.Add(swatch);
        }
        var emojiRow = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 2 };
        var selectedEmoji = "🏷️";
        foreach (var emoji in CategoryEmojis)
        {
            var emojiBtn = new Button
            {
                Content = new TextBlock { Text = emoji, FontSize = 14 },
                Width = 34,
                Height = 30,
                Padding = new Thickness(0),
                CornerRadius = new CornerRadius(4),
                Background = (Brush)Application.Current.Resources["WaLayerAltBrush"],
                Tag = emoji,
            };
            emojiBtn.Click += (sender, _) =>
            {
                selectedEmoji = (string)((Button)sender!).Tag!;
                foreach (Button other in emojiRow.Children)
                {
                    other.Background = (string)other.Tag == selectedEmoji
                        ? (Brush)Application.Current.Resources["WaAccentBrush"]
                        : (Brush)Application.Current.Resources["WaLayerAltBrush"];
                }
            };
            emojiRow.Children.Add(emojiBtn);
        }

        createPanel.Children.Add(nameBox);
        createPanel.Children.Add(colorRow);
        createPanel.Children.Add(emojiRow);

        var confirmCreate = new Button
        {
            Content = I18n.T("timeline.confirmChange"),
            Padding = new Thickness(12, 6, 12, 6),
            CornerRadius = new CornerRadius(4),
            HorizontalAlignment = HorizontalAlignment.Right,
        };
        confirmCreate.Click += async (_, _) => await CreateCustomCategoryAsync(nameBox.Text, selectedColor, selectedEmoji);
        createPanel.Children.Add(confirmCreate);

        createButton.Click += (_, _) =>
            createPanel.Visibility = createPanel.Visibility == Visibility.Visible
                ? Visibility.Collapsed
                : Visibility.Visible;
        categorySection.Children.Add(createPanel);

        content.Children.Add(categorySection);

        // 截图预览
        content.Children.Add(new TextBlock
        {
            Text = I18n.T("timeline.detail.screenshot"),
            FontSize = 13,
            Foreground = (Brush)Application.Current.Resources["WaTextTertiaryBrush"],
        });
        var screenshotHost = new Border
        {
            CornerRadius = new CornerRadius(8),
            Background = (Brush)Application.Current.Resources["WaLayerAltBrush"],
            MinHeight = 120,
            MaxHeight = 380,
            HorizontalAlignment = HorizontalAlignment.Center,
            Child = screenshot is null
                ? (FrameworkElement)new FontIcon
                {
                    FontSize = 22,
                    Glyph = "\uEB9F",
                    Foreground = (Brush)Application.Current.Resources["WaTextTertiaryBrush"],
                    HorizontalAlignment = HorizontalAlignment.Center,
                    VerticalAlignment = VerticalAlignment.Center,
                }
                : new ScrollViewer
                {
                    HorizontalScrollBarVisibility = ScrollBarVisibility.Auto,
                    VerticalScrollBarVisibility = ScrollBarVisibility.Auto,
                    Content = new Image { Source = screenshot, Stretch = Stretch.Uniform },
                },
        };
        content.Children.Add(screenshotHost);

        // 窗口标题
        content.Children.Add(new TextBlock
        {
            Text = I18n.T("timeline.detail.windowTitle"),
            FontSize = 13,
            Foreground = (Brush)Application.Current.Resources["WaTextTertiaryBrush"],
        });
        content.Children.Add(new TextBlock
        {
            Text = string.IsNullOrEmpty(activity.WindowTitle) ? I18n.T("timeline.noTitle") : activity.WindowTitle,
            TextWrapping = TextWrapping.Wrap,
            FontSize = 14,
            Foreground = (Brush)Application.Current.Resources["WaTextPrimaryBrush"],
        });

        // 时间 + 时长
        var metaGrid = new Grid { ColumnSpacing = 20 };
        metaGrid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        metaGrid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        var timeColumn = BuildMetaColumn(I18n.T("timeline.detail.recordTime"), FormatTime(activity.Timestamp));
        var durationColumn = BuildMetaColumn(I18n.T("timeline.detail.duration"), I18n.FormatDuration(activity.Duration));
        metaGrid.Children.Add(timeColumn);
        metaGrid.Children.Add(durationColumn);
        Grid.SetColumn(timeColumn, 0);
        Grid.SetColumn(durationColumn, 1);
        content.Children.Add(metaGrid);

        // URL
        if (!string.IsNullOrEmpty(activity.BrowserUrl))
        {
            content.Children.Add(new TextBlock
            {
                Text = I18n.T("timeline.detail.visitedUrl"),
                FontSize = 13,
                Foreground = (Brush)Application.Current.Resources["WaTextTertiaryBrush"],
            });
            var urlButton = new HyperlinkButton
            {
                Content = FormatBrowserUrl(activity.BrowserUrl),
                Padding = new Thickness(0),
            };
            urlButton.Click += (_, _) => OpenExternal(activity.BrowserUrl!);
            content.Children.Add(urlButton);
        }

        var dialog = new ContentDialog
        {
            XamlRoot = XamlRoot,
            Style = (Style)Application.Current.Resources["WaContentDialog"],
            Title = BuildDialogTitle(appName, categoryName),
            Content = new ScrollViewer
            {
                HorizontalScrollBarVisibility = ScrollBarVisibility.Disabled,
                VerticalScrollBarVisibility = ScrollBarVisibility.Auto,
                Content = content,
            },
            CloseButtonText = I18n.T("common.close"),
        };

        try
        {
            await dialog.ShowAsync();
        }
        catch
        {
            // 对话框被新的对话框抢占时忽略
        }
        finally
        {
            _selectedActivity = null;
        }
    }

    private StackPanel BuildDialogTitle(string appName, string categoryName) => new()
    {
        Orientation = Orientation.Horizontal,
        Spacing = 10,
        Children =
        {
            new TextBlock
            {
                Text = appName,
                FontSize = 18,
                FontWeight = Microsoft.UI.Text.FontWeights.SemiBold,
                VerticalAlignment = VerticalAlignment.Center,
            },
            new Border
            {
                Background = (Brush)Application.Current.Resources["WaLayerAltBrush"],
                CornerRadius = new CornerRadius(4),
                Padding = new Thickness(8, 2, 8, 3),
                VerticalAlignment = VerticalAlignment.Center,
                Child = new TextBlock
                {
                    Text = categoryName,
                    FontSize = 11,
                    Foreground = (Brush)Application.Current.Resources["WaTextSecondaryBrush"],
                },
            },
        },
    };

    private static StackPanel BuildMetaColumn(string label, string value) => new()
    {
        Spacing = 4,
        Children =
        {
            new TextBlock
            {
                Text = label,
                FontSize = 13,
                Foreground = (Brush)Application.Current.Resources["WaTextTertiaryBrush"],
            },
            new TextBlock
            {
                Text = value,
                FontSize = 14,
                FontFamily = new FontFamily("Cascadia Mono, Consolas"),
                Foreground = (Brush)Application.Current.Resources["WaTextPrimaryBrush"],
            },
        },
    };

    private async Task ConfirmAndChangeCategoryAsync(Activity activity, CategoryInfo target)
    {
        if (_categorySaving || string.Equals(activity.Category ?? "other", target.Key, StringComparison.Ordinal))
        {
            return;
        }

        var targetName = target.IsCustom ? target.Name : I18n.TranslateCategoryLabel(target.Key);
        var dialog = new ContentDialog
        {
            XamlRoot = XamlRoot,
            Style = (Style)Application.Current.Resources["WaContentDialog"],
            Title = I18n.T("timeline.changeCategoryTitle"),
            Content = I18n.T("timeline.changeCategoryMessage",
                ("appName", activity.AppName), ("category", targetName)),
            PrimaryButtonText = I18n.T("timeline.confirmChange"),
            CloseButtonText = I18n.T("timeline.cancel"),
            DefaultButton = ContentDialogButton.Primary,
        };
        var result = await dialog.ShowAsync();
        if (result != ContentDialogResult.Primary)
        {
            return;
        }

        await ChangeAppCategoryAsync(activity, target.Key, targetName);
    }

    private async Task ChangeAppCategoryAsync(Activity activity, string categoryKey, string targetName)
    {
        _categorySaving = true;
        try
        {
            var affected = await EngineApi.InvokeAsync<long>("set_app_category_rule", new
            {
                app_name = activity.AppName,
                category = categoryKey,
                sync_history = true,
            });

            var matchKey = NormalizeAppKey(activity.AppName);
            foreach (var item in _activities.Where(a => NormalizeAppKey(a.AppName) == matchKey))
            {
                item.Category = categoryKey;
            }

            _featuredIds = SelectFeaturedActivityIds(_activities);
            RebuildRows();

            MainWindow.Toast(InfoBarSeverity.Success, I18n.T("timeline.categoryUpdated",
                ("appName", activity.AppName), ("category", targetName), ("count", affected)));
        }
        catch (Exception e)
        {
            MainWindow.Toast(InfoBarSeverity.Error, I18n.T("timeline.categoryUpdateFailed",
                ("appName", activity.AppName), ("error", e.Message)));
        }
        finally
        {
            _categorySaving = false;
        }
    }

    private async Task CreateCustomCategoryAsync(string name, string color, string icon)
    {
        name = name.Trim();
        if (name.Length == 0)
        {
            MainWindow.Toast(InfoBarSeverity.Error, I18n.T("timeline.categoryNameRequired"));
            return;
        }

        var key = name.ToLowerInvariant() is { } lowered
            ? new string(lowered.Select(c => char.IsAsciiLetterOrDigit(c) || c == '-' ? c : '-').ToArray())
            : string.Empty;
        while (key.Contains("--"))
        {
            key = key.Replace("--", "-");
        }
        key = key.Trim('-');

        if (key.Length == 0)
        {
            var hash = 0;
            foreach (var ch in name)
            {
                hash = (hash * 31 + ch) | 0;
            }
            key = "cat-" + Math.Abs(hash).ToString("x");
        }

        try
        {
            await EngineApi.InvokeAsync("save_custom_category", new
            {
                key,
                name,
                color,
                icon,
            });
            await CategoryStoreSingleton.Instance.RefreshAsync();
            MainWindow.Toast(InfoBarSeverity.Success, I18n.T("timeline.categoryCreated"));
        }
        catch (Exception e)
        {
            MainWindow.Toast(InfoBarSeverity.Error, e.Message);
        }
    }

    // ---------- 交互 ----------

    private void OnDateChanged(CalendarDatePicker sender, CalendarDatePickerDateChangedEventArgs args)
    {
        if (args.NewDate is not { } date)
        {
            return;
        }
        var next = $"{date.Year:D4}-{date.Month:D2}-{date.Day:D2}";
        if (next != _selectedDate)
        {
            _selectedDate = next;
            SyncDatePicker();
            _ = LoadTimelineAsync();
        }
    }

    private void SyncDatePicker()
    {
        if (DateOnly.TryParseExact(_selectedDate, "yyyy-MM-dd", null, System.Globalization.DateTimeStyles.None, out var parsed))
        {
            DatePicker.Date = new DateTimeOffset(parsed.ToDateTime(TimeOnly.MinValue));
        }
    }

    private void OnTodayClicked(object sender, RoutedEventArgs e)
    {
        _selectedDate = TodayString();
        SyncDatePicker();
        _ = LoadTimelineAsync();
    }

    private void OnRefreshClicked(object sender, RoutedEventArgs e)
    {
        _ = LoadTimelineAsync();
    }

    private void OnLoadMoreClicked(object sender, RoutedEventArgs e)
    {
        _ = LoadMoreAsync();
    }

    private void OnOpenSummaryClicked(object sender, RoutedEventArgs e)
    {
        MainWindow.Instance.NavigateToSummary(_selectedDate);
    }

    private void UpdateLiveClock()
    {
        // 实时时钟：选中今天时摘要条结束时间保持“最新记录时间”，无需每秒重绘
        // （避免整表抖动；时间锚点均为记录时间，非相对时长）
    }

    private static void OpenExternal(string url)
    {
        try
        {
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
            {
                FileName = url,
                UseShellExecute = true,
            });
        }
        catch
        {
            // 打开失败静默
        }
    }
}
