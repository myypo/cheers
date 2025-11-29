use std::sync::Arc;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::{Ident, LitStr, Visibility};

use crate::crabstar::args::CrabstarSuspenseArgs;
use crate::crabstar::{CrabstarArgs, complete_ident};

pub(crate) struct SuspenseImplArgs<'a> {
    pub path: &'a Arc<str>,
    pub args: &'a CrabstarArgs,

    pub ident: &'a Ident,
    pub vis: &'a Visibility,
    pub generic_params: &'a TokenStream,
    pub generic_args: &'a TokenStream,
}

struct SuspenseField {
    ident: Ident,
    output: TokenStream,
}

fn to_snake_case(ident: &Ident) -> Ident {
    let s = ident.to_string();
    let mut result = String::new();

    let mut chars = s.chars();
    if let Some(first) = chars.next() {
        result.push(first.to_ascii_lowercase());
    }
    for c in chars {
        if c.is_uppercase() {
            result.push('_');
        }
        result.push(c.to_ascii_lowercase());
    }

    Ident::new(&result, ident.span())
}

fn suspense_fields_from_args(args: &[CrabstarSuspenseArgs]) -> Vec<SuspenseField> {
    args.iter()
        .filter_map(|f| f.template.as_ref().map(|t| (t, &f.name)))
        .map(|(t, name)| {
            let span = t.span();

            let ident = name.clone().unwrap_or_else(|| {
                t.segments
                    .last()
                    .map(|seg| to_snake_case(&seg.ident))
                    .unwrap()
            });

            let full_path = t
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<String>>()
                .join("::");
            let output = complete_ident(&Ident::new(&full_path, span));

            SuspenseField {
                ident,
                output: quote! { #output },
            }
        })
        .collect()
}

fn future_wrapper(field: &SuspenseField) -> TokenStream {
    let field_ident = &field.ident;
    let output_type = &field.output;

    quote! {
        ::std::boxed::Box::pin(async move {
            use ::crabstar::suspense::Complete;
            let path = <#output_type as Complete>::PATH;
            let complete = match self.#field_ident.await {
                Ok(r) => r,
                Err(e) => {
                    return Ok(::crabstar::suspense::SuspenseItem {
                        path,
                        immediate: e.user_error(),
                        nested: Vec::new(),
                    });
                }
            };
            let mut immediate = String::new();
            complete.immediate_into(&mut immediate)?;
            let nested = complete.into_futures();
            Ok(::crabstar::suspense::SuspenseItem {
                path,
                immediate,
                nested,
            })
        })
    }
}

pub(crate) fn suspense_impl<'a>(
    SuspenseImplArgs {
        path,
        args,
        ident,
        vis,
        generic_params,
        generic_args,
    }: SuspenseImplArgs<'a>,
) -> TokenStream {
    let suspense_fields = &args.suspense;
    if suspense_fields.is_empty() {
        return quote! {};
    }
    let suspense_fields = suspense_fields_from_args(&args.suspense);

    let complete_ident = complete_ident(ident);
    let suspense_ident = Ident::new(&format!("{ident}Suspense"), ident.span());
    let path_lit = LitStr::new(path, Span::call_site());

    if suspense_fields.is_empty() {
        return quote! {
            #[allow(type_alias_bounds)]
            #vis type #complete_ident #generic_params = #ident #generic_args;

            impl #generic_params ::crabstar::suspense::Complete for #complete_ident #generic_args
            where
                Self: ::std::marker::Send + 'static,
            {
                const PATH: &'static str = #path_lit;

                fn immediate_into(&self, buf: &mut ::std::string::String) -> ::std::result::Result<(), ::crabstar::askama::Error> {
                    use ::crabstar::askama::Template;
                    self.render_into(buf)
                }

                fn into_futures(self) -> ::std::vec::Vec<::crabstar::suspense::SuspenseFuture> {
                    ::std::vec::Vec::new()
                }
            }
        };
    }

    let suspensed_fields = suspense_fields.iter().map(|SuspenseField { ident: field_ident, output, .. }| {
        quote! {
            #vis #field_ident: ::std::pin::Pin<::std::boxed::Box<dyn ::std::future::Future<Output = ::std::result::Result<#output, ::crabstar::suspense::Error>> + ::std::marker::Send + 'static>>
        }
    });

    let suspensed_struct = quote! {
        #vis struct #suspense_ident {
            #(#suspensed_fields,)*
        }
    };

    let complete_struct = quote! {
        #vis struct #complete_ident #generic_params (#ident #generic_args, #suspense_ident);
    };

    let future_wrappers = suspense_fields.iter().map(|field| future_wrapper(field));

    let suspensed_impl = quote! {
        impl ::crabstar::suspense::Suspensed for #suspense_ident {
            fn into_futures(self) -> ::std::vec::Vec<::crabstar::suspense::SuspenseFuture> {
                vec![
                    #(#future_wrappers,)*
                ]
            }
        }
    };

    let complete_impl = quote! {
        impl #generic_params ::crabstar::suspense::Complete for #complete_ident #generic_args
        where
            Self: ::std::marker::Send + 'static,
        {
            const PATH: &'static str = #path_lit;

            fn immediate_into(&self, buf: &mut ::std::string::String) -> ::std::result::Result<(), ::crabstar::askama::Error> {
                use ::crabstar::askama::Template;
                self.0.render_into(buf)
            }

            fn into_futures(self) -> ::std::vec::Vec<::crabstar::suspense::SuspenseFuture> {
                use ::crabstar::suspense::Suspensed;
                self.1.into_futures()
            }
        }
    };

    let helper_constructor = quote! {
        impl #generic_params #ident #generic_args {
            #vis fn into_suspense(self, suspensed: #suspense_ident) -> #complete_ident #generic_args {
                #complete_ident(self, suspensed)
            }
        }
    };

    quote! {
        #suspensed_struct

        #complete_struct

        #suspensed_impl

        #complete_impl

        #helper_constructor
    }
}
