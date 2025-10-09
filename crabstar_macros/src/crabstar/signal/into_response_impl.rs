use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

pub fn into_response_impl(
    signal_ident: &Ident,
    generic_params: &TokenStream,
    generic_args: &TokenStream,
    where_clause: &Option<syn::WhereClause>,
) -> TokenStream {
    quote! {
        impl #generic_params ::axum::response::IntoResponse for #signal_ident #generic_args #where_clause {
            fn into_response(self) -> ::axum::response::Response {
                let Ok(body) = ::serde_json::to_string(&self) else {
                    return ::axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
                };

                match ::axum::response::Response::builder()
                    .status(::axum::http::StatusCode::OK)
                    .header("Content-Type", "application/json")
                    .body(body.into())
                {
                    Ok(r) => r,
                    Err(err) => ::axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response(),

                }
            }
        }
    }
}
