using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using WorkReview.Engine;
using WorkReview.Models;
using WorkReview.Services;

namespace WorkReview.Views;

/// <summary>时段汇总页：意图分布 + 小时摘要带。</summary>
public sealed partial class SummaryPage : UserControl
{
    private List<HourlySummary> _summaries = new();
    private List<IntentSummary> _intentSummary = new();
    private HashSet<int> _expandedHours = new();
    private string _selectedDate = TodayString();
    private bool _loading;
    private int _loadRequestId;

    public SummaryPage()
    {
        InitializeComponent();
        ApplyLocale();
        SyncDatePicker();
    }

    public void OnActivated()
    {
        if (!_loading)
        {
            _ = LoadAsync();
        }
    }

    public void SetDate(string date)
    {
        _selectedDate = date;
        SyncDatePicker();
    }

    public void ApplyLocale()
    {
        PageTitle.Text = I18n.T("timelineSummary.title");
        PageDescription.Text = I18n.T("timelineSummary.description");
        TodayButton.Content = I18n.T("datePicker.today");
        EmptyText.Text = I18n.T("timelineSummary.noData");
        IntentTitle.Text = I18n.T("timelineSummary.intentDistribution.title");
        IntentDescription.Text = I18n.T("timelineSummary.intentDistribution.description");
        IntentEmpty.Text = I18n.T("timelineSummary.intentDistribution.empty");
    }

    private static string TodayString()
    {
        var now = DateTime.Now;
        return $"{now.Year:D4}-{now.Month:D2}-{now.Day:D2}";
    }

