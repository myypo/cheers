use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Expr, ExprPath, FieldPat, File, Item, Local, Member, Pat, PatIdent, PatReference, Path, QSelf,
    Stmt,
};

const INDENT: usize = 4;

/// Where a pattern is printed: from `column` on a line indented by `line_indent`, followed on its
/// last line by `trailing` columns that cannot move to another line.
#[derive(Clone, Copy)]
pub struct PatPlace {
    pub column: usize,
    pub line_indent: usize,
    pub trailing: usize,
    pub line_length: usize,
}

impl PatPlace {
    fn fits(&self, flat: &str) -> bool {
        self.column + flat.len() + self.trailing <= self.line_length
    }

    /// Further along the same line, past `text`.
    fn after(self, text: &str) -> Self {
        Self {
            column: self.column + text.len(),
            ..self
        }
    }

    /// An item of a broken list, on a line of its own one indent deeper, followed by its `,`.
    fn item(self) -> Self {
        Self {
            column: self.line_indent + INDENT,
            line_indent: self.line_indent + INDENT,
            trailing: ",".len(),
            ..self
        }
    }
}

/// Lays `pat` out on one line when it fits at `place`, and otherwise breaks its outermost list
/// of fields, elements or cases one to a line, laying each out the same way. The first line
/// starts at `place.column`; every later line carries its own indentation.
pub fn unparse_pat(pat: &Pat, place: PatPlace) -> Vec<String> {
    if let Some(flat) = flat_pat(pat)
        && place.fits(&flat)
    {
        return vec![flat];
    }
    match pat {
        Pat::Or(or) => {
            let lead = if or.leading_vert.is_some() { "| " } else { "" };
            let mut lines = Vec::new();
            for (i, case) in or.cases.iter().enumerate() {
                let (indent, vert, case_place) = match i {
                    0 => (String::new(), lead, place.after(lead)),
                    _ => {
                        let indent = " ".repeat(place.line_indent);
                        let line = PatPlace {
                            column: place.line_indent,
                            ..place
                        };
                        (indent, "| ", line.after("| "))
                    }
                };
                let trailing = if i + 1 == or.cases.len() {
                    place.trailing
                } else {
                    0
                };
                let mut case_lines = unparse_pat(
                    case,
                    PatPlace {
                        trailing,
                        ..case_place
                    },
                );
                case_lines[0] = format!("{indent}{vert}{}", case_lines[0]);
                lines.extend(case_lines);
            }
            lines
        }
        Pat::Struct(v) if v.fields.iter().all(|f| f.attrs.is_empty()) => {
            let mut items: Vec<_> = v
                .fields
                .iter()
                .map(|f| field_lines(f, place.item()))
                .collect();
            if v.rest.is_some() {
                items.push(vec!["..".to_owned()]);
            }
            let open = format!("{} {{", path_text(&v.qself, &v.path));
            // `..` ends a struct pattern and takes no `,`.
            broken_list(&open, items, v.rest.is_none(), "}", place)
        }
        Pat::TupleStruct(v) => {
            let items = v
                .elems
                .iter()
                .map(|e| unparse_pat(e, place.item()))
                .collect();
            broken_list(
                &format!("{}(", path_text(&v.qself, &v.path)),
                items,
                true,
                ")",
                place,
            )
        }
        Pat::Tuple(v) => {
            let items = v
                .elems
                .iter()
                .map(|e| unparse_pat(e, place.item()))
                .collect();
            broken_list("(", items, true, ")", place)
        }
        Pat::Slice(v) => {
            let items = v
                .elems
                .iter()
                .map(|e| unparse_pat(e, place.item()))
                .collect();
            broken_list("[", items, true, "]", place)
        }
        Pat::Ident(
            v @ PatIdent {
                subpat: Some((_, sub)),
                ..
            },
        ) => prefixed(&ident_prefix(v), sub, place),
        Pat::Reference(v) => prefixed(reference_prefix(v), &v.pat, place),
        Pat::Paren(v) => {
            let inner = PatPlace {
                trailing: place.trailing + ")".len(),
                ..place.after("(")
            };
            let mut lines = prefixed("(", &v.pat, inner);
            lines.last_mut().expect("pattern lines").push(')');
            lines
        }
        pat => prettyplease_pat(pat, place.line_indent),
    }
}

