//! Helpers for end-to-end tests.

use std::{future::Future, io, net::SocketAddr, ops::Deref, panic, time::Duration};

use axum::Router;
use futures::FutureExt;
use thirtyfour::{
    IntoArcStr,
    error::{WebDriverError, WebDriverResult},
};
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};

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
}

/// Environment variable used by [`chrome`] to locate WebDriver.
pub const WEBDRIVER_URL_ENV: &str = "WEBDRIVER_URL";

/// Environment variable used by [`chrome`] to locate the Chrome/Chromium binary.
pub const CHROME_BIN_ENV: &str = "CHROME_BIN";

fn chromium_binary() -> Option<String> {
    if let Ok(path) = std::env::var(CHROME_BIN_ENV) {
        return Some(path);
    }

    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|p| p.join("chromium"))
            .find(|p| p.is_file())
            .map(|p| p.to_string_lossy().into_owned())
    })
}

/// Create a headless Chrome WebDriver session using [`WEBDRIVER_URL_ENV`] or a default URL.
async fn chrome() -> WebDriverResult<thirtyfour::WebDriver> {
    use thirtyfour::prelude::*;

    const DEFAULT_WEBDRIVER_URL: &str = "http://127.0.0.1:9515";
    let url = std::env::var(WEBDRIVER_URL_ENV).unwrap_or_else(|_| DEFAULT_WEBDRIVER_URL.to_owned());

    let mut caps = DesiredCapabilities::chrome();
    if let Some(path) = chromium_binary() {
        caps.set_binary(&path)?;
    }
    caps.set_headless()?;
    caps.set_no_sandbox()?;
    caps.set_disable_gpu()?;
    caps.set_disable_dev_shm_usage()?;

    let driver = WebDriver::new(url, caps).await?;
    // Datastar announces readiness only through a one-shot event, so the listener has to exist
    // before any page script runs for `Session::goto` to tell a bound page from an unbound one.
    driver
        .cdp()
        .page()
        .add_script_to_evaluate_on_new_document(RECORD_DATASTAR_READY_SCRIPT)
        .await?;
    Ok(driver)
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

        let driver = match chrome().await {
            Ok(driver) => driver,
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
        };

        let future = match panic::catch_unwind(panic::AssertUnwindSafe(|| f(session))) {
            Ok(future) => future,
            Err(payload) => {
                if let Err(err) = self.shutdown().await {
                    eprintln!("failed to shut down test app after panic: {err}");
                }
                panic::resume_unwind(payload);
            }
        };

        let result = panic::AssertUnwindSafe(future).catch_unwind().await;

        match result {
            Ok(result) => match (result, self.shutdown().await) {
                (Ok(value), Ok(())) => Ok(value),
                (Err(err), Ok(())) => Err(err),
                (Ok(_), Err(err)) => Err(err),
                (Err(err), Err(shutdown_err)) => {
                    eprintln!("failed to shut down test app after test error: {shutdown_err}");
                    Err(err)
                }
            },
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

const DATASTAR_READY_TIMEOUT: Duration = Duration::from_secs(20);

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
