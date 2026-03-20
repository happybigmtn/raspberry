#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 7 ]; then
  echo "usage: $0 <manifest> <raspberry_bin> <fabro_bin> <pid_file> <run_log> <snapshot_log> <autodev_json>" >&2
  exit 1
fi

manifest_path=$1
raspberry_bin=$2
fabro_bin=$3
pid_file=$4
run_log=$5
snapshot_log=$6
autodev_json=$7

interval_secs=${INTERVAL_SECS:-300}
max_parallel=${MAX_PARALLEL:-5}
max_cycles=${MAX_CYCLES:-40000}
poll_interval_ms=${POLL_INTERVAL_MS:-1000}
evolve_every_seconds=${EVOLVE_EVERY_SECS:-0}

mkdir -p "$(dirname "$pid_file")" "$(dirname "$run_log")" "$(dirname "$snapshot_log")"

timestamp() {
  date -u +"%Y-%m-%dT%H:%M:%SZ"
}

current_pid() {
  if [ -f "$pid_file" ]; then
    tr -d '[:space:]' <"$pid_file"
  fi
}

pid_is_alive() {
  local pid
  pid=$(current_pid)
  [ -n "${pid:-}" ] && kill -0 "$pid" 2>/dev/null
}

report_has_pending_work() {
  if [ ! -f "$autodev_json" ]; then
    return 0
  fi
  python - "$autodev_json" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
try:
    data = json.loads(path.read_text())
except Exception:
    raise SystemExit(0)

current = data.get("current") or {}
ready = int(current.get("ready", 0) or 0)
running = int(current.get("running", 0) or 0)
failed = int(current.get("failed", 0) or 0)
raise SystemExit(0 if (ready or running or failed) else 1)
PY
}

start_autodev() {
  local ts
  ts=$(timestamp)
  echo "[$ts] starting autodev for $manifest_path" >>"$run_log"
  setsid "$raspberry_bin" autodev \
    --manifest "$manifest_path" \
    --fabro-bin "$fabro_bin" \
    --max-parallel "$max_parallel" \
    --max-cycles "$max_cycles" \
    --poll-interval-ms "$poll_interval_ms" \
    --evolve-every-seconds "$evolve_every_seconds" \
    >>"$run_log" 2>&1 < /dev/null &
  printf "%s\n" "$!" >"$pid_file"
}

write_fallback_summary() {
  if [ ! -f "$autodev_json" ]; then
    echo "autodev_report=missing path=$autodev_json" >>"$snapshot_log"
  else
    python - "$autodev_json" >>"$snapshot_log" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
try:
    data = json.loads(path.read_text())
except Exception as exc:
    print(f"autodev_report=unreadable path={path} error={exc}")
    raise SystemExit(0)

current = data.get("current", {})
print(
    "autodev_report"
    f" updated_at={data.get('updated_at', 'unknown')}"
    f" ready={current.get('ready', 'unknown')}"
    f" running={current.get('running', 'unknown')}"
    f" blocked={current.get('blocked', 'unknown')}"
    f" failed={current.get('failed', 'unknown')}"
    f" complete={current.get('complete', 'unknown')}"
)
running = current.get("running_lanes") or []
failed = current.get("failed_lanes") or []
if running:
    print("running_lanes=" + ", ".join(running))
if failed:
    print("failed_lanes=" + ", ".join(failed))
PY
  fi
  local state_dir
  state_dir=$(dirname "$autodev_json")
  python - "$state_dir" >>"$snapshot_log" <<'PY'
import json
import pathlib
import sys

state_dir = pathlib.Path(sys.argv[1])
for path in sorted(state_dir.glob("*-state.json")):
    try:
        data = json.loads(path.read_text())
    except Exception:
        continue
    program = data.get("program")
    lanes = data.get("lanes", {})
    running = []
    failed = []
    for lane in lanes.values():
        status = lane.get("status")
        lane_key = lane.get("lane_key", "unknown")
        if status == "running":
            stage = lane.get("current_stage_label")
            running.append(f"{lane_key}@{stage}" if stage else lane_key)
        elif status == "failed":
            failed.append(lane_key)
    if running or failed:
        print(
            f"child_state program={program} running={', '.join(running) if running else 'none'}"
            f" failed={', '.join(failed) if failed else 'none'}"
        )
PY
}

write_snapshot() {
  local ts pid pending_work
  ts=$(timestamp)
  pid=$(current_pid)
  if report_has_pending_work; then
    pending_work=yes
  else
    pending_work=no
  fi
  {
    echo "=== $ts ==="
    if pid_is_alive; then
      echo "autodev_pid=$pid alive=yes pending_work=$pending_work"
    else
      echo "autodev_pid=${pid:-missing} alive=no pending_work=$pending_work"
      if [ "$pending_work" = no ]; then
        echo "autodev_idle=settled"
      fi
    fi
  } >>"$snapshot_log"

  if timeout 45s "$raspberry_bin" status --manifest "$manifest_path" >>"$snapshot_log" 2>&1; then
    echo >>"$snapshot_log"
    return
  fi

  echo "status_command=timeout_or_error" >>"$snapshot_log"
  write_fallback_summary
  echo >>"$snapshot_log"
}

while true; do
  if ! pid_is_alive; then
    if report_has_pending_work; then
      start_autodev
      sleep 2
    fi
  fi
  write_snapshot
  sleep "$interval_secs"
done