    private async Task LoadAsync()
    {
        var requestId = ++_loadRequestId;
        _loading = true;
        ShowState(showLoading: true);

        try
        {
            var summariesTask = EngineApi.InvokeAsync<List<HourlySummary>>("get_hourly_summaries", new
            {
                date = _selectedDate,
            });
            var intentsTask = EngineApi.InvokeAsync<IntentAnalysisResult>("recognize_work_intents", new
            {
                date_from = _selectedDate,
                date_to = _selectedDate,
                limit = 5000,
            });
            await Task.WhenAll(summariesTask, intentsTask);

            if (requestId != _loadRequestId)
            {
                return;
            }

            _summaries = summariesTask.Result;
            _intentSummary = intentsTask.Result.Summary;

            if (_summaries.Count == 0 && _intentSummary.Count == 0)
            {
                ShowState(showEmpty: true);
                return;
            }

            RenderIntentList();
            RenderHourBands();
            ShowState(showContent: true);
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

    private void ShowState(
        bool showLoading = false,
        bool showContent = false,
        bool showError = false,
        bool showEmpty = false)
    {
        LoadingRing.IsActive = showLoading;
        LoadingRing.Visibility = showLoading ? Visibility.Visible : Visibility.Collapsed;
        ContentScroll.Visibility = showContent ? Visibility.Visible : Visibility.Collapsed;
        ErrorBanner.Visibility = showError ? Visibility.Visible : Visibility.Collapsed;
        EmptyState.Visibility = showEmpty ? Visibility.Visible : Visibility.Collapsed;
    }

    // ---------- 意图分布 ----------

    private void RenderIntentList()
    {
        IntentList.Children.Clear();
        IntentCount.Text = _intentSummary.Count.ToString();

        var totalDuration = _intentSummary.Sum(i => Math.Max(0, i.Duration));
        IntentEmpty.Visibility = _intentSummary.Count > 0 ? Visibility.Collapsed : Visibility.Visible;

        foreach (var item in _intentSummary)
        {
            var duration = Math.Max(0, item.Duration);
            var width = totalDuration > 0 ? Math.Max(8, (int)Math.Round(duration * 100.0 / totalDuration)) : 0;

            var row = new StackPanel { Spacing = 5 };

            row.Children.Add(new StackPanel
            {
                Orientation = Orientation.Horizontal,
                Spacing = 8,
                Children =
                {
                    new TextBlock
                    {
                        Text = item.Label,
                        FontSize = 13,
                        FontWeight = Microsoft.UI.Text.FontWeights.SemiBold,
                        Foreground = (Brush)Application.Current.Resources["WaTextPrimaryBrush"],
                    },
                    new TextBlock
                    {
                        Text = $"{I18n.FormatDuration(duration)} · {I18n.T("timelineSummary.intentDistribution.sessions", ("count", item.SessionCount))}",
                        FontSize = 11,
                        Foreground = (Brush)Application.Current.Resources["WaTextTertiaryBrush"],
                        VerticalAlignment = VerticalAlignment.Center,
                    },
                },
            });

            row.Children.Add(new Border
            {
                Height = 6,
                CornerRadius = new CornerRadius(3),
                Background = (Brush)Application.Current.Resources["WaLayerAltBrush"],
                Child = new Border
                {
                    Width = new GridLength(width, GridUnitType.Star).Value,
                    HorizontalAlignment = HorizontalAlignment.Left,
                    CornerRadius = new CornerRadius(3),
                    Background = (Brush)Application.Current.Resources["WaAccentBrush"],
                },
            });

            IntentList.Children.Add(row);
        }
    }

    // ---------- 小时带 ----------

    private static string NormalizeSummary(string? text) =>
        System.Text.RegularExpressions.Regex.Replace(text ?? string.Empty, @"\s+", " ").Trim();

    private static string[] SplitSentences(string text) =>
        System.Text.RegularExpressions.Regex.Split(text, @"[。！？!?]")
            .Select(s => s.Trim())
            .Where(s => s.Length > 0)
            .ToArray();

    private static string[] SplitClauses(string text) =>
        text.Split(new[] { '，', ',', '；', ';', '、' })
            .Select(s => s.Trim())
            .Where(s => s.Length > 0)
            .ToArray();

    private static string EnsureChineseStop(string text) =>
        string.IsNullOrEmpty(text) ? string.Empty : $"{System.Text.RegularExpressions.Regex.Replace(text, @"[。！？!?]+$", "").Trim()}。";

    private static string GetFullSummary(string text) => NormalizeSummary(text);

    private static string GetPrimarySummary(string text)
    {
        var normalized = NormalizeSummary(text);
        if (normalized.Length == 0)
        {
            return string.Empty;
        }
        var firstSentence = SplitSentences(normalized).FirstOrDefault() ?? normalized;
        var clauses = SplitClauses(firstSentence);
        return clauses.Length >= 3
            ? string.Join("，", clauses.Take(2))
            : firstSentence;
    }

    private static string GetSecondarySummary(string text)
    {
        var normalized = NormalizeSummary(text);
        if (normalized.Length == 0)
        {
            return string.Empty;
        }
        var sentences = SplitSentences(normalized);
        var firstSentence = sentences.FirstOrDefault() ?? normalized;
        var clauses = SplitClauses(firstSentence);

        if (clauses.Length >= 3)
        {
            return EnsureChineseStop(string.Join("，", clauses.Skip(2)));
        }
        if (sentences.Length > 1)
        {
            return EnsureChineseStop(string.Join("。", sentences.Skip(1)));
        }
        return string.Empty;
    }

    private static string[] GetMainApps(string mainApps) =>
        (mainApps ?? string.Empty)
            .Split(new[] { '，', ',' })
            .Select(s => s.Trim())
            .Where(s => s.Length > 0)
            .Take(4)
            .ToArray();

    private bool NeedsExpand(HourlySummary summary)
    {
        var full = GetFullSummary(summary.Summary);
        var primary = GetPrimarySummary(summary.Summary);
        var secondary = GetSecondarySummary(summary.Summary);
        var displayed = string.Concat(new[] { primary, secondary }.Where(s => !string.IsNullOrEmpty(s)));
        return full.Length > displayed.Length + 2;
    }

    private (string Tone, string Label) RhythmMeta(long totalDuration)
    {
        if (totalDuration >= 45 * 60)
        {
            return ("deep", "深度推进");
        }
        if (totalDuration >= 20 * 60)
        {
            return ("steady", "持续推进");
        }
        return ("light", "轻量切换");
    }

    private void RenderHourBands()
    {
        HourBands.Children.Clear();
        var peak = _summaries.Count > 1 ? _summaries.Max(s => Math.Max(0, s.TotalDuration)) : 0;

        foreach (var summary in _summaries)
        {
            var apps = GetMainApps(summary.MainApps);
            var isPeak = peak > 0 && summary.TotalDuration == peak;
            var expanded = _expandedHours.Contains(summary.Hour);
            var canExpand = NeedsExpand(summary);
            var rhythm = RhythmMeta(summary.TotalDuration);

            var band = new Grid { ColumnSpacing = 12 };

            band.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(86) });
            band.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });

            var anchor = new StackPanel { Spacing = 2, VerticalAlignment = VerticalAlignment.Center };
            anchor.Children.Add(new TextBlock
            {
                Text = $"{summary.Hour:D2}:00",
                Style = (Style)Application.Current.Resources["WaMono"],
                FontSize = 15,
                FontWeight = Microsoft.UI.Text.FontWeights.SemiBold,
            });
            anchor.Children.Add(new TextBlock
            {
                Text = I18n.FormatDuration(summary.TotalDuration),
                FontSize = 11,
                Foreground = (Brush)Application.Current.Resources["WaTextTertiaryBrush"],
            });
            Grid.SetColumn(anchor, 0);
            band.Children.Add(anchor);

            var card = new Border
            {
                Background = isPeak
                    ? (Brush)Application.Current.Resources["WaLayerFloatingBrush"]
                    : (Brush)Application.Current.Resources["WaLayerBrush"],
                BorderBrush = isPeak
                    ? (Brush)Application.Current.Resources["WaStrokeStrongBrush"]
                    : (Brush)Application.Current.Resources["WaStrokeBrush"],
                BorderThickness = new Thickness(1),
                CornerRadius = new CornerRadius(8),
                Padding = new Thickness(14, 10, 14, 12),
                Child = BuildBandBody(summary, apps, isPeak, expanded, canExpand, rhythm),
            };
            Grid.SetColumn(card, 1);
            band.Children.Add(card);

