//! Chromium browsers kept running between [`App`](super::App)s, so a test run starts one per
//! concurrent test instead of one per test.
//!
//! The pool directory holds one `{slot}.lock` file per browser slot. An `App` holds the lock of
//! its slot for as long as it runs, and the file names the slot's current browser, whose user data
//! dir is `{slot}-{n}`, with its process id in `{slot}-{n}.pid` and its output in
//! `{slot}-{n}.log`. A browser that a test may have left state in is replaced by a new one with a
//! new user data dir, so no two browsers ever share one.

use std::{
    fs::{self, File, TryLockError},
    io::{self, Read as _, Seek as _, Write as _},
    os::unix::process::CommandExt as _,
    path::Path,
    process::{Command, Stdio},
    sync::Arc,
    time::{Duration, SystemTime},
};

use futures::{SinkExt as _, StreamExt as _};
use serde_json::{Value, json};
use thirtyfour::{
    ChromeCapabilities, ChromiumLikeCapabilities as _, WebDriver, error::WebDriverResult,
};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{self, Message},
};

const LAUNCH_TIMEOUT: Duration = Duration::from_secs(60);
const CDP_TIMEOUT: Duration = Duration::from_secs(30);

/// The switches ChromeDriver adds to the ones it is given when it starts Chrome itself, so a
/// pooled browser behaves like one ChromeDriver started: popups are allowed, background tabs are
/// not throttled, and no keyring or first-run UI is touched.
const CHROMEDRIVER_SWITCHES: &[&str] = &[
    "--allow-pre-commit-input",
    "--disable-background-networking",
    "--disable-background-timer-throttling",
    "--disable-backgrounding-occluded-windows",
    "--disable-client-side-phishing-detection",
    "--disable-default-apps",
    "--disable-features=IgnoreDuplicateNavs,Prewarm",
    "--disable-hang-monitor",
    "--disable-popup-blocking",
    "--disable-prompt-on-repost",
    "--disable-sync",
    "--enable-automation",
    "--enable-logging",
    "--log-level=0",
    "--no-first-run",
    "--no-service-autorun",
    "--password-store=basic",
    "--test-type=webdriver",
    "--use-mock-keychain",
];

/// Exclusive use of one pooled browser, holding a fresh browser context with a single page.
///
/// The context has its own cookies, storage, cache and window, so nothing a test leaves in it
/// reaches the next test that uses the same browser.
pub(super) struct Lease {
    slot: File,
    cdp: Cdp,
    context: Arc<str>,
    target: String,
}

impl Lease {
    /// Claim the first browser slot in `dir` no other `App` is using, with a running browser that
    /// has no pages and no browser contexts but a new one.
    pub(super) async fn take(dir: &Path, chromium: &str, args: &[&str]) -> io::Result<Self> {
        fs::create_dir_all(dir)?;
        let (index, mut slot) = claim_slot(dir)?;

        let mut current = String::new();
        slot.read_to_string(&mut current)?;
        let mut cdp = match current.trim() {
            "" => None,
            name => match Cdp::connect(&dir.join(name)).await {
                Ok(cdp) => Some(cdp),
                Err(err) => {
                    eprintln!("replacing pooled Chromium {name}, which did not answer: {err}");
                    kill_browser(dir, name);
                    None
                }
            },
        };
        if let Some(mut browser) = cdp.take() {
            if browser.dispose_contexts_and_check_empty().await? {
                cdp = Some(browser);
            } else {
                eprintln!(
                    "replacing pooled Chromium {}, which has pages outside a test's browser context",
                    current.trim()
                );
                browser.close().await;
            }
        }
        let mut cdp = match cdp {
            Some(cdp) => cdp,
            None => launch(dir, index, &mut slot, chromium, args).await?,
        };

        let context: Arc<str> = cdp
            .call("Target.createBrowserContext", json!({}))
            .await?
            .get("browserContextId")
            .and_then(Value::as_str)
            .ok_or_else(|| io::Error::other("Target.createBrowserContext returned no id"))?
            .into();
        let target = create_target(&mut cdp, &context).await?;
        cdp.watch_for_pages_outside(Arc::clone(&context)).await?;

        Ok(Self {
            slot,
            cdp,
            context,
            target,
        })
    }

