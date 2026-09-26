//! Decides whether markup uses template-local bindings, which `@async` needs to know to pick
//! where its future is built.

use proc_macro2::{Spacing, TokenStream, TokenTree};
use syn::{
    Expr, ExprClosure, ExprPath, Ident, Item, Local, Macro, Pat,
    visit::{self, Visit},
};

use crate::{
    Attribute, AttributeKind, AttributeValueNode, DataContent, DataExpr, ElementBody, ElementNode,
    Node, Nodes, ParenExpr, ParenExprBody,
    component::{Component, ComponentAttribute, ComponentAttributeValue},
    control::{
        Control, ControlIfOrBlock, ControlKind, If, Let, MatchNodeArmBody, for_each_pat_ident,
    },
};

/// Whether `nodes` use any of `bindings`, not counting uses of names that `nodes` bind themselves
/// before the use.
///
/// Element, attribute and component names are not uses. Macro bodies are scanned as tokens, so
/// uncertain cases there count as uses.
pub fn nodes_use_any(nodes: &Nodes<ElementNode>, bindings: &[Ident]) -> bool {
    let mut scan = Scan {
        bindings,
        shadowed: Vec::new(),
        found: false,
    };
    nodes.uses(&mut scan);
    scan.found
}

struct Scan<'a> {
    bindings: &'a [Ident],
    shadowed: Vec<Ident>,
    found: bool,
}

impl Scan<'_> {
    fn use_ident(&mut self, ident: &Ident) {
        if !self.found && self.bindings.contains(ident) && !self.shadowed.contains(ident) {
            self.found = true;
        }
    }

    fn use_name(&mut self, name: &str) {
        if !self.found
            && self.bindings.iter().any(|binding| binding == name)
            && !self.shadowed.iter().any(|shadowed| shadowed == name)
        {
            self.found = true;
        }
    }

    fn scoped(&mut self, f: impl FnOnce(&mut Self)) {
        let len = self.shadowed.len();
        f(self);
        self.shadowed.truncate(len);
    }

    fn shadow_pat(&mut self, pat: &Pat) {
        for_each_pat_ident(pat, &mut |pat| self.shadowed.push(pat.ident.clone()));
    }

    fn expr(&mut self, expr: &Expr) {
        self.visit_expr(expr);
    }

    fn data_expr(&mut self, expr: &DataExpr) {
        self.expr(&expr.expr);
    }

    /// Scans `cond`, then `body` with the bindings of `cond`'s `let` chain in scope.
    fn cond(&mut self, cond: &Expr, body: impl FnOnce(&mut Self)) {
        fn chain(scan: &mut Scan<'_>, cond: &Expr) {
            match cond {
                Expr::Let(let_) => {
                    scan.expr(&let_.expr);
                    scan.shadow_pat(&let_.pat);
                }
                Expr::Binary(binary) if matches!(binary.op, syn::BinOp::And(_)) => {
                    chain(scan, &binary.left);
                    chain(scan, &binary.right);
                }
                cond => scan.expr(cond),
            }
        }

        self.scoped(|scan| {
            chain(scan, cond);
            body(scan);
        });
    }

    fn tokens(&mut self, tokens: TokenStream) {
        let tokens = tokens.into_iter().collect::<Vec<_>>();
        let is_punct = |idx: Option<usize>, ch: char, spacing: Option<Spacing>| {
            idx.and_then(|idx| tokens.get(idx)).is_some_and(|token| {
                matches!(token, TokenTree::Punct(punct)
                    if punct.as_char() == ch && spacing.is_none_or(|spacing| punct.spacing() == spacing))
            })
        };

        for (idx, token) in tokens.iter().enumerate() {
            match token {
                TokenTree::Ident(ident) => {
                    let prev = idx.checked_sub(1);
                    let prev_prev = prev.and_then(|prev| prev.checked_sub(1));
                    // `a..b` puts a joint `.` before the last one, while `a.b` does not.
                    let after_member_dot = is_punct(prev, '.', None)
                        && !is_punct(prev_prev, '.', Some(Spacing::Joint));
                    let after_path_sep =
                        is_punct(prev, ':', None) && is_punct(prev_prev, ':', Some(Spacing::Joint));
                    let before_path_sep = is_punct(Some(idx + 1), ':', Some(Spacing::Joint))
                        && is_punct(Some(idx + 2), ':', None);

                    if !after_member_dot && !after_path_sep && !before_path_sep {
                        self.use_ident(ident);
                    }
                }
                TokenTree::Group(group) => self.tokens(group.stream()),
                TokenTree::Literal(lit) => self.format_captures(&lit.to_string()),
                TokenTree::Punct(_) => {}
            }
        }
    }

    /// Scans the implicit captures of `format!`-style placeholders in a string literal: the
    /// argument in `{name}` and `{name:..}`, and width or precision arguments like `{:>w$}`.
    fn format_captures(&mut self, lit: &str) {
        if !lit.ends_with(['"', '#']) {
            return;
        }

        let mut rest = lit;
        while let Some(open) = rest.find('{') {
            rest = &rest[open + 1..];
            if let Some(escaped) = rest.strip_prefix('{') {
                rest = escaped;
                continue;
            }
            let Some(close) = rest.find('}') else {
                break;
            };
            let placeholder = &rest[..close];
            rest = &rest[close + 1..];

            let (argument, spec) = placeholder.split_once(':').unwrap_or((placeholder, ""));
            self.use_name(argument);
            for (dollar, _) in spec.match_indices('$') {
                let before = &spec[..dollar];
                let name_start = before
                    .rfind(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .map_or(0, |idx| idx + 1);
                self.use_name(&before[name_start..]);
            }
        }
    }
}

