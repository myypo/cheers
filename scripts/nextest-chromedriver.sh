#!/usr/bin/env bash
set -euo pipefail

if [[ -z "${NEXTEST_ENV:-}" ]]; then
    echo "NEXTEST_ENV is not set; this script must be run by cargo-nextest" >&2
    exit 1
fi

webdriver_url="${WEBDRIVER_URL:-http://127.0.0.1:9515}"
webdriver_host_port="${webdriver_url#http://}"
webdriver_host_port="${webdriver_host_port%%/*}"
webdriver_host="${webdriver_host_port%%:*}"
webdriver_port="${webdriver_host_port##*:}"
webdriver_url_from_env="${WEBDRIVER_URL:-}"

if [[ -z "$webdriver_host" || -z "$webdriver_port" || "$webdriver_host" == "$webdriver_port" ]]; then
    echo "unsupported WEBDRIVER_URL '$webdriver_url'; expected http://host:port" >&2
    exit 1
fi

log_dir="${CARGO_TARGET_DIR:-target}/nextest"
mkdir -p "$log_dir"
log_file="$log_dir/chromedriver-$webdriver_port.log"
pid_file="$log_dir/chromedriver-$webdriver_port.pid"

webdriver_ready() {
    local response
    # shellcheck disable=SC2016 # Inner shell expands positional parameters.
    response="$(timeout 1 bash -c '
      exec 3<>/dev/tcp/"$1"/"$2"
      printf "GET /status HTTP/1.1\r\nHost: %s:%s\r\nConnection: close\r\n\r\n" "$1" "$2" >&3
      dd bs=4096 count=1 <&3 2>/dev/null
    ' _ "$webdriver_host" "$webdriver_port" 2>/dev/null)" || return 1
    grep -q '"ready"[[:space:]]*:[[:space:]]*true' <<<"$response"
}

managed_webdriver_pid() {
    local pid args

    if [[ ! -f "$pid_file" ]]; then
        return 1
    fi

    read -r pid <"$pid_file" || return 1
    if [[ ! "$pid" =~ ^[0-9]+$ ]]; then
        rm -f "$pid_file"
        return 1
    fi

    args="$(ps -p "$pid" -o args= 2>/dev/null || true)"
    if [[ "$args" != *chromedriver* ]]; then
        rm -f "$pid_file"
        return 1
    fi

    echo "$pid"
}

stop_managed_webdriver() {
    local pid
    pid="$(managed_webdriver_pid)" || return 1

    echo "stopping stale managed WebDriver at $webdriver_url" >&2
    kill "$pid" >/dev/null 2>&1 || true
    for _ in {1..50}; do
        if ! kill -0 "$pid" >/dev/null 2>&1; then
            rm -f "$pid_file"
            return 0
        fi
        sleep 0.1
    done

    kill -9 "$pid" >/dev/null 2>&1 || true
    rm -f "$pid_file"
}

if webdriver_ready; then
    if [[ -n "$webdriver_url_from_env" ]]; then
        echo "using existing WebDriver at $webdriver_url" >&2
        echo "WEBDRIVER_URL=$webdriver_url" >>"$NEXTEST_ENV"
        exit 0
    fi

    if stop_managed_webdriver; then
        for _ in {1..50}; do
            if ! webdriver_ready; then
                break
            fi
            sleep 0.1
        done

        if webdriver_ready; then
            echo "managed WebDriver at $webdriver_url did not stop" >&2
            exit 1
        fi
    else
        echo "using existing unmanaged WebDriver at $webdriver_url" >&2
        echo "WEBDRIVER_URL=$webdriver_url" >>"$NEXTEST_ENV"
        exit 0
    fi
fi

if ! command -v chromedriver >/dev/null 2>&1; then
    echo "chromedriver not found in PATH" >&2
    exit 1
fi

find_nextest_pid() {
    local pid comm args
    pid="$PPID"
    while [[ -n "$pid" && "$pid" != "1" ]]; do
        comm="$(ps -p "$pid" -o comm= 2>/dev/null || true)"
        args="$(ps -p "$pid" -o args= 2>/dev/null || true)"
        if [[ "$comm" == *cargo-nextest* || "$args" == *cargo-nextest* || "$args" == *"cargo nextest"* ]]; then
            echo "$pid"
            return 0
        fi
        pid="$(ps -p "$pid" -o ppid= 2>/dev/null | tr -d '[:space:]')"
    done
}

nextest_pid="$(find_nextest_pid)"

# Start ChromeDriver detached from this setup script so nextest doesn't treat the
# long-running server as a leaked child process. A small watcher tears it down
# when the cargo-nextest process exits.
# shellcheck disable=SC2016 # Inner script expands its own positional parameters.
setsid bash -c '
  set -euo pipefail
  port="$1"
  log_file="$2"
  pid_file="$3"
  nextest_pid="$4"

  nohup chromedriver --port="$port" >"$log_file" 2>&1 &
  driver_pid="$!"
  echo "$driver_pid" >"$pid_file"

  if [[ -n "$nextest_pid" ]]; then
    (
      while kill -0 "$nextest_pid" >/dev/null 2>&1; do
        sleep 1
      done
      kill "$driver_pid" >/dev/null 2>&1 || true
      wait "$driver_pid" >/dev/null 2>&1 || true
      rm -f "$pid_file"
    ) >/dev/null 2>&1 &
  fi
' _ "$webdriver_port" "$log_file" "$pid_file" "$nextest_pid" >/dev/null 2>&1 </dev/null

for _ in {1..50}; do
    if webdriver_ready; then
        echo "started ChromeDriver at $webdriver_url (log: $log_file)" >&2
        echo "WEBDRIVER_URL=$webdriver_url" >>"$NEXTEST_ENV"
        exit 0
    fi
    sleep 0.1
done

echo "ChromeDriver did not start at $webdriver_url; log follows:" >&2
if [[ -f "$log_file" ]]; then
    cat "$log_file" >&2
fi
exit 1
