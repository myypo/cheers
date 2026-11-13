use ast::{
    Attribute, AttributeKind, AttributeName, AttributeValueNode, DataContent, DataExpr,
    DataExprValue, DataModifierPart, DataModifiers, ElementBody, ElementNode, KvEntries, KvEntry,
    Node, ParenExpr, ParenExprBody, Toggle,
    component::{ComponentAttribute, ComponentAttributeValue, ComponentDefaultAttributes},
    control::{self, ControlBlock},
};
use syn::{
    Expr, ExprMacro, Ident, MacroDelimiter, Token, punctuated::Punctuated, spanned::Spanned,
    visit::Visit,
};

use crate::print::Printer;

/// A node a control block can hold.
pub(super) trait BlockNode: Node {
    /// The length of the node inside a block on one line, if it can be printed there.
    fn inline_len(&self, p: &Printer) -> Option<usize>;
}

impl BlockNode for ElementNode {
    fn inline_len(&self, p: &Printer) -> Option<usize> {
        p.node_len(self)
    }
}

impl BlockNode for AttributeValueNode {
    fn inline_len(&self, p: &Printer) -> Option<usize> {
        p.attribute_value_len(self)
    }
}

/// Whether `expr` holds a macro with a non-empty body in braces, which prettyplease prints over
/// lines.
fn has_braced_macro(expr: &Expr) -> bool {
    struct Finder(bool);
    impl<'ast> Visit<'ast> for Finder {
        fn visit_macro(&mut self, mac: &'ast syn::Macro) {
            self.0 |= matches!(mac.delimiter, MacroDelimiter::Brace(_)) && !mac.tokens.is_empty();
        }
    }
    let mut finder = Finder(false);
    finder.visit_expr(expr);
    finder.0
}

fn data_modifier_part_len(part: &DataModifierPart) -> Option<usize> {
    match part {
        DataModifierPart::Ident(ident) => span_len(&ident.0),
        DataModifierPart::Literal(literal) => span_len(&literal.lit_str()),
    }
}

fn data_modifiers_len(modifiers: &DataModifiers) -> Option<usize> {
    // `[` + `]`
    let mut len = 2;

    let mut first_modifier = true;
    for modifier in &modifiers.modifiers {
        if !first_modifier {
            // `, `
            len += 2;
        }

        len += data_modifier_part_len(&modifier.name)?;

        if modifier.paren_token.is_some() {
            // `(` + `)`
            len += 2;

            let mut first_tag = true;
            for tag in &modifier.tags {
                if !first_tag {
                    // `, `
                    len += 2;
                }

                len += data_modifier_part_len(tag)?;
                first_tag = false;
            }
        }

        first_modifier = false;
    }

    Some(len)
}

fn span_len<S: Spanned>(s: &S) -> Option<usize> {
    let span = s.span();
    let start = span.start();
    let end = span.end();

    if start.line != end.line {
        None
    } else {
        Some(end.column.saturating_sub(start.column))
    }
}

fn attribute_name_len(attr_name: &AttributeName) -> Option<usize> {
    match attr_name {
        AttributeName::Normal { name } => {
            let name_len = span_len(&name.0)?;
            Some(name_len)
        }
        AttributeName::Namespace { namespace, rest } => {
            let ns_len = span_len(&namespace.0)?;
            let rest_len = span_len(&rest.0)?;
            // namespace + `:` + rest
            let len = ns_len + 1 + rest_len;
            Some(len)
        }
        AttributeName::Unchecked(lit) => span_len(lit),
    }
}

fn comma_separated_len<T>(
    items: impl IntoIterator<Item = T>,
    mut item_len: impl FnMut(T) -> Option<usize>,
) -> Option<usize> {
    let mut total_len = 0usize;
    for (idx, item) in items.into_iter().enumerate() {
        if idx > 0 {
            // `, `
            total_len += 2;
        }
        total_len += item_len(item)?;
    }
    Some(total_len)
}

fn body_len(body: &ElementBody) -> usize {
    match body {
        ElementBody::Void { .. } => ";".len(),
        // An empty body closes on the same line.
        ElementBody::Normal { children, .. } if children.0.is_empty() => " {}".len(),
        ElementBody::Normal { .. } => " {".len(),
    }
}

impl<'a, 'b> Printer<'a, 'b> {
    /// The length of `expr` on one line, if it is printed on one.
    fn expr_len(&self, expr: &Expr) -> Option<usize> {
        match expr {
            Expr::Macro(expr_macro) => self.macro_len(expr_macro),
            expr if has_braced_macro(expr) => None,
            expr => span_len(expr),
        }
    }

    /// The length of a macro printed as a whole expression: nested markup as it formats on one
    /// line, and any other macro as written.
    fn macro_len(&self, expr_macro: &ExprMacro) -> Option<usize> {
        match self.format_nested_macro(expr_macro, 0, 0) {
            Some(text) => (!text.contains('\n')).then(|| text.chars().count()),
            None => span_len(expr_macro),
        }
    }

