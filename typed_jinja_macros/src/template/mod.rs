use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error, Fields, spanned::Spanned};

use crate::template::opts::{TemplateOpts, template_opts};

mod opts;

fn minijinja_context(fields: &Fields) -> Result<TokenStream, Error> {
    let s = fields
        .iter()
        .map(|f| -> Result<String, Error> {
            let ident = f
                .ident
                .as_ref()
                .ok_or(Error::new(f.span(), "Anonymous fields are not supported"))?;

            Ok(format!("{ident} => self.{ident},"))
        })
        .collect::<Result<String, Error>>()?;

    s.parse().map_err(|e| {
        Error::new(
            fields.span(),
            format!("Unexpected error while parsing the built context for minijinja: {e}"),
        )
    })
}

pub fn expand_attr(args: TokenStream, input: &DeriveInput) -> Result<TokenStream, Error> {
    let ident = &input.ident;

    let fields = match &input.data {
        syn::Data::Struct(fields) => fields,
        _ => {
            return Err(Error::new(
                ident.span(),
                "Template can currently be derived only for structs",
            ));
        }
    };

    let TemplateOpts {
        path: template_path,
    } = template_opts(args)?;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let minijinja_context = minijinja_context(&fields.fields)?;

    let render_method_body = if cfg!(debug_assertions) {
        quote! {
            let mut env = ::typed_jinja::reloader().acquire_env().map_err(|e| ::typed_jinja::Error::Reload(Box::new(e)))?;

            let template = env.get_template(<Self as ::typed_jinja::Template>::PATH).map_err(|e| typed_jinja::Error::Render(Box::new(e)))?;

            let template = template.render(::typed_jinja::minijinja_context!(#minijinja_context));

            template.map_err(|e| ::typed_jinja::Error::Render(Box::new(e)))
        }
    } else {
        quote! {
            ::askama::Template::render(self).map_err(|e| ::typed_jinja::Error::Render(Box::new(e)))
        }
    };

    let askama_derive = if cfg!(all(debug_assertions, feature = "skip_dev_check")) {
        quote! {}
    } else {
        quote! {
            #[derive(::askama::Template)]
            #[template(path = #template_path)]
        }
    };

    Ok(quote! {
        #askama_derive
        #input

        impl #impl_generics ::typed_jinja::Template for #ident #ty_generics #where_clause {
            const PATH: &'static str = #template_path;

            fn render(&self) -> ::std::result::Result<String, ::typed_jinja::Error> {
                #render_method_body
            }
        }
    })
}
