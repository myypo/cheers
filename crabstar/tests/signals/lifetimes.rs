use askama::Template;
use crabstar::page;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, Default, Debug)]
struct PostContent<'a> {
    rating: &'a str,
}

#[page(path = "nested-post.html")]
#[derive(Deserialize, Serialize, Default)]
struct Post<'a> {
    title: &'a str,
    content: PostContent<'a>,
}

#[test]
fn works_with_nested_lifetimes() {
    let rating = "berrygood";
    let title = "nolife";

    let content = PostContent { rating };
    let post = Post { title, content };

    let got = post.render().unwrap();
    let want = format!("<body>{title}{rating}</body>");
    got.starts_with(&want);
}