    /// Open a WebDriver session on the leased page.
    pub(super) async fn attach(&self, webdriver_url: String) -> WebDriverResult<WebDriver> {
        let mut caps = ChromeCapabilities::new();
        caps.set_debugger_address(&self.cdp.address)?;
        let driver = WebDriver::new(webdriver_url, caps).await?;
        driver.switch_to_window(self.target.clone().into()).await?;
        Ok(driver)
    }

    pub(super) fn context(&self) -> Arc<str> {
        Arc::clone(&self.context)
    }

    /// Close the context and its pages, and hand the browser to the next `App`.
    ///
    /// Windows opened through WebDriver's new-window command land in the browser's default
    /// context, where they would share cookies and storage with later tests, so a browser that
    /// had any is closed and the slot gets a new one.
    pub(super) async fn release(mut self) -> io::Result<()> {
        self.cdp
            .call(
                "Target.disposeBrowserContext",
                json!({ "browserContextId": &*self.context }),
            )
            .await?;
        if self.cdp.saw_page_outside || !self.cdp.pages().await?.is_empty() {
            self.slot.set_len(0)?;
            self.cdp.close().await;
        }
        Ok(())
    }
}

async fn create_target(cdp: &mut Cdp, context: &str) -> io::Result<String> {
    Ok(cdp
        .call(
            "Target.createTarget",
            json!({ "url": "about:blank", "browserContextId": context }),
        )
        .await?
        .get("targetId")
        .and_then(Value::as_str)
        .ok_or_else(|| io::Error::other("Target.createTarget returned no id"))?
        .to_owned())
}

fn claim_slot(dir: &Path) -> io::Result<(usize, File)> {
    for index in 0.. {
        let slot = File::options()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(dir.join(format!("{index}.lock")))?;
        match slot.try_lock() {
            Ok(()) => return Ok((index, slot)),
            Err(TryLockError::WouldBlock) => continue,
            Err(TryLockError::Error(err)) => return Err(err),
        }
    }
    unreachable!("an unbounded range always yields another slot")
}