            HourBands.Children.Add(band);
        }
    }

    private StackPanel BuildBandBody(
        HourlySummary summary,
        string[] apps,
        bool isPeak,
        bool expanded,
        bool canExpand,
        (string Tone, string Label) rhythm)
    {
        var body = new StackPanel { Spacing = 8 };

        var header = new Grid { ColumnSpacing = 10 };
        header.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        header.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        header.Children.Add(new TextBlock
        {
            Text = expanded
                ? (GetFullSummary(summary.Summary) is { Length: > 0 } full ? full : I18n.T("timelineSummary.noData"))
                : (GetPrimarySummary(summary.Summary) is { Length: > 0 } primary ? primary : I18n.T("timelineSummary.noData")),
            TextWrapping = TextWrapping.Wrap,
            FontSize = 14,
            Foreground = (Brush)Application.Current.Resources["WaTextPrimaryBrush"],
        });
        if (isPeak)
        {
            var badge = new Border
            {
                Background = (Brush)Application.Current.Resources["WaAccentBrush"],
                CornerRadius = new CornerRadius(4),
                Padding = new Thickness(8, 2, 8, 3),
                VerticalAlignment = VerticalAlignment.Center,
                Child = new TextBlock
                {
                    Text = I18n.T("timelineSummary.peakBadge"),
                    FontSize = 11,
                    FontWeight = Microsoft.UI.Text.FontWeights.SemiBold,
                    Foreground = (Brush)Application.Current.Resources["WaTextPrimaryBrush"],
                },
            };
            Grid.SetColumn(badge, 1);
            header.Children.Add(badge);
        }
        body.Children.Add(header);

        // 元信息行：节奏 + 应用数
        var metaRow = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8 };
        metaRow.Children.Add(new Border
        {
            Background = (Brush)Application.Current.Resources["WaLayerAltBrush"],
            CornerRadius = new CornerRadius(4),
            Padding = new Thickness(8, 2, 8, 3),
            Child = new TextBlock
            {
                Text = rhythm.Label,
                FontSize = 11,
                Foreground = (Brush)Application.Current.Resources["WaAccentTextBrush"],
            },
        });
        if (apps.Length > 0)
        {
            metaRow.Children.Add(new TextBlock
            {
                Text = I18n.T("timelineSummary.appsCount", ("count", apps.Length)),
                FontSize = 11,
                VerticalAlignment = VerticalAlignment.Center,
                Foreground = (Brush)Application.Current.Resources["WaTextTertiaryBrush"],
            });
        }
        body.Children.Add(metaRow);

        if (!expanded && GetSecondarySummary(summary.Summary) is { Length: > 0 } secondary)
        {
            body.Children.Add(new TextBlock
            {
                Text = secondary,
                TextWrapping = TextWrapping.Wrap,
                FontSize = 12,
                Foreground = (Brush)Application.Current.Resources["WaTextSecondaryBrush"],
            });
        }

        if (canExpand)
        {
            var expandButton = new HyperlinkButton
            {
                Content = new StackPanel
                {
                    Orientation = Orientation.Horizontal,
                    Spacing = 4,
                    Children =
                    {
                        new TextBlock
                        {
                            Text = expanded ? I18n.T("timelineSummary.collapse") : I18n.T("timelineSummary.expandFull"),
                            FontSize = 12,
                        },
                        new FontIcon { FontSize = 9, Glyph = expanded ? "\uE70E" : "\uE70D" },
                    },
                },
                Padding = new Thickness(0),
            };
            expandButton.Click += (_, _) =>
            {
                if (_expandedHours.Contains(summary.Hour))
                {
                    _expandedHours.Remove(summary.Hour);
                }
                else
                {
                    _expandedHours.Add(summary.Hour);
                }
                RenderHourBands();
            };
            body.Children.Add(expandButton);
        }

        if (apps.Length > 0)
        {
            var tags = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 6 };
            foreach (var app in apps)
            {
                tags.Children.Add(new Border
                {
                    Background = (Brush)Application.Current.Resources["WaLayerAltBrush"],
                    BorderBrush = (Brush)Application.Current.Resources["WaStrokeBrush"],
                    BorderThickness = new Thickness(1),
                    CornerRadius = new CornerRadius(4),
                    Padding = new Thickness(8, 2, 8, 3),
                    Child = new TextBlock
                    {
                        Text = app,
                        FontSize = 11,
                        Foreground = (Brush)Application.Current.Resources["WaTextSecondaryBrush"],
                    },
                });
            }
            body.Children.Add(tags);
        }

        return body;
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
            _ = LoadAsync();
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
        _ = LoadAsync();
    }
}
