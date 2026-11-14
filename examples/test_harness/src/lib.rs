//! Browser tests of `cheers::test` itself, among them its sharing of browsers between apps.

use axum::{Router, response::Html, routing::get};

#[cfg(test)]
mod hooks;

pub fn router() -> Router {
    Router::new().route(
        "/",
        get(|| async {
            Html("<!doctype html><html lang=\"en\"><title>Harness</title><p>Harness</p></html>")
        }),
    )
}

#[cfg(test)]
mod tests {
    use thirtyfour::prelude::*;

    use super::router;

    const REMEMBER: &str = "document.cookie = 'remembered=yes'; \
        localStorage.setItem('remembered', 'yes'); \
        sessionStorage.setItem('remembered', 'yes');";
    const RECALL: &str = "return [document.cookie, localStorage.getItem('remembered'), \
        sessionStorage.getItem('remembered')];";

    async fn recall(app: &WebDriver) -> WebDriverResult<Vec<Option<String>>> {
        app.execute(RECALL, vec![]).await?.convert()
    }

    #[tokio::test]
    async fn an_app_starts_with_one_window_and_nothing_stored() {
        for _ in 0..2 {
            cheers::test::App::new(router())
                .await
                .expect("start the app")
                .run(|app| async move {
                    assert_eq!(app.windows().await?.len(), 1);
                    app.goto(app.url("/")).await?;
                    assert_eq!(recall(&app).await?, [Some(String::new()), None, None]);
                    app.execute(REMEMBER, vec![]).await?;
                    app.set_window_rect(0, 0, 500, 400).await?;
                    Ok(())
                })
                .await
                .expect("run the app");
        }
    }

    #[tokio::test]
    async fn a_new_tab_shares_the_session_cookies_and_storage() {
        cheers::test::App::new(router())
            .await
            .expect("start the app")
            .run(|app| async move {
                app.goto(app.url("/")).await?;
                app.execute(REMEMBER, vec![]).await?;
                for tab in [app.new_tab().await?, app.new_window().await?] {
                    app.switch_to_window(tab).await?;
                    app.goto(app.url("/")).await?;
                    let [cookie, local, _] = &recall(&app).await?[..] else {
                        unreachable!("RECALL returns three values")
                    };
                    assert_eq!(cookie.as_deref(), Some("remembered=yes"));
                    assert_eq!(local.as_deref(), Some("yes"));
                }
                assert_eq!(app.windows().await?.len(), 3);
                Ok(())
            })
            .await
            .expect("run the app");
    }

    #[tokio::test]
    async fn a_window_webdriver_opened_leaves_nothing_for_the_next_app() {
        cheers::test::App::new(router())
            .await
            .expect("start the app")
            .run(|app| async move {
                let tab = (*app).new_tab().await?;
                app.switch_to_window(tab).await?;
                app.goto(app.url("/")).await?;
                app.execute(REMEMBER, vec![]).await?;
                Ok(())
            })
            .await
            .expect("run the first app");

        cheers::test::App::new(router())
            .await
            .expect("start the second app")
            .run(|app| async move {
                assert_eq!(app.windows().await?.len(), 1);
                app.goto(app.url("/")).await?;
                assert_eq!(recall(&app).await?, [Some(String::new()), None, None]);
                Ok(())
            })
            .await
            .expect("run the second app");
    }
}
