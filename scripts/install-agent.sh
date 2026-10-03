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
binary="${1:?Usage: install-agent.sh /absolute/path/work-review-agent [data-directory] [signing-identity]}"
data="${2:-$HOME/Library/Application Support/work-review-agent}"
identity="${3:-}"
[[ "$binary" == /* && "$data" == /* && -x "$binary" ]] || { printf '%s\n' 'Use absolute paths and an executable binary.' >&2; exit 1; }
[[ -f "$data/config.json" ]] || { printf '%s\n' 'Run agent init with this data directory first.' >&2; exit 1; }
current="$data/bin/work-review-agent"
# Keep an existing deployment's program path so privacy grants survive reinstalls.
if [[ -f "$plist" ]]; then
  installed_data=$(/usr/bin/plutil -extract ProgramArguments.2 raw -o - "$plist")
  installed_binary=$(/usr/bin/plutil -extract ProgramArguments.0 raw -o - "$plist")
  [[ "$installed_data" == "$data" ]] || { printf '%s\n' 'Installed LaunchAgent uses another data directory; uninstall it explicitly before switching.' >&2; exit 1; }
  [[ "$installed_binary" == /* && "${installed_binary##*/}" == work-review-agent ]] || { printf '%s\n' 'Invalid installed program path.' >&2; exit 1; }
  current="$installed_binary"
fi
mkdir -p "$HOME/Library/LaunchAgents" "${current%/*}" /tmp/.agents
temporary=$(mktemp /tmp/.agents/work-review-launchagent.XXXXXX)
staged=$(mktemp "${current%/*}/.work-review-agent.XXXXXX")
trap 'rm -f "$temporary" "$staged"' EXIT
/usr/bin/install -m 755 "$binary" "$staged"
if [[ -n "$identity" ]]; then
  /usr/bin/codesign --force --sign "$identity" --identifier io.work-review.agent "$staged"
fi
/usr/bin/codesign --verify --strict "$staged"
# Certificate-backed identity must survive an update; reject an unsigned replacement.
if [[ -x "$current" ]]; then
  requirement=$(/usr/bin/codesign --display -r- "$current" 2>&1)
  requirement="${requirement##*designated => }"
  if [[ "$requirement" == *certificate* ]]; then
    /usr/bin/codesign --verify --strict -R "=$requirement" "$staged" || {
      printf '%s\n' 'Update does not preserve the installed signing identity. Pass the original signing identity as the third argument.' >&2
      exit 1
    }
  fi
fi
/bin/launchctl bootout "$domain/$label" 2>/dev/null || true
/bin/mv "$staged" "$current"
/usr/bin/plutil -create xml1 "$temporary"
/usr/bin/plutil -insert Label -string "$label" "$temporary"
/usr/bin/plutil -insert ProgramArguments -xml '<array/>' "$temporary"
/usr/bin/plutil -insert ProgramArguments.0 -string "$current" "$temporary"
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
