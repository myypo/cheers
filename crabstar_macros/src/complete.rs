use std::fmt::Display;

use proc_macro2::Span;
use syn::Ident;

pub fn complete_ident(ident: &impl Display) -> Ident {
    Ident::new(&format!("{ident}Complete"), Span::call_site())
}
