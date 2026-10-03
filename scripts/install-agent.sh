#!/bin/bash
set -euo pipefail
label=io.work-review.agent
plist="$HOME/Library/LaunchAgents/$label.plist"
domain="gui/$(id -u)"
if [[ "${1:-}" == --uninstall ]]; then
  /bin/launchctl bootout "$domain/$label" 2>/dev/null || true
  /bin/rm -f "$plist"
  printf '%s\n' 'LaunchAgent removed; configuration and records preserved.'
  exit 0
fi
binary="${1:?Usage: install-agent.sh /absolute/path/work-review-agent [data-directory]}"
data="${2:-$HOME/Library/Application Support/work-review-agent}"
[[ "$binary" == /* && "$data" == /* && -x "$binary" ]] || { printf '%s\n' 'Use absolute paths and an executable binary.' >&2; exit 1; }
[[ -f "$data/config.json" ]] || { printf '%s\n' 'Run agent init with this data directory first.' >&2; exit 1; }
mkdir -p "$HOME/Library/LaunchAgents" "$data/bin" /tmp/.agents
temporary=$(mktemp /tmp/.agents/work-review-launchagent.XXXXXX)
trap 'rm -f "$temporary"' EXIT
/bin/launchctl bootout "$domain/$label" 2>/dev/null || true
if [[ "$binary" != "$data/bin/work-review-agent" ]]; then /usr/bin/install -m 755 "$binary" "$data/bin/work-review-agent"; fi
/usr/bin/plutil -create xml1 "$temporary"
/usr/bin/plutil -insert Label -string "$label" "$temporary"
/usr/bin/plutil -insert ProgramArguments -xml '<array/>' "$temporary"
/usr/bin/plutil -insert ProgramArguments.0 -string "$data/bin/work-review-agent" "$temporary"
/usr/bin/plutil -insert ProgramArguments.1 -xml '<string>--data-dir</string>' "$temporary"
/usr/bin/plutil -insert ProgramArguments.2 -string "$data" "$temporary"
/usr/bin/plutil -insert ProgramArguments.3 -string run "$temporary"
/usr/bin/plutil -insert WorkingDirectory -string "$data" "$temporary"
/usr/bin/plutil -insert RunAtLoad -bool true "$temporary"
/usr/bin/plutil -insert KeepAlive -bool true "$temporary"
/usr/bin/plutil -insert ThrottleInterval -integer 30 "$temporary"
/usr/bin/plutil -insert ProcessType -string Background "$temporary"
/usr/bin/plutil -insert LimitLoadToSessionType -string Aqua "$temporary"
/usr/bin/plutil -insert StandardOutPath -string "$data/agent.stdout.log" "$temporary"
/usr/bin/plutil -insert StandardErrorPath -string "$data/agent.stderr.log" "$temporary"
/usr/bin/plutil -lint "$temporary"
/bin/mv "$temporary" "$plist"
/bin/launchctl bootstrap "$domain" "$plist"
printf 'Background collector started. Logs: %s\n' "$data"
