//! Helpers for end-to-end tests.

use std::{
    future::{self, Future},
    io,
    net::SocketAddr,
    ops::Deref,
    panic,
    path::PathBuf,
    pin::{Pin, pin},
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use axum::Router;
use thirtyfour::{
    IntoArcStr, WindowHandle,
    error::{WebDriverError, WebDriverResult},
};
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};

use crate::{router::ActionDef, track};

mod hooks;
#[cfg(unix)]
mod pool;

pub use hooks::HeldAction;
pub(crate) use hooks::TRACK_HEADER;

#[cfg(not(unix))]
mod pool {
    use std::{io, path::Path, sync::Arc};

    use thirtyfour::{WebDriver, error::WebDriverResult};

    pub(super) enum Lease {}

    impl Lease {
        pub(super) async fn take(_: &Path, _: &str, _: &[&str]) -> io::Result<Self> {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("{} is only supported on Unix", super::BROWSER_POOL_ENV),
            ))
        }

        pub(super) async fn attach(&self, _: String) -> WebDriverResult<WebDriver> {
            match *self {}
        }

        pub(super) fn context(&self) -> Arc<str> {
            match *self {}
        }

        pub(super) async fn release(self) -> io::Result<()> {
            match self {}
        }
    }
}

/// A running test server and browser session.
///
/// Creating an `App` turns off debug-build live reload for the whole process, so pages under test
/// are never reloaded by source edits and no file watcher is started.
///
/// Drive the browser through [`App::run`], whose [`Session`] builds URLs for the server and
/// waits for Datastar after navigating.
///
/// ```ignore
/// cheers::test::App::new(router).await?.run(|app| async move {
///     app.goto(app.url("/")).await?;
///     Ok(())
/// }).await?;
/// ```
pub struct App {
    addr: SocketAddr,
    shutdown_tx: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<io::Result<()>>>,
    driver: Option<thirtyfour::WebDriver>,
    lease: Option<pool::Lease>,
    hooks: Arc<hooks::Hooks>,
}

/// Environment variable used by [`chrome`] to locate WebDriver.
pub const WEBDRIVER_URL_ENV: &str = "WEBDRIVER_URL";

/// Environment variable used by [`chrome`] to locate the Chrome/Chromium binary.
pub const CHROME_BIN_ENV: &str = "CHROME_BIN";

/// Environment variable naming an absolute directory in which [`App`]s keep Chromium browsers
/// running for later `App`s, in this process or another, instead of having ChromeDriver start and
/// stop one each. Unix only.
///
/// Each `App` has a browser to itself and opens a fresh browser context in it, so cookies,
/// storage, cache and window size never carry over. The browser is started on this host, so the
/// WebDriver at [`WEBDRIVER_URL_ENV`] has to run here too, and it keeps the environment of the
/// test process that started it. Whoever sets the variable stops the browsers once the tests are
/// done: each writes its process id, which is also its process group, to a `*.pid` file in the
/// directory.
pub const BROWSER_POOL_ENV: &str = "CHEERS_TEST_BROWSER_POOL";

const CHROMIUM_ARGS: &[&str] = &[
    "--headless",
    "--no-sandbox",
    "--disable-gpu",
    "--disable-dev-shm-usage",
];

/// The first of `names` on `PATH`, unless [`CHROME_BIN_ENV`] names the binary.
fn chromium_binary(names: &[&str]) -> Option<String> {
    if let Ok(path) = std::env::var(CHROME_BIN_ENV) {
        return Some(path);
    }

    let paths: Vec<_> = std::env::split_paths(&std::env::var_os("PATH")?).collect();
    names
        .iter()
        .flat_map(|name| paths.iter().map(move |dir| dir.join(name)))
        .find(|p| p.is_file())
        .map(|p| p.to_string_lossy().into_owned())
}

fn pool_dir() -> io::Result<Option<PathBuf>> {
    let Some(dir) = std::env::var_os(BROWSER_POOL_ENV) else {
        return Ok(None);
    };
    let dir = PathBuf::from(dir);
    if !dir.is_absolute() {
        return Err(io::Error::other(format!(
            "{BROWSER_POOL_ENV} must be an absolute path, not {dir:?}"
        )));
    }
    Ok(Some(dir))
}

