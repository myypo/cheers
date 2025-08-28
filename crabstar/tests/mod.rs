mod js_script;
mod page;
mod patch_elements;
mod signals;
mod suspense;

async fn read_axum_body(resp: impl axum::response::IntoResponse) -> String {
    use futures::StreamExt;

    let resp = resp.into_response();
    resp.into_body()
        .into_data_stream()
        .fold(String::new(), async |mut acc, ch| {
            acc.push_str(&String::from_utf8(ch.unwrap().to_vec()).unwrap());
            acc
        })
        .await
}

async fn next_axum_chunk(body: &mut axum::body::BodyDataStream) -> String {
    use futures::StreamExt;

    let ch = body.next().await.unwrap().unwrap();
    String::from_utf8(ch.to_vec()).unwrap()
}
