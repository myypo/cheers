#[cfg(test)]
pub(crate) async fn read_axum_body(resp: impl axum::response::IntoResponse) -> String {
    use futures::StreamExt;

    let resp = resp.into_response();
    resp.into_body()
        .into_data_stream()
        .fold(String::new(), async |mut acc, ch| {
            let bytes = ch.expect("axum body chunk should be readable");
            let text =
                std::str::from_utf8(bytes.as_ref()).expect("axum body chunk should be valid UTF-8");
            acc.push_str(text);
            acc
        })
        .await
}