impl<'ast> Visit<'ast> for Scan<'_> {
    fn visit_expr_path(&mut self, path: &'ast ExprPath) {
        if path.qself.is_none()
            && let Some(ident) = path.path.get_ident()
        {
            self.use_ident(ident);
        }
    }

    fn visit_expr_closure(&mut self, closure: &'ast ExprClosure) {
        self.scoped(|scan| {
            for input in &closure.inputs {
                scan.shadow_pat(input);
            }
            scan.visit_expr(&closure.body);
        });
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.scoped(|scan| visit::visit_block(scan, block));
    }

    fn visit_local(&mut self, local: &'ast Local) {
        if let Some(init) = &local.init {
            self.visit_expr(&init.expr);
            if let Some((_, diverge)) = &init.diverge {
                self.visit_expr(diverge);
            }
        }
        self.shadow_pat(&local.pat);
    }

    fn visit_macro(&mut self, mac: &'ast Macro) {
        self.tokens(mac.tokens.clone());
    }

    fn visit_item(&mut self, _item: &'ast Item) {}
}

trait Uses {
    fn uses(&self, scan: &mut Scan<'_>);
}

impl<N: Node + Uses> Uses for Nodes<N> {
    fn uses(&self, scan: &mut Scan<'_>) {
        scan.scoped(|scan| {
            for node in &self.0 {
                node.uses(scan);
            }
        });
    }
}

impl Uses for ElementNode {
    fn uses(&self, scan: &mut Scan<'_>) {
        match self {
            Self::Element(element) => {
                for attr in &element.attrs {
                    attr.uses(scan);
                }
                element.body.uses(scan);
            }
            Self::Component(component) => component.uses(scan),
            Self::Literal(_) => {}
            Self::Control(control) => control.uses(scan),
            Self::Expr(expr) => expr.uses(scan),
            Self::Group(group) => group.nodes.uses(scan),
        }
    }
}

impl Uses for AttributeValueNode {
    fn uses(&self, scan: &mut Scan<'_>) {
        match self {
            Self::Literal(_) => {}
            Self::Group(group) => group.nodes.uses(scan),
            Self::Control(control) => control.uses(scan),
            Self::Expr(expr) => expr.uses(scan),
            Self::Ident(ident) => scan.use_ident(ident),
        }
    }
}

impl Uses for ElementBody {
    fn uses(&self, scan: &mut Scan<'_>) {
        match self {
            Self::Normal { children, .. } => children.uses(scan),
            Self::Void { .. } => {}
        }
    }
}

impl Uses for Attribute {
    fn uses(&self, scan: &mut Scan<'_>) {
        match self {
            Self::Regular { kind, .. } => match kind {
                AttributeKind::Value { value, toggle } => {
                    value.uses(scan);
                    if let Some(toggle) = toggle {
                        scan.expr(&toggle.expr);
                    }
                }
                AttributeKind::Empty(toggle) => {
                    if let Some(toggle) = toggle {
                        scan.expr(&toggle.expr);
                    }
                }
                AttributeKind::Option(toggle) => scan.expr(&toggle.expr),
            },
            Self::Data { data, .. } => data.content.uses(scan),
        }
    }
}

impl Uses for DataContent {
    fn uses(&self, scan: &mut Scan<'_>) {
        match self {
            Self::Node(node) => node.uses(scan),
            Self::Signals(signals) => {
                for signal in signals {
                    scan.data_expr(&signal.ident);
                    scan.expr(&signal.value);
                }
            }
            Self::Kv(pairs) | Self::Computed(pairs) => {
                for pair in pairs {
                    scan.data_expr(&pair.ident);
                    pair.value.uses(scan);
                }
            }
            Self::Bind(expr) => scan.data_expr(expr),
            Self::Show { value, initially } => {
                value.uses(scan);
                scan.expr(initially);
            }
            Self::Empty | Self::Recovered => {}
        }
    }
}

impl Uses for Component {
    fn uses(&self, scan: &mut Scan<'_>) {
        let default_attrs = self
            .default_attrs
            .iter()
            .flat_map(|default_attrs| &default_attrs.attrs);
        for attr in self.attrs.iter().chain(default_attrs) {
            attr.uses(scan);
        }
        self.body.uses(scan);
    }
}

impl Uses for ComponentAttribute {
    fn uses(&self, scan: &mut Scan<'_>) {
        match &self.value {
            Some(ComponentAttributeValue::Literal(_)) => {}
            Some(ComponentAttributeValue::Ident(ident)) => scan.use_ident(ident),
            Some(ComponentAttributeValue::Expr(expr)) => expr.uses(scan),
            None => scan.use_ident(&self.name),
        }
    }
}

impl<N: Node> Uses for ParenExpr<N> {
    fn uses(&self, scan: &mut Scan<'_>) {
        match &self.body {
            ParenExprBody::Unit => {}
            ParenExprBody::Expr(expr) => scan.expr(expr),
            ParenExprBody::Tuple(exprs) => {
                for expr in exprs {
                    scan.expr(expr);
                }
            }
        }
    }
}

impl<N: Node + Uses> Uses for Control<N> {
    fn uses(&self, scan: &mut Scan<'_>) {
        match &self.kind {
            ControlKind::Let(Let(local)) => scan.visit_local(local),
            ControlKind::If(if_) => if_.uses(scan),
            ControlKind::For(for_) => {
                scan.expr(&for_.expr);
                scan.scoped(|scan| {
                    scan.shadow_pat(&for_.pat);
                    for_.block.nodes.uses(scan);
                });
            }
            ControlKind::While(while_) => {
                scan.cond(&while_.cond, |scan| while_.block.nodes.uses(scan));
            }
            ControlKind::Match(match_) => {
                scan.expr(&match_.expr);
                for arm in &match_.arms {
                    scan.scoped(|scan| {
                        scan.shadow_pat(&arm.pat);
                        if let Some((_, guard)) = &arm.guard {
                            scan.expr(guard);
                        }
                        match &arm.body {
                            MatchNodeArmBody::Block(block) => block.nodes.uses(scan),
                            MatchNodeArmBody::Node(node) => node.uses(scan),
                        }
                    });
                }
            }
            ControlKind::Async(async_) => {
                async_.async_block.nodes.uses(scan);
                async_.else_block.nodes.uses(scan);
            }
        }
    }
}

impl<N: Node + Uses> Uses for If<N> {
    fn uses(&self, scan: &mut Scan<'_>) {
        scan.cond(&self.cond, |scan| self.then_block.nodes.uses(scan));
        if let Some((_, _, else_branch)) = &self.else_branch {
            match &**else_branch {
                ControlIfOrBlock::If(if_) => if_.uses(scan),
                ControlIfOrBlock::Block(block) => block.nodes.uses(scan),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use quote::{format_ident, quote};

    use super::nodes_use_any;
    use crate::Document;

    fn uses(bindings: &[&str], tokens: TokenStream) -> bool {
        let nodes = syn::parse2::<Document>(tokens).expect("markup should parse");
        let bindings = bindings
            .iter()
            .map(|binding| format_ident!("{binding}"))
            .collect::<Vec<_>>();
        nodes_use_any(&nodes, &bindings)
    }

    #[test]
    fn names_in_markup_syntax_are_not_uses() {
        assert!(!uses(
            &["id", "p", "Card", "title", "bind"],
            quote! {
                p id="x" {}
                input !bind(signal) {}
                Card title="x" {}
            }
        ));
    }

    #[test]
    fn paths_fields_and_methods_are_not_uses() {
        assert!(!uses(
            &["item", "len"],
            quote! {
                (item::render(other.item, other.len()))
            }
        ));
    }

    #[test]
    fn own_bindings_shadow_later_uses() {
        assert!(!uses(
            &["data", "x", "row"],
            quote! {
                @let data = load().await;
                (data)
                (items.iter().map(|x| x + 1).count())
                @for row in rows { (row) }
            }
        ));
    }

    #[test]
    fn own_binding_initializer_is_a_use() {
        assert!(uses(
            &["client"],
            quote! {
                @let client = client.clone();
                (client)
            }
        ));
    }

    #[test]
    fn shadowing_ends_with_its_block() {
        assert!(uses(
            &["row"],
            quote! {
                @for row in rows { (row) }
                (row)
            }
        ));
    }

    #[test]
    fn expression_uses_are_found() {
        for (binding, tokens) in [
            ("name", quote!(Card name;)),
            ("title", quote!(Card title=title;)),
            ("user", quote!((Foo { user }))),
            ("user", quote!((format!("{user}")))),
            ("user", quote!((format!("{user:?}")))),
            ("user", quote!((vec![user]))),
            ("user", quote!(p class=(user) {})),
            ("user", quote!(p { (user) })),
            ("user", quote!(@if user.admin { p {} })),
            ("user", quote!(@async { (user) } @else { p {} })),
            ("user", quote!(@async { p {} } @else { p { (user) } })),
        ] {
            assert!(uses(&[binding], tokens.clone()), "{tokens}");
        }
    }

    #[test]
    fn macro_paths_and_named_arguments_are_scanned_conservatively() {
        assert!(!uses(&["item"], quote!((format!("{}", item::NAME)))));
        assert!(!uses(&["name"], quote!((format!("{}", user.name)))));
        assert!(uses(&["value"], quote!((json!({ "a": value })))));
    }

    #[test]
    fn macro_ranges_and_struct_updates_are_uses() {
        for (binding, tokens) in [
            ("n", quote!((format!("{:?}", (0..n).collect::<Vec<_>>())))),
            ("n", quote!((format!("{:?}", (0..=n).collect::<Vec<_>>())))),
            ("base", quote!((vec![Foo { a: 1, ..base }]))),
        ] {
            assert!(uses(&[binding], tokens.clone()), "{tokens}");
        }
    }

    #[test]
    fn format_width_and_precision_captures_are_uses() {
        for (binding, tokens) in [
            ("w", quote!((format!("{:>w$}", "a")))),
            ("p", quote!((format!("{:.p$}", 1.5)))),
            ("p", quote!((format!("{value:>8.p$}")))),
        ] {
            assert!(uses(&[binding], tokens.clone()), "{tokens}");
        }
        assert!(!uses(&["w"], quote!((format!("{:>1$}", "a", 3)))));
    }
}
