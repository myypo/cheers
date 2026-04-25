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

if [[ -z "$webdriver_host" || -z "$webdriver_port" || "$webdriver_host" == "$webdriver_port" ]]; then
    echo "unsupported WEBDRIVER_URL '$webdriver_url'; expected http://host:port" >&2
    exit 1
fi

can_connect() {
    # shellcheck disable=SC2016 # Inner shell expands positional parameters.
    timeout 1 bash -c '</dev/tcp/"$1"/"$2"' _ "$webdriver_host" "$webdriver_port" >/dev/null 2>&1
}

if can_connect; then
    echo "using existing WebDriver at $webdriver_url" >&2
    echo "WEBDRIVER_URL=$webdriver_url" >>"$NEXTEST_ENV"
    exit 0
fi

if ! command -v chromedriver >/dev/null 2>&1; then
    echo "chromedriver not found in PATH" >&2
    exit 1
fi

log_dir="${CARGO_TARGET_DIR:-target}/nextest"
mkdir -p "$log_dir"
log_file="$log_dir/chromedriver-$webdriver_port.log"
pid_file="$log_dir/chromedriver-$webdriver_port.pid"
nextest_pid="$PPID"

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

  chromedriver --port="$port" >"$log_file" 2>&1 &
  driver_pid="$!"
  echo "$driver_pid" >"$pid_file"

  (
    while kill -0 "$nextest_pid" >/dev/null 2>&1; do
      sleep 1
    done
    kill "$driver_pid" >/dev/null 2>&1 || true
    wait "$driver_pid" >/dev/null 2>&1 || true
    rm -f "$pid_file"
  ) >/dev/null 2>&1 &
' _ "$webdriver_port" "$log_file" "$pid_file" "$nextest_pid" >/dev/null 2>&1 </dev/null

for _ in {1..50}; do
    if can_connect; then
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