/// Start a browser for slot `index` under a user data dir no earlier browser used, and record it
/// as the slot's current one.
async fn launch(
    dir: &Path,
    index: usize,
    slot: &mut File,
    chromium: &str,
    args: &[&str],
) -> io::Result<Cdp> {
    let stamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let name = format!("{index}-{stamp}");
    let profile = dir.join(&name);
    let log = File::create(dir.join(format!("{name}.log")))?;

    // In a process group of its own, so it outlives the test process that started it and can be
    // stopped with its helpers; its standard streams are not the test's either, which nextest
    // would report as leaked.
    let mut child = Command::new(chromium)
        .args(args)
        .args(CHROMEDRIVER_SWITCHES)
        .arg("--remote-debugging-port=0")
        .arg(format!("--user-data-dir={}", profile.display()))
        .arg("about:blank")
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log)
        .process_group(0)
        .spawn()?;
    fs::write(dir.join(format!("{name}.pid")), format!("{}\n", child.id()))?;

    let started = tokio::time::Instant::now();
    loop {
        match Cdp::connect(&profile).await {
            Ok(mut cdp) => {
                // The startup page would show up as an extra WebDriver window; it has to exist
                // until now for Chromium to size later windows as ChromeDriver's are.
                for page in cdp.pages().await? {
                    cdp.call(
                        "Target.closeTarget",
                        json!({ "targetId": page["targetId"] }),
                    )
                    .await?;
                }
                slot.set_len(0)?;
                slot.rewind()?;
                slot.write_all(name.as_bytes())?;
                return Ok(cdp);
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => {
                kill_browser(dir, &name);
                return Err(err);
            }
        }
        if let Some(status) = child.try_wait()? {
            return Err(io::Error::other(format!(
                "Chromium exited with {status} before opening its DevTools port; see {}",
                dir.join(format!("{name}.log")).display()
            )));
        }
        if started.elapsed() > LAUNCH_TIMEOUT {
            kill_browser(dir, &name);
            return Err(io::Error::other(format!(
                "Chromium did not open its DevTools port within {LAUNCH_TIMEOUT:?}; see {}",
                dir.join(format!("{name}.log")).display()
            )));
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

fn kill_browser(dir: &Path, name: &str) {
    if let Ok(pid) = fs::read_to_string(dir.join(format!("{name}.pid")))
        && let Ok(pid) = pid.trim().parse::<libc::pid_t>()
    {
        // SAFETY: `killpg` only sends a signal; the group is the one the browser was started in.
        unsafe { libc::killpg(pid, libc::SIGKILL) };
    }
}

/// A connection to a browser's own DevTools target, which can create and dispose browser
/// contexts; the page targets WebDriver forwards CDP commands to are not allowed to.
struct Cdp {
    address: String,
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    next_id: u64,
    watched_context: Option<Arc<str>>,
    saw_page_outside: bool,
}

impl Cdp {
    /// Connect to the browser whose user data dir is `profile`. Fails with
    /// [`io::ErrorKind::NotFound`] while the browser has not yet opened its DevTools port.
    async fn connect(profile: &Path) -> io::Result<Self> {
        let active = fs::read_to_string(profile.join("DevToolsActivePort"))?;
        let mut lines = active.lines();
        let (Some(port), Some(path)) = (lines.next(), lines.next()) else {
            return Err(io::ErrorKind::NotFound.into());
        };
        let address = format!("127.0.0.1:{port}");
        let (socket, _) =
            tokio::time::timeout(CDP_TIMEOUT, connect_async(format!("ws://{address}{path}")))
                .await
                .map_err(|_| timed_out("connecting to Chromium's DevTools"))?
                .map_err(ws_error)?;
        Ok(Self {
            address,
            socket,
            next_id: 0,
            watched_context: None,
            saw_page_outside: false,
        })
    }

    /// Dispose every browser context and report whether the browser is left without pages.
    ///
    /// Contexts and pages are left behind by a test process killed before releasing its lease.
    async fn dispose_contexts_and_check_empty(&mut self) -> io::Result<bool> {
        let contexts = self.call("Target.getBrowserContexts", json!({})).await?;
        for context in contexts["browserContextIds"]
            .as_array()
            .into_iter()
            .flatten()
        {
            self.call(
                "Target.disposeBrowserContext",
                json!({ "browserContextId": context }),
            )
            .await?;
        }
        Ok(self.pages().await?.is_empty())
    }

    /// Record from now on whether a page opens outside `context`, in [`Self::saw_page_outside`].
    async fn watch_for_pages_outside(&mut self, context: Arc<str>) -> io::Result<()> {
        self.watched_context = Some(context);
        self.call("Target.setDiscoverTargets", json!({ "discover": true }))
            .await
            .map(drop)
    }

    async fn pages(&mut self) -> io::Result<Vec<Value>> {
        let targets = self.call("Target.getTargets", json!({})).await?;
        Ok(targets["targetInfos"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|target| target["type"] == "page")
            .cloned()
            .collect())
    }

    async fn close(mut self) {
        let _ = self.call("Browser.close", json!({})).await;
    }

    async fn call(&mut self, method: &str, params: Value) -> io::Result<Value> {
        tokio::time::timeout(CDP_TIMEOUT, self.call_untimed(method, params))
            .await
            .map_err(|_| timed_out(method))?
    }

    async fn call_untimed(&mut self, method: &str, params: Value) -> io::Result<Value> {
        self.next_id += 1;
        let id = self.next_id;
        let request = json!({ "id": id, "method": method, "params": params });
        self.socket
            .send(Message::text(request.to_string()))
            .await
            .map_err(ws_error)?;

        while let Some(message) = self.socket.next().await {
            let Message::Text(text) = message.map_err(ws_error)? else {
                continue;
            };
            let mut reply: Value = serde_json::from_str(&text)?;
            if reply["id"] != id {
                self.observe(&reply);
                continue;
            }
            if let Some(err) = reply.get("error") {
                return Err(io::Error::other(format!("{method} failed: {err}")));
            }
            return Ok(reply["result"].take());
        }
        Err(io::Error::other(format!(
            "Chromium closed the DevTools connection during {method}"
        )))
    }
}

impl Cdp {
    /// Events arrive interleaved with replies, so each call notes the ones it reads past.
    fn observe(&mut self, event: &Value) {
        let target = &event["params"]["targetInfo"];
        if let Some(context) = &self.watched_context
            && event["method"] == "Target.targetCreated"
            && target["type"] == "page"
            && target["browserContextId"] != **context
        {
            self.saw_page_outside = true;
        }
    }
}

fn ws_error(err: tungstenite::Error) -> io::Error {
    match err {
        tungstenite::Error::Io(err) => err,
        err => io::Error::other(err),
    }
}

fn timed_out(what: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::TimedOut,
        format!("Chromium did not answer {what} within {CDP_TIMEOUT:?}"),
    )
}