/// Create a headless Chrome WebDriver session using [`WEBDRIVER_URL_ENV`] or a default URL, in a
/// browser from [`BROWSER_POOL_ENV`] when it is set.
async fn chrome() -> WebDriverResult<(thirtyfour::WebDriver, Option<pool::Lease>)> {
    use thirtyfour::prelude::*;

    const DEFAULT_WEBDRIVER_URL: &str = "http://127.0.0.1:9515";
    let url = std::env::var(WEBDRIVER_URL_ENV).unwrap_or_else(|_| DEFAULT_WEBDRIVER_URL.to_owned());

    let Some(dir) = pool_dir()? else {
        let mut caps = DesiredCapabilities::chrome();
        if let Some(path) = chromium_binary(&["chromium"]) {
            caps.set_binary(&path)?;
        }
        for arg in CHROMIUM_ARGS {
            caps.add_arg(arg)?;
        }
        let driver = WebDriver::new(url, caps).await?;
        return match record_datastar_ready(&driver).await {
            Ok(()) => Ok((driver, None)),
            Err(err) => {
                let _ = driver.quit().await;
                Err(err)
            }
        };
    };

    // ChromeDriver is not there to find a browser for a pooled one.
    let chromium = chromium_binary(&[
        "chromium",
        "chromium-browser",
        "google-chrome-stable",
        "google-chrome",
        "chrome",
    ])
    .ok_or_else(|| {
        io::Error::other(format!(
            "{BROWSER_POOL_ENV} is set but no browser was found to start; set {CHROME_BIN_ENV}"
        ))
    })?;
    let lease = pool::Lease::take(&dir, &chromium, CHROMIUM_ARGS).await?;
    let driver = match lease.attach(url).await {
        Ok(driver) => driver,
        Err(err) => {
            let _ = lease.release().await;
            return Err(err);
        }
    };
    match record_datastar_ready(&driver).await {
        Ok(()) => Ok((driver, Some(lease))),
        Err(err) => {
            let _ = driver.quit().await;
            let _ = lease.release().await;
            Err(err)
        }
    }
}

async fn record_datastar_ready(driver: &thirtyfour::WebDriver) -> WebDriverResult<()> {
    // Datastar announces readiness only through a one-shot event, so the listener has to exist
    // before any page script runs for `Session::goto` to tell a bound page from an unbound one.
    driver
        .cdp()
        .page()
        .add_script_to_evaluate_on_new_document(RECORD_DATASTAR_READY_SCRIPT)
        .await?;
    Ok(())
}

const RECORD_DATASTAR_READY_SCRIPT: &str = r#"
document.addEventListener("datastar-ready", () => {
  window.__cheersTestDatastarReady = true;
}, { once: true });
"#;

impl App {
    /// Start serving `app` on `127.0.0.1` and a random available port,
    /// then open a headless Chrome WebDriver session.
    pub async fn new(app: Router) -> WebDriverResult<Self> {
        crate::router::disable_live_reload();

        let hooks = Arc::new(hooks::Hooks::default());
        let app = app.layer(axum::middleware::from_fn_with_state(
            hooks.clone(),
            hooks::intercept,
        ));
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let addr = listener.local_addr()?;
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
        });

        let (driver, lease) = match chrome().await {
            Ok(browser) => browser,
            Err(err) => {
                let _ = shutdown_tx.send(());
                return Err(err);
            }
        };

        Ok(Self {
            addr,
            shutdown_tx: Some(shutdown_tx),
            task: Some(task),
            driver: Some(driver),
            lease,
            hooks,
        })
    }

    /// Run a browser test and always shut down the browser and server afterwards.
    ///
    /// This avoids the common pattern of manually storing the test result, calling
    /// [`shutdown`](Self::shutdown), then re-raising the result. If the test body
    /// panics, `run` performs best-effort shutdown and then resumes the panic.
    ///
    /// ```ignore
    /// cheers::test::App::new(router).await?.run(|app| async move {
    ///     app.goto(app.url("/")).await?;
    ///     Ok(())
    /// }).await?;
    /// ```
    pub async fn run<F, Fut, T>(self, f: F) -> WebDriverResult<T>
    where
        F: FnOnce(Session) -> Fut,
        Fut: Future<Output = WebDriverResult<T>>,
    {
        let session = Session {
            addr: self.addr,
            driver: self.driver.as_ref().expect("browser to be present").clone(),
            context: self.lease.as_ref().map(pool::Lease::context),
            hooks: self.hooks.clone(),
        };

        let mut output = None;
        let caught = {
            let mut body = pin!(async { output = Some(f(session).await) });
            future::poll_fn(|cx| poll_catching_panics(body.as_mut(), cx)).await
        };

        match caught {
            Ok(()) => {
                let result = output.expect("a body that completed stored its result");
                match (result, self.shutdown().await) {
                    (Ok(value), Ok(())) => Ok(value),
                    (Err(err), Ok(())) => Err(err),
                    (Ok(_), Err(err)) => Err(err),
                    (Err(err), Err(shutdown_err)) => {
                        eprintln!("failed to shut down test app after test error: {shutdown_err}");
                        Err(err)
                    }
                }
            }
            Err(payload) => {
                if let Err(err) = self.shutdown().await {
                    eprintln!("failed to shut down test app after panic: {err}");
                }
                panic::resume_unwind(payload);
            }
        }
    }

    fn remember_error(first_error: &mut Option<WebDriverError>, err: WebDriverError) {
        if first_error.is_none() {
            *first_error = Some(err);
        }
    }

    async fn shutdown(mut self) -> WebDriverResult<()> {
        let mut first_error = None;

        if let Some(driver) = self.driver.take()
            && let Err(err) = driver.quit().await
        {
            Self::remember_error(&mut first_error, err);
        }
        if let Some(lease) = self.lease.take()
            && let Err(err) = lease.release().await
        {
            Self::remember_error(&mut first_error, WebDriverError::IoError(err));
        }
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        if let Some(task) = self.task.take() {
            match task.await {
                Ok(Ok(())) => {}
                Ok(Err(err)) => {
                    Self::remember_error(&mut first_error, WebDriverError::IoError(err))
                }
                Err(err) => Self::remember_error(
                    &mut first_error,
                    WebDriverError::IoError(io::Error::other(format!(
                        "test server task failed: {err}"
                    ))),
                ),
            }
        }

        if let Some(err) = first_error {
            Err(err)
        } else {
            Ok(())
        }
    }
}

