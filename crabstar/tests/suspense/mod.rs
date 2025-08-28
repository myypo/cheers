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
    #[delayed(id = "content")]
    content: PostContent,
}

#[suspense(path = "status.html")]
pub struct Status {
    outages_today: i32,
}

#[page(path = "home.html", suspense)]
struct HomePage {
    user: String,
    #[delayed(id = "post")]
    post: Post,
    #[delayed(id = "status")]
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

                Post { title }.into_suspense(PostDelayed {
                    content: async move { PostContent { content } },
                })
            },
            status: async move {
                let _guard_c = mutex_c_status.lock().await;
                barrier_status.wait().await;
                let _guard_a = mutex_a_status.lock().await;

                Status { outages_today }
            },
        })
    };

    let h = h.into_response();
    let mut h = h.into_body().into_data_stream();
    tokio::time::timeout(Duration::from_secs(1), async {
        // We append hydration script to the end of body
        assert!(
            String::from_utf8(h.next().await.unwrap().unwrap().to_vec())
                .unwrap()
                .starts_with(&user),
        );

        // But the rest of chunks have to be wrapped in templates
        let title_wrapped = format!(
            r#"<template id=post data-on-load="hydrate(el.id)">{}</template>"#,
            title
        );
        let outages_wrapped = format!(
            r#"<template id=status data-on-load="hydrate(el.id)">{}</template>"#,
            outages_today
        );
        let content_wrapped = format!(
            r#"<template id=content data-on-load="hydrate(el.id)">{}</template>"#,
            content
        );

        let got1 = h.next().await.unwrap().unwrap();
        let expected1 = if got1 == title_wrapped {
            title_wrapped
        } else {
            outages_wrapped.clone()
        };
        assert_eq!(got1, expected1);

        let got2 = h.next().await.unwrap().unwrap();
        let expected2 = if got2 == outages_wrapped {
            outages_wrapped.clone()
        } else {
            content_wrapped.clone()
        };
        assert_eq!(got2, expected2);

        let got3 = h.next().await.unwrap().unwrap();
        let expected3 = if got3 == outages_wrapped {
            outages_wrapped
        } else {
            content_wrapped
        };
        assert_eq!(got3, expected3);

        assert!(h.next().await.is_none());
    })
    .await
    .expect("deadlock");
}

#[tokio::test]
async fn can_stream_with_axum() {
    let user = "user".to_owned();
    let title = "title".to_owned();
    let content = "content".to_owned();
    let outages_today = 4;

    let resp = HomePage { user: user.clone() }.into_suspense(HomePageDelayed {
        post: async move {
            Post { title }.into_suspense(PostDelayed {
                content: async move { PostContent { content } },
            })
        },
        status: async move { Status { outages_today } },
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
