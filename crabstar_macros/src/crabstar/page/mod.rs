use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::crabstar::{CrabstarArgs, complete_ident};

pub(crate) struct PageImplArgs<'a> {
    pub args: &'a CrabstarArgs,
    pub ident: &'a Ident,
    pub generic_params: &'a TokenStream,
    pub generic_args: &'a TokenStream,
    pub where_clause: &'a Option<syn::WhereClause>,
}

pub(crate) fn page_impl<'a>(
    PageImplArgs {
        args,
        ident,
        generic_params,
        generic_args,
        where_clause,
    }: PageImplArgs<'a>,
) -> TokenStream {
    let Some(page) = &args.page else {
        return quote! {};
    };

    let status = match &page.status {
        Some(status) => quote! { ::crabstar::helpers::axum::http::StatusCode::#status },
        None => quote! { ::crabstar::helpers::axum::http::StatusCode::OK },
    };

    let suspense = args.suspense.iter().any(|f| f.template.is_some());
    let body = if suspense {
        quote! {
            use ::crabstar::suspense::Page;
            <Self as Page>::into_suspensed_response(self)
        }
    } else {
        quote! {
            use ::crabstar::askama::Template;
            let body = match self.render() {
                Ok(body) => body,
                Err(_) => return ::crabstar::helpers::axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            };
            (#status, ::crabstar::helpers::axum::response::Html(body)).into_response()
        }
    };

    let ident = if suspense {
        &complete_ident(ident)
    } else {
        ident
    };

    let page_impl = if suspense {
        quote! {
            impl #generic_params ::crabstar::suspense::Page for #ident #generic_args #where_clause {
                const STATUS: ::crabstar::helpers::axum::http::StatusCode = #status;
            }
        }
    } else {
        quote! {}
    };

    let ref_impl = if suspense {
        quote! {}
    } else {
        quote! {
            impl #generic_params ::crabstar::helpers::axum::response::IntoResponse for &#ident #generic_args #where_clause {
                fn into_response(self) -> ::crabstar::helpers::axum::response::Response {
                    #body
                }
            }
        }
    };

    quote! {
        #page_impl

        impl #generic_params ::crabstar::helpers::axum::response::IntoResponse for #ident #generic_args #where_clause {
            fn into_response(self) -> ::crabstar::helpers::axum::response::Response {
                #body
            }
        }

        #ref_impl
    }
}
