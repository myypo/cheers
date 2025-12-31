#![expect(missing_docs)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

mod action;
mod component;
mod shared;

use cheers_ast::{AttributeValueNode, Document, Nodes};
use syn::{ItemStruct, parse_macro_input};

use crate::{action::ActionArgs, shared::MaybeItemFn};

#[proc_macro_derive(Component, attributes(id, signal, form))]
pub fn component_derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item = parse_macro_input!(item as ItemStruct);

    component::generate(item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro]
pub fn html(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    cheers_ast::generate::lazy::<Document>(tokens.into(), true)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro]
pub fn html_borrow(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    cheers_ast::generate::lazy::<Document>(tokens.into(), false)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro]
pub fn html_static(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    cheers_ast::generate::literal::<Document>(tokens.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro]
pub fn attribute(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    cheers_ast::generate::lazy::<Nodes<AttributeValueNode>>(tokens.into(), true)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro]
pub fn attribute_borrow(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    cheers_ast::generate::lazy::<Nodes<AttributeValueNode>>(tokens.into(), false)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro]
pub fn attribute_static(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    cheers_ast::generate::literal::<Nodes<AttributeValueNode>>(tokens.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn action(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = parse_macro_input!(args as ActionArgs);
    let mut item = parse_macro_input!(item as MaybeItemFn);
    action::generate(args, &mut item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
