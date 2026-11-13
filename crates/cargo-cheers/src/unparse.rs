use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, File, Item, Local, Pat, Stmt};

pub fn unparse_pat(pat: &Pat, total_indent_size: usize) -> Vec<String> {
    // `let` takes no top-level `|`, so unparse in `if let`, which takes any pattern.
    let tokens = quote!(if let #pat = x {});
    let mut unparsed = unparse(tokens, total_indent_size);
    if let Some(first) = unparsed.first_mut() {
        *first = first
            .trim()
            .strip_prefix("if let ")
            .expect("if let prefix")
            .to_string();
    }
    if let Some(last) = unparsed.last_mut() {
        *last = last
            .strip_suffix(" = x {}")
            .expect("= x {} suffix")
            .to_string();
    }
    unparsed
}

pub fn unparse_local(local: &Local, total_indent_size: usize) -> Vec<String> {
    let tokens = quote!(#local;);
    let unparsed = unparse(tokens, total_indent_size);

    match unparsed.len() {
        0 => unparsed,
        1 => vec![
            unparsed[0]
                .trim()
                .strip_suffix(";")
                .expect("; suffix")
                .to_string(),
        ],
        _ => {
            let mut unparsed = unparsed;
            let last_idx = unparsed.len() - 1;
            unparsed[last_idx] = unparsed[last_idx]
                .strip_suffix(";")
                .expect("; suffix")
                .to_string();

            unparsed
        }
    }
}

pub fn unparse_expr(expr: &Expr, total_indent_size: usize) -> Vec<String> {
    let tokens = quote!(#expr);
    unparse(tokens, total_indent_size)
}

pub fn unparse_stmts(stmts: &Vec<Stmt>, total_indent_size: usize) -> Vec<String> {
    let tokens = quote!(#(#stmts)*);
    unparse(tokens, total_indent_size)
}

fn unparse(tokens: TokenStream, total_indent_size: usize) -> Vec<String> {
    let mut indented_tokens = tokens;
    for _ in 0..total_indent_size {
        indented_tokens = quote! {
            {
                ///
                #indented_tokens
            }
        };
    }

    let file = File {
        shebang: None,
        attrs: vec![],
        items: vec![
            //
            Item::Verbatim(quote::quote! {
                fn main() {
                    #indented_tokens
                }
            }),
        ],
    };

    let wrapped = prettyplease::unparse(&file);

    let indented_unwrapped = wrapped
        .strip_prefix("fn main() {\n")
        .expect("main function opened")
        .strip_suffix("}\n")
        .expect("main function closed")
        .lines()
        .map(|line| line.to_string())
        .collect::<Vec<_>>();

    indented_unwrapped[(2 * total_indent_size)..(indented_unwrapped.len() - total_indent_size)]
        .to_vec()
}
