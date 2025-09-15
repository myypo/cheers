use axum::response::IntoResponse;
use crabstar::{page, suspense};
use futures::StreamExt;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Barrier, Mutex};

use crate::next_axum_chunk;

#[suspense(path = "post-content.html")]
pub struct PostContent {
    content: String,
}

#[suspense(path = "post.html")]
pub struct Post {
    title: String,
    #[delayed]
    content: PostContent,
}

#[suspense(path = "status.html")]
pub struct Status {
    outages_today: i32,
}

#[page(path = "home.html", suspense)]
struct HomePage {
    user: String,
    #[delayed]
    post: Post,
    #[delayed]
    status: Status,
}

#[tokio::test]
async fn can_render_concurrently_in_order() {
    let user = "myypo".to_owned();
    let title = "Hello".to_owned();
    let content = "World".to_owned();
    let outages_today = 1;

    let barrier = Arc::new(Barrier::new(2));

    let mutex_a = Arc::new(Mutex::new(()));
    let mutex_b = Arc::new(Mutex::new(()));
    let mutex_c = Arc::new(Mutex::new(()));

    let h = {
        let title = title.clone();
        let content = content.clone();
        let barrier_post = barrier.clone();
        let barrier_status = barrier.clone();
        let mutex_a_post = mutex_a.clone();
        let mutex_b_post = mutex_b.clone();
        let mutex_a_status = mutex_a.clone();
        let mutex_c_status = mutex_c.clone();

        HomePage { user: user.clone() }.into_suspense(HomePageDelayed {
            post: async move {
                let _guard_a = mutex_a_post.lock().await;
                barrier_post.wait().await;
                let _guard_b = mutex_b_post.lock().await;

                Ok(Post { title }.into_suspense(PostDelayed {
                    content: async move { Ok(PostContent { content }) },
                }))
            },
            status: async move {
                let _guard_c = mutex_c_status.lock().await;
                barrier_status.wait().await;
                let _guard_a = mutex_a_status.lock().await;

                Ok(Status { outages_today })
            },
        })
    };

    let h = h.into_response();
    let mut h = h.into_body().into_data_stream();
    tokio::time::timeout(Duration::from_secs(1), async {
        // We append streaming SSR script to the end of page
        let home = h.next().await.unwrap().unwrap();

        let home_unwrapped = r#"<body>
    Home of myypo
    Latest post:
    <div data-suspense="post.html">Loading post...</div>
    Status:
    <div data-suspense="status.html">Loading status...</div>
"#;
        assert_ne!(home, home_unwrapped);
        assert!(
            home.starts_with(
            home_unwrapped.as_bytes(),
        ), "home = {:?}", home);

        // But the rest of chunks have to be wrapped in templates
        let post_wrapped = format!(
            r#"<template id="crabstar-template-post.html" data-on-load="streamSsr(el.id, 'post.html')">Hello
Content:
<div data-suspense="post-content.html">Loading content...</div>
</template>"#,
        );
        let status_wrapped = format!(
            r#"<template id="crabstar-template-status.html" data-on-load="streamSsr(el.id, 'status.html')">{}
</template>"#,
            outages_today
        );
        let post_content_wrapped = format!(
            r#"<template id="crabstar-template-post-content.html" data-on-load="streamSsr(el.id, 'post-content.html')">{}
</template>"#,
            content
        );

        let got_post = h.next().await.unwrap().unwrap();
        assert_eq!(got_post, post_wrapped);
        let got_post_content = h.next().await.unwrap().unwrap();
        assert_eq!(got_post_content, post_content_wrapped);
        let got_status = h.next().await.unwrap().unwrap();
        assert_eq!(got_status, status_wrapped);
    })
    .await
    .expect("deadlock");
}

#[tokio::test]
async fn streaming_ssr_script_works_with_extends() {
    #[page(path = "child.html", suspense)]
    struct ChildPage {
        user: String,
    }

    let page = ChildPage {
        user: "test".to_owned(),
    };
    let response = page.into_response();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let content = String::from_utf8(body.to_vec()).unwrap();

    assert!(
        content.contains("streamSsr"),
        "Streaming SSR script should be present in extended template"
    );
}

#[tokio::test]
async fn can_stream_with_axum() {
    let user = "user".to_owned();
    let title = "title".to_owned();
    let content = "content".to_owned();
    let outages_today = 4;

    let resp = HomePage { user: user.clone() }.into_suspense(HomePageDelayed {
        post: async move {
            Ok(Post { title }.into_suspense(PostDelayed {
                content: async move { Ok(PostContent { content }) },
            }))
        },
        status: async move { Ok(Status { outages_today }) },
    });

    let resp = resp.into_response();
    let mut body = resp.into_body().into_data_stream();

    let got = next_axum_chunk(&mut body).await;
    assert!(got.contains("user"));
    let got = next_axum_chunk(&mut body).await;
    assert!(got.contains("title"));
    let got = next_axum_chunk(&mut body).await;
    assert!(got.contains("content"));
    let got = next_axum_chunk(&mut body).await;
    assert!(got.contains("4"));
}

#[tokio::test]
async fn error_handling_works() {
    #[suspense(path = "post-content.html")]
    pub struct Error {
        content: String,
    }

    let user = "user".to_owned();
    let post = "post".to_owned();
    let status = "status".to_owned();

    let resp = HomePage { user: user.clone() }.into_suspense(HomePageDelayed {
        post: {
            let post = post.clone();
            async move { Err(Error { content: post }.into()) }
        },
        status: {
            let status = status.clone();
            async move { Err(Error { content: status }.into()) }
        },
    });

    let resp = resp.into_response();
    let mut body = resp.into_body().into_data_stream();

    let initial = next_axum_chunk(&mut body).await;
    assert!(initial.contains(&user));
    assert!(initial.contains("Loading post..."));
    assert!(initial.contains("Loading status..."));

    let error_chunk1 = next_axum_chunk(&mut body).await;
    assert_eq!(
        error_chunk1,
        format!(
            "<template id=\"crabstar-template-post.html\" data-on-load=\"streamSsr(el.id, 'post.html')\">{}\n</template>",
            post
        )
    );

    let error_chunk2 = next_axum_chunk(&mut body).await;
    assert_eq!(
        error_chunk2,
        format!(
            "<template id=\"crabstar-template-status.html\" data-on-load=\"streamSsr(el.id, 'status.html')\">{}\n</template>",
            status
        )
    );

    assert!(body.next().await.is_none());
}
