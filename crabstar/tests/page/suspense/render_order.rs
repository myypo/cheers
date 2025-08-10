use crabstar::page::suspense::Suspense;
use crabstar_macros::page;

use futures::{StreamExt, pin_mut};
use typed_jinja::{Template, template};

#[page]
#[template(path = "post_content.html")]
pub struct PostContent {
    content: String,
}

#[page]
#[template(path = "post.html")]
pub struct Post {
    title: String,
    #[suspense]
    content: PostContent,
}

#[page]
#[template(path = "status.html")]
pub struct Status {
    outages_today: i32,
}

#[page]
#[template(path = "home.html")]
struct HomePage {
    user: String,
    #[suspense]
    post: Post,
    #[suspense]
    status: Status,
}

#[tokio::test]
async fn can_render_in_order() {
    let user = "myypo".to_owned();
    let title = "Hello".to_owned();
    let content = "World".to_owned();
    let outages_today = 1;

    let (home_tx, home_rx) = tokio::sync::oneshot::channel();
    let (status_tx, status_rx) = tokio::sync::oneshot::channel();
    let (post_tx, post_rx) = tokio::sync::oneshot::channel();

    let h = HomePage { user: user.clone() }.into_suspense(HomePageDelayed {
        post: async {
            let _ = status_rx.await.expect("status future should signal");
            home_tx.send(()).unwrap();

            Post {
                title: title.clone(),
            }
            .into_suspense(PostDelayed {
                content: async {
                    post_rx.await.expect("post future should signal");

                    PostContent {
                        content: content.clone(),
                    }
                },
            })
        },
        status: async {
            status_tx.send(()).unwrap();
            let _ = home_rx.await.expect("home future should acknowledge");
            post_tx.send(()).expect("post future should acknowledge");

            Status { outages_today }
        },
    });

    let h = h.suspense();
    pin_mut!(h);

    assert_eq!(h.next().await.unwrap().unwrap(), user);
    assert_eq!(h.next().await.unwrap().unwrap(), title);
    assert_eq!(h.next().await.unwrap().unwrap(), outages_today.to_string());
    assert_eq!(h.next().await.unwrap().unwrap(), content);
    assert!(h.next().await.is_none());
}