    fn paren_expr_len<N: Node>(&self, paren_expr: &ParenExpr<N>) -> Option<usize> {
        let body_len = match &paren_expr.body {
            ParenExprBody::Unit => 0,
            ParenExprBody::Expr(expr) => self.expr_len(expr)?,
            ParenExprBody::Tuple(elems) => {
                let mut len = 0;

                for (idx, expr) in elems.iter().enumerate() {
                    len += self.expr_len(expr)?;

                    if idx + 1 < elems.len() {
                        // `, `
                        len += 2;
                    }
                }

                if elems.trailing_punct() {
                    // `,`
                    len += 1;
                }

                len
            }
        };

        Some(body_len + 2 + paren_expr.mode.prefix_len())
    }

    fn data_expr_len(&self, expr: &DataExpr) -> Option<usize> {
        self.expr_len(&expr.expr).map(|len| {
            len + expr.mode.prefix_len() + if expr.paren_token.is_some() { 2 } else { 0 }
        })
    }

    fn component_attr_value_len(&self, value: &ComponentAttributeValue) -> Option<usize> {
        match value {
            ComponentAttributeValue::Literal(literal) => span_len(literal),
            ComponentAttributeValue::Ident(ident) => span_len(ident),
            ComponentAttributeValue::Expr(paren_expr) => self.paren_expr_len(paren_expr),
        }
    }

    fn component_attr_len(&self, attr: &ComponentAttribute) -> Option<usize> {
        let mut len = span_len(&attr.name)?;

        if let Some(value) = &attr.value {
            len += 1;
            len += self.component_attr_value_len(value)?;
        }

        Some(len)
    }

    pub(super) fn node_len(&self, node: &ElementNode) -> Option<usize> {
        match node {
            ElementNode::Literal(lit) => span_len(lit),
            ElementNode::Expr(paren_expr) => self.paren_expr_len(paren_expr),
            _ => None,
        }
    }

    pub(super) fn element_len(
        &self,
        ident: &Ident,
        attrs: &[Attribute],
        body: &ElementBody,
    ) -> Option<usize> {
        let mut element_len = 0usize;

        // name
        element_len += span_len(ident)?;

        // attributes
        for attr in attrs {
            // ` `
            element_len += 1;

            match attr {
                Attribute::Regular { name, kind } => {
                    element_len += attribute_name_len(name)?;

                    match kind {
                        AttributeKind::Value { value, toggle } => {
                            // `=`
                            element_len += 1;
                            element_len += self.attribute_value_len(value)?;
                            if let Some(toggle) = toggle {
                                element_len += self.toggle_len(toggle)?;
                            };
                        }
                        AttributeKind::Option(toggle) => {
                            let len = self.toggle_len(toggle)?;
                            // `=`
                            element_len += 1 + len;
                        }
                        AttributeKind::Empty(maybe_toggle) => {
                            if let Some(toggle) = maybe_toggle {
                                element_len += self.toggle_len(toggle)?;
                            }

                            // Empty attribute with no toggle - no additional length
                        }
                    }
                }
                Attribute::Data { data, .. } => {
                    // `!`
                    element_len += 1;

                    if let Some(namespace) = &data.namespace {
                        element_len += span_len(&namespace.0)?;
                        // `:`
                        element_len += 1;
                    }

                    if let Some(name) = data.name.ident() {
                        element_len += span_len(&name.0)?;
                    }

                    if data.has_parens() {
                        // `(` + `)` = 2
                        element_len += 2;
                    }

                    match &data.content {
                        DataContent::Bind(expr) => {
                            element_len += self.data_expr_len(expr)?;
                        }
                        DataContent::Node(attribute_value_node) => {
                            element_len += self.attribute_value_len(attribute_value_node)?;
                        }
                        DataContent::Signals(decls) => {
                            element_len += self.data_decl_len_expr(decls)?;
                        }
                        DataContent::Kv(entries) => {
                            element_len += self.kv_entries_len(entries)?;
                        }
                        DataContent::Computed(decls) => {
                            element_len +=
                                comma_separated_len(decls, |d| self.data_decl_len_attr_value(d))?;
                        }
                        DataContent::Show { value, initially } => {
                            // `, initially: `
                            element_len += 13;
                            element_len += self.attribute_value_len(value)?;
                            element_len += span_len(initially)?;
                        }
                        DataContent::Empty => {}
                        // Printed back verbatim, so it takes its original width (parens counted
                        // above).
                        DataContent::Recovered => {
                            if let Some(paren_span) = data.paren_span() {
                                let span = paren_span.join();
                                let (start, end) = (span.start(), span.end());
                                if start.line != end.line {
                                    return None;
                                }
                                element_len +=
                                    end.column.saturating_sub(start.column).saturating_sub(2);
                            }
                        }
                    }

                    if let Some(modifiers) = &data.modifiers {
                        element_len += data_modifiers_len(modifiers)?;
                    }
                }
            }
        }

        Some(element_len + body_len(body))
    }