/// Browser session for a running test app.
///
/// This is passed to [`App::run`] and can be used like a [`thirtyfour::WebDriver`]
/// via [`Deref`].
pub struct Session {
    addr: SocketAddr,
    driver: thirtyfour::WebDriver,
    context: Option<Arc<str>>,
    hooks: Arc<hooks::Hooks>,
}

impl Session {
    /// Build an absolute URL for `path` on the managed server.
    pub fn url(&self, path: impl AsRef<str>) -> String {
        let path = path.as_ref();
        let separator = if path.starts_with('/') { "" } else { "/" };
        format!("http://{}{}{}", self.addr, separator, path)
    }

    /// Navigate to `url` and, when the page loads the Cheers runtime, wait until Datastar has
    /// bound its attributes.
    ///
    /// WebDriver returns once the document has loaded, but Datastar applies attributes in a
    /// later task, so input sent right after a plain navigation can reach the page before its
    /// handlers exist. This shadows [`thirtyfour::WebDriver::goto`].
    pub async fn goto(&self, url: impl IntoArcStr) -> WebDriverResult<()> {
        self.driver.goto(url).await?;
        self.wait_for_datastar().await
    }

    /// Open a tab without switching to it. This shadows [`thirtyfour::WebDriver::new_tab`], which
    /// in a browser from [`BROWSER_POOL_ENV`] opens it outside the session's cookies and storage.
    pub async fn new_tab(&self) -> WebDriverResult<WindowHandle> {
        self.open_window(false).await
    }

    /// Open a window without switching to it. This shadows [`thirtyfour::WebDriver::new_window`],
    /// which in a browser from [`BROWSER_POOL_ENV`] opens it outside the session's cookies and
    /// storage.
    pub async fn new_window(&self) -> WebDriverResult<WindowHandle> {
        self.open_window(true).await
    }

    async fn open_window(&self, new_window: bool) -> WebDriverResult<WindowHandle> {
        let Some(context) = &self.context else {
            return if new_window {
                self.driver.new_window().await
            } else {
                self.driver.new_tab().await
            };
        };
        let created = self
            .driver
            .cdp()
            .send_raw(
                "Target.createTarget",
                serde_json::json!({
                    "url": "about:blank",
                    "browserContextId": &**context,
                    "newWindow": new_window,
                }),
            )
            .await?;
        let target = created["targetId"].as_str().ok_or_else(|| {
            WebDriverError::ParseError(format!("Target.createTarget returned no id: {created}"))
        })?;
        Ok(target.into())
    }

    /// Wait until Datastar has bound the current page, if the page loads the Cheers runtime.
    ///
    /// Call this after navigating by other means, such as following a link or going back.
    /// Only the session's initial window records readiness, so this times out in windows
    /// opened later.
    pub async fn wait_for_datastar(&self) -> WebDriverResult<()> {
        let ready = self
            .driver
            .execute(
                WAIT_FOR_DATASTAR_SCRIPT,
                vec![serde_json::json!(DATASTAR_READY_TIMEOUT.as_millis() as u64)],
            )
            .await?
            .convert::<bool>()?;

        if ready {
            Ok(())
        } else {
            Err(WebDriverError::Timeout(format!(
                "Datastar did not become ready within {DATASTAR_READY_TIMEOUT:?}"
            )))
        }
    }
}

