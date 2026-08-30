using Microsoft.UI.Xaml;
using System.Text.Json.Nodes;
using WorkReview.Engine;
using WorkReview.Services;

namespace WorkReview;

public partial class App : Application
{
    private MainWindow? _mainWindow;

    // 单实例：第二个实例唤醒第一个实例后退出
    private static Mutex? _singleInstanceMutex;
    private const string MutexName = "Local\\WorkReview.SingleInstance";
    private const string ActivateEventName = "Local\\WorkReview.Activate";

    public App()
    {
        // 产品契约：始终深色运行
        RequestedTheme = ApplicationTheme.Dark;
        InitializeComponent();

        UnhandledException += (_, e) =>
        {
            Crash($"UnhandledException: {e.Message}\n{e.Exception?.StackTrace}");
            e.Handled = false;
        };
        AppDomain.CurrentDomain.UnhandledException += (_, e) =>
            Crash($"AppDomain: {e.ExceptionObject}");

        // 引擎在此同步启动（配置、数据库、采集线程）
        Stage("engine-start");
        EngineApi.Initialize();
        Stage("engine-started");
    }

    public static void Crash(string line)
    {
        try
        {
            File.AppendAllText(
                Path.Combine(Path.GetTempPath(), "workreview-ui.log"),
                $"[{DateTime.Now:HH:mm:ss.fff}] {line}\n");
        }
        catch
        {
        }
    }

    public static void Stage(string line)
    {
        Crash($"[stage] {line}");
    }

    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        _singleInstanceMutex = new Mutex(true, MutexName, out var isFirst);
        if (!isFirst)
        {
            // 唤醒已有实例后退出
            if (System.Threading.EventWaitHandle.TryOpenExisting(ActivateEventName, out var ev))
            {
                ev.Set();
                ev.Dispose();
            }
            Environment.Exit(0);
            return;
        }

        GC.KeepAlive(_singleInstanceMutex);

        var startHidden = Environment.GetCommandLineArgs().Any(
            a => a is "--hidden" or "--minimized" or "--autostart");

        // UI 偏好目录与引擎数据目录一致
        try
        {
            var dataDir = EngineApi.Invoke("get_data_dir")?.GetValue<string>();
            AppPreferences.DataDir = dataDir ?? string.Empty;
            AppPreferences.Load();
        }
        catch
        {
            // 数据目录不可用时退化为临时偏好
            AppPreferences.DataDir = Path.GetTempPath();
        }

        Stage("prefs-loaded");
        I18n.Initialize(AppPreferences.Get("locale"));
        Stage("i18n-ready");

        _mainWindow = new MainWindow();
        _mainWindow.Activate();

        if (startHidden)
        {
            _mainWindow.HideToTray();
        }

        var activateEvent = new System.Threading.EventWaitHandle(
            false, System.Threading.EventResetMode.AutoReset, ActivateEventName);
        _ = Task.Run(() =>
        {
            while (activateEvent.WaitOne())
            {
                var window = _mainWindow;
                window?.DispatcherQueue.TryEnqueue(window.ShowFromTray);
            }
        });
    }
}
