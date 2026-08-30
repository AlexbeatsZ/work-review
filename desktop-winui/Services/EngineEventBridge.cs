using System.Text.Json;
using System.Text.Json.Nodes;
using Microsoft.UI.Dispatching;
using WorkReview.Engine;
using WorkReview.Models;

namespace WorkReview.Services;

/// <summary>
/// 引擎事件桥：原生回调 → UI 线程（DispatcherQueue）→ 类型化事件。
/// </summary>
public sealed class EngineEventBridge
{
    private static readonly Lazy<EngineEventBridge> _instance = new(() => new EngineEventBridge());
    public static EngineEventBridge Instance => _instance.Value;

    private DispatcherQueue? _dispatcher;

    /// <summary>新的活动已写入（含截图活动）。</summary>
    public event Action<Activity>? ScreenshotTaken;

    /// <summary>录制状态变化（是否录制 / 是否暂停）。</summary>
    public event Action<bool, bool>? RecordingStateChanged;

    /// <summary>配置已变更（后端在保存后广播）。</summary>
    public event Action? ConfigChanged;

    private EngineEventBridge()
    {
        EngineApi.EngineEvent += OnEngineEvent;
    }

    /// <summary>在 MainWindow 初始化时调用，绑定 UI 线程调度器。</summary>
    public void AttachDispatcher(DispatcherQueue dispatcher)
    {
        _dispatcher = dispatcher;
    }

    private void OnEngineEvent(string name, JsonNode payload)
    {
        var dispatcher = _dispatcher;
        if (dispatcher is null)
        {
            return;
        }

        switch (name)
        {
            case "screenshot-taken":
            {
                var activity = JsonSerializer.Deserialize<Activity>(payload, EngineApi.JsonOptions);
                if (activity is not null)
                {
                    dispatcher.TryEnqueue(() => ScreenshotTaken?.Invoke(activity));
                }
                break;
            }
            case "recording-state-changed":
            {
                var state = payload.Deserialize<RecordingStatePayload>(EngineApi.JsonOptions);
                if (state is not null)
                {
                    dispatcher.TryEnqueue(() => RecordingStateChanged?.Invoke(state.IsRecording, state.IsPaused));
                }
                break;
            }
            case "config-changed":
                dispatcher.TryEnqueue(() => ConfigChanged?.Invoke());
                break;
        }
    }
}