impl Session {
    /// Hold the response to the next request for action `A` until the returned [`HeldAction`] is
    /// released or dropped.
    ///
    /// The handler runs as usual and only its response is kept from the browser, so the page can
    /// be checked while the request is in flight, such as an indicator showing or a button
    /// disabled, with the server-side effects already applied. Each call holds one request;
    /// several holds for the same action take its requests in the order they were made.
    ///
    /// ```ignore
    /// let held = app.hold_action::<SaveAction>();
    /// app.find(By::Css("button")).await?.click().await?;
    /// held.reached().await?;
    /// // The page still shows the request as pending.
    /// held.release();
    /// ```
    pub fn hold_action<A: ActionDef>(&self) -> HeldAction {
        self.hooks.hold(A::METHOD, A::PATH, hooks::Stage::Response)
    }

    /// Hold the next request for action `A` before its handler runs, until the returned
    /// [`HeldAction`] is released or dropped.
    ///
    /// Unlike [`hold_action`](Self::hold_action), the server sees nothing of the request until it is
    /// released, so requests sent meanwhile are handled first.
    pub fn hold_action_request<A: ActionDef>(&self) -> HeldAction {
        self.hooks.hold(A::METHOD, A::PATH, hooks::Stage::Request)
    }

    /// The items the page posted to the track endpoint, in the order the server received them.
    ///
    /// The endpoint is the one configured with [`crate::router::Config::track`]. A batch counts once the app answers it successfully; when the app
    /// serves nothing at the endpoint, the batch is answered with `202 Accepted` and counted, so a
    /// test app needs no ingest route.
    ///
    /// The runtime sends items in batches up to half a second apart, so wait for the ones a test
    /// expects with [`wait_for_tracked`](Self::wait_for_tracked).
    pub fn tracked(&self) -> Vec<track::Item<serde_json::Value>> {
        self.hooks.tracked()
    }

    /// Wait until `f` finds what it looks for in the [`tracked`](Self::tracked) items and return
    /// it.
    ///
    /// `f` runs on every item received so far, each time a batch arrives. Fails with
    /// [`WebDriverError::Timeout`] when it finds nothing within 20 seconds.
    ///
    /// ```ignore
    /// let opened = app
    ///     .wait_for_tracked(|items| {
    ///         items.iter().find_map(|item| match item {
    ///             Item::Analytics(event) if event.props["vault_id"] == 69 => Some(event.clone()),
    ///             _ => None,
    ///         })
    ///     })
    ///     .await?;
    /// ```
    pub async fn wait_for_tracked<T>(
        &self,
        f: impl FnMut(&[track::Item<serde_json::Value>]) -> Option<T>,
    ) -> WebDriverResult<T> {
        self.hooks.wait_for_tracked(f).await
    }
}

/// Poll `body` once, catching a panic it raises.
///
/// This must stay non-generic and out of line. A `catch_unwind` instantiated in the caller's crate
/// is compiled by the caller's codegen backend, and Cranelift ships without unwinding support, so
/// in a test crate built with it a generic catch would never fire and a failing test would leave
/// its browser running. Compiled here, the catch frame belongs to this crate.
#[inline(never)]
fn poll_catching_panics(
    body: Pin<&mut (dyn Future<Output = ()> + '_)>,
    cx: &mut Context<'_>,
) -> Poll<std::thread::Result<()>> {
    match panic::catch_unwind(panic::AssertUnwindSafe(|| body.poll(cx))) {
        Ok(Poll::Pending) => Poll::Pending,
        Ok(Poll::Ready(())) => Poll::Ready(Ok(())),
        Err(payload) => Poll::Ready(Err(payload)),
    }
}

const DATASTAR_READY_TIMEOUT: Duration = Duration::from_secs(20);
const WAIT_TIMEOUT: Duration = Duration::from_secs(20);

const WAIT_FOR_DATASTAR_SCRIPT: &str = r#"
const timeoutMs = arguments[0];
if (window.__cheersTestDatastarReady === true
    || !document.querySelector('script[data-cheers-runtime="datastar"]')) {
  return true;
}
return new Promise((resolve) => {
  document.addEventListener("datastar-ready", () => resolve(true), { once: true });
  setTimeout(() => resolve(window.__cheersTestDatastarReady === true), timeoutMs);
});
"#;

impl Deref for App {
    type Target = thirtyfour::WebDriver;

    fn deref(&self) -> &Self::Target {
        self.driver.as_ref().expect("browser to be present")
    }
}

impl Deref for Session {
    type Target = thirtyfour::WebDriver;

    fn deref(&self) -> &Self::Target {
        &self.driver
    }
}