    pub(super) fn component_len(
        &self,
        ident: &Ident,
        attrs: &[ComponentAttribute],
        default_attrs: Option<&ComponentDefaultAttributes>,
        dotdot: bool,
        body: &ElementBody,
    ) -> Option<usize> {
        let mut element_len = 0usize;

        // name
        element_len += span_len(ident)?;

        // attributes
        for attr in attrs {
            // ` `
            element_len += 1;

            element_len += self.component_attr_len(attr)?;
        }

        if let Some(default_attrs) = default_attrs {
            // ` ` + `(` + `)`
            element_len += 3;

            for (idx, attr) in default_attrs.attrs.iter().enumerate() {
                if idx > 0 {
                    // ` `
                    element_len += 1;
                }

                element_len += self.component_attr_len(attr)?;
            }
        }

        if dotdot {
            // ` ` + `..`
            element_len += 3;
        }

        Some(element_len + body_len(body))
    }

    fn toggle_len(&self, toggle: &Toggle) -> Option<usize> {
        // add 2 for `[` + `]`
        self.expr_len(&toggle.expr).map(|len| len + 2)
    }

    pub(super) fn control_block_len<N: BlockNode>(&self, block: &ControlBlock<N>) -> Option<usize> {
        let mut element_len = 0usize;

        // `{` + ` `
        element_len += 2;

        for node in &block.nodes.0 {
            element_len += node.inline_len(self)?;
            // ` `
            element_len += 1;
        }

        // `}`
        element_len += 1;

        Some(element_len)
    }

    fn control_attribute_value_len(
        &self,
        control: &control::Control<AttributeValueNode>,
    ) -> Option<usize> {
        use control::ControlKind;

        let mut len = 1; // `@`

        match &control.kind {
            ControlKind::If(if_) => {
                len += self.if_len(if_)?;
                Some(len)
            }
            ControlKind::For(for_) => {
                // `for `
                len += 4;
                len += span_len(&for_.pat)?;
                // ` in `
                len += 4;
                len += self.expr_len(&for_.expr)?;
                // ` `
                len += 1;

                len += self.control_block_len(&for_.block)?;

                Some(len)
            }
            ControlKind::While(while_) => {
                // `while `
                len += 6;
                len += self.expr_len(&while_.cond)?;
                // ` `
                len += 1;

                // block
                len += self.control_block_len(&while_.block)?;

                Some(len)
            }
            ControlKind::Match(_) => None,
            ControlKind::Let(_) => None,
            ControlKind::Async(_) => None,
        }
    }

    /// The length of `if cond { .. }` and its `@else` branches on one line, without the leading
    /// `@`.
    fn if_len<N: BlockNode>(&self, if_: &control::If<N>) -> Option<usize> {
        let cond = self.expr_len(&if_.cond)?;
        Some("if ".len() + cond + " ".len() + self.if_blocks_len(if_)?)
    }

    /// The length of an `@if`'s block and its `@else` branches on one line.
    pub(super) fn if_blocks_len<N: BlockNode>(&self, if_: &control::If<N>) -> Option<usize> {
        let then_block = self.control_block_len(&if_.then_block)?;
        let else_branch = match if_.else_branch.as_ref().map(|(_, _, v)| &**v) {
            None => 0,
            Some(control::ControlIfOrBlock::If(else_if)) => {
                " @else ".len() + self.if_len(else_if)?
            }
            Some(control::ControlIfOrBlock::Block(block)) => {
                " @else ".len() + self.control_block_len(block)?
            }
        };
        Some(then_block + else_branch)
    }

    pub(super) fn attribute_value_len(&self, value: &AttributeValueNode) -> Option<usize> {
        match value {
            AttributeValueNode::Literal(lit) => span_len(lit),
            AttributeValueNode::Ident(ident) => span_len(ident),
            AttributeValueNode::Expr(paren_expr) => self.paren_expr_len(paren_expr),
            AttributeValueNode::Group(group) => self.attribute_value_group_len(&group.nodes.0),
            AttributeValueNode::Control(control) => self.control_attribute_value_len(control),
        }
    }

    fn attribute_value_group_len(&self, nodes: &[AttributeValueNode]) -> Option<usize> {
        // start with 2 for `{` and `}`
        let mut len = 2;
        for (i, node) in nodes.iter().enumerate() {
            if i == 0 {
                // first ` ` after `{`
                len += 1;
            }
            len += self.attribute_value_len(node)?;
            // ` ` after attribute
            len += 1;
        }
        Some(len)
    }

    pub(super) fn data_decl_len_expr(
        &self,
        decls: &Punctuated<DataExprValue<Expr>, Token![,]>,
    ) -> Option<usize> {
        comma_separated_len(decls, |d| {
            // `: `
            Some(self.data_expr_len(&d.ident)? + 2 + self.expr_len(&d.value)?)
        })
    }

    pub(super) fn kv_entries_len(&self, entries: &KvEntries) -> Option<usize> {
        comma_separated_len(entries, |entry| match entry {
            KvEntry::Pair(pair) => self.data_decl_len_attr_value(pair),
            // `..`
            KvEntry::Spread { value, .. } => Some(2 + self.data_expr_len(value)?),
        })
    }

    fn data_decl_len_attr_value(&self, decl: &DataExprValue<AttributeValueNode>) -> Option<usize> {
        // `: `
        Some(self.data_expr_len(&decl.ident)? + 2 + self.attribute_value_len(&decl.value)?)
    }
}
