use axum::response::IntoResponse;
use crabstar::{page, suspense};
use futures::StreamExt;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Barrier, Mutex};

#[tokio::test]
async fn can_render_concurrently_in_order() {
    #[suspense(path = "post_content.html")]
    pub struct PostContent {
        content: String,
    }

    #[suspense(path = "post.html")]
    pub struct Post {
        title: String,
        #[suspense]
        content: PostContent,
    }

    #[suspense(path = "status.html")]
    pub struct Status {
        outages_today: i32,
    }

    #[page(path = "home.html", suspense)]
    struct HomePage {
        user: String,
        #[suspense]
        post: Post,
        #[suspense]
        status: Status,
    }

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
        assert_eq!(h.next().await.unwrap().unwrap(), user);
        assert!({
            let got = h.next().await.unwrap().unwrap();
            got == title || got == outages_today.to_string()
        });
        assert!({
            let got = h.next().await.unwrap().unwrap();
            got == outages_today.to_string() || got == content
        });
        assert!({
            let got = h.next().await.unwrap().unwrap();
            got == outages_today.to_string() || got == content
        });
        assert!(h.next().await.is_none());
    })
    .await
    .expect("deadlock");
}
