using System;
using System.Diagnostics;
using System.IO;
using System.Reflection;
using System.Text;
using System.Threading.Tasks;

// Compile as winexe: this task action never allocates a console of its own.
internal static class AgentLauncher
{
    private static string QuoteArgument(string value)
    {
        var quoted = new StringBuilder("\"");
        int slashes = 0;
        foreach (char character in value)
        {
            if (character == '\\')
            {
                slashes++;
                continue;
            }
            quoted.Append('\\', character == '"' ? slashes * 2 + 1 : slashes);
            quoted.Append(character);
            slashes = 0;
        }
        quoted.Append('\\', slashes * 2);
        return quoted.Append('"').ToString();
    }

    private static int Main(string[] args)
    {
        if (args.Length != 1) return 2;
        string dataDir = null;
        try
        {
            dataDir = Path.GetFullPath(args[0]);
            string binary = Path.Combine(
                Path.GetDirectoryName(Assembly.GetExecutingAssembly().Location),
                "work-review-agent.exe");
            var start = new ProcessStartInfo(binary, "--data-dir " + QuoteArgument(dataDir) + " run")
            {
                WorkingDirectory = dataDir,
                UseShellExecute = false,
                CreateNoWindow = true,
                RedirectStandardOutput = true,
                RedirectStandardError = true
            };
            using (var stdout = new FileStream(Path.Combine(dataDir, "agent.stdout.log"),
                FileMode.Append, FileAccess.Write, FileShare.ReadWrite))
            using (var stderr = new FileStream(Path.Combine(dataDir, "agent.stderr.log"),
                FileMode.Append, FileAccess.Write, FileShare.ReadWrite))
            using (var process = new Process { StartInfo = start })
            {
                process.Start();
                Task.WaitAll(
                    process.StandardOutput.BaseStream.CopyToAsync(stdout),
                    process.StandardError.BaseStream.CopyToAsync(stderr));
                process.WaitForExit();
                return process.ExitCode;
            }
        }
        catch (Exception error)
        {
            if (dataDir != null)
            {
                try
                {
                    File.AppendAllText(Path.Combine(dataDir, "agent.stderr.log"),
                        DateTimeOffset.Now.ToString("o") + " launcher: " + error + Environment.NewLine);
                }
                catch { }
            }
            return 1;
        }
    }
}