/// `pat` on one line: as prettyplease prints it when it fits prettyplease's own width, and
/// otherwise put together from the one-line forms of its parts.
fn flat_pat(pat: &Pat) -> Option<String> {
    if !matches!(pat, Pat::Or(_))
        && let [line] = prettyplease_pat(pat, 0).as_slice()
    {
        return Some(line.clone());
    }
    let list = |elems: &mut dyn Iterator<Item = &Pat>| {
        elems
            .map(flat_pat)
            .collect::<Option<Vec<_>>>()
            .map(|v| v.join(", "))
    };
    Some(match pat {
        Pat::Or(or) => {
            let lead = if or.leading_vert.is_some() { "| " } else { "" };
            format!(
                "{lead}{}",
                or.cases
                    .iter()
                    .map(flat_pat)
                    .collect::<Option<Vec<_>>>()?
                    .join(" | ")
            )
        }
        Pat::Struct(v) if v.fields.iter().all(|f| f.attrs.is_empty()) => {
            let mut fields = v
                .fields
                .iter()
                .map(|f| match f.colon_token {
                    None => flat_pat(&f.pat),
                    Some(_) => Some(format!("{}: {}", member_text(&f.member), flat_pat(&f.pat)?)),
                })
                .collect::<Option<Vec<_>>>()?;
            fields.extend(v.rest.as_ref().map(|_| "..".to_owned()));
            let path = path_text(&v.qself, &v.path);
            match fields.is_empty() {
                true => format!("{path} {{}}"),
                false => format!("{path} {{ {} }}", fields.join(", ")),
            }
        }
        Pat::TupleStruct(v) => {
            format!(
                "{}({})",
                path_text(&v.qself, &v.path),
                list(&mut v.elems.iter())?
            )
        }
        Pat::Tuple(v) if v.elems.len() == 1 => format!("({},)", list(&mut v.elems.iter())?),
        Pat::Tuple(v) => format!("({})", list(&mut v.elems.iter())?),
        Pat::Slice(v) => format!("[{}]", list(&mut v.elems.iter())?),
        Pat::Ident(
            v @ PatIdent {
                subpat: Some((_, sub)),
                ..
            },
        ) => format!("{}{}", ident_prefix(v), flat_pat(sub)?),
        Pat::Reference(v) => format!("{}{}", reference_prefix(v), flat_pat(&v.pat)?),
        Pat::Paren(v) => format!("({})", flat_pat(&v.pat)?),
        _ => return None,
    })
}

fn member_text(member: &Member) -> String {
    match member {
        Member::Named(v) => v.to_string(),
        Member::Unnamed(v) => v.index.to_string(),
    }
}

/// `ref mut name @ ` before an identifier pattern's subpattern.
fn ident_prefix(v: &PatIdent) -> String {
    let ref_ = if v.by_ref.is_some() { "ref " } else { "" };
    let mut_ = if v.mutability.is_some() { "mut " } else { "" };
    format!("{ref_}{mut_}{} @ ", v.ident)
}

fn reference_prefix(v: &PatReference) -> &'static str {
    if v.mutability.is_some() { "&mut " } else { "&" }
}

/// A broken list: `open`, each item on lines of its own followed by a `,`, the last one only
/// when `last_comma`, then `close`.
fn broken_list(
    open: &str,
    items: Vec<Vec<String>>,
    last_comma: bool,
    close: &str,
    place: PatPlace,
) -> Vec<String> {
    let indent = " ".repeat(place.line_indent + INDENT);
    let mut lines = vec![open.to_owned()];
    let count = items.len();
    for (i, mut item) in items.into_iter().enumerate() {
        item[0] = format!("{indent}{}", item[0]);
        if last_comma || i + 1 < count {
            item.last_mut().expect("item lines").push(',');
        }
        lines.extend(item);
    }
    lines.push(format!("{}{close}", " ".repeat(place.line_indent)));
    lines
}

fn field_lines(field: &FieldPat, place: PatPlace) -> Vec<String> {
    if field.colon_token.is_none() {
        return unparse_pat(&field.pat, place);
    }
    prefixed(
        &format!("{}: ", member_text(&field.member)),
        &field.pat,
        place,
    )
}

/// `prefix` followed by `pat`, laid out in the room left after it.
fn prefixed(prefix: &str, pat: &Pat, place: PatPlace) -> Vec<String> {
    let mut lines = unparse_pat(pat, place.after(prefix));
    lines[0] = format!("{prefix}{}", lines[0]);
    lines
}

fn path_text(qself: &Option<QSelf>, path: &Path) -> String {
    let pat = Pat::Path(ExprPath {
        attrs: Vec::new(),
        qself: qself.clone(),
        path: path.clone(),
    });
    prettyplease_pat(&pat, 0).join(" ")
}

/// `pat`, which has no top-level `|` for `let` to refuse, as prettyplease prints it, with every
/// line after the first indented from `line_indent`.
fn prettyplease_pat(pat: &Pat, line_indent: usize) -> Vec<String> {
    let indent = " ".repeat(line_indent);
    let mut lines = unparse(quote!(let #pat;), 0);
    for (i, line) in lines.iter_mut().enumerate() {
        let rest = line.strip_prefix("    ").expect("statement indent");
        *line = match i {
            0 => rest.strip_prefix("let ").expect("let prefix").to_owned(),
            _ => format!("{indent}{rest}"),
        };
    }
    let last = lines.last_mut().expect("unparsed lines");
    *last = last.strip_suffix(';').expect("; suffix").to_owned();
    lines
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
