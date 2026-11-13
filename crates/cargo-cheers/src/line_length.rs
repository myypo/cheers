use ast::{
    Attribute, AttributeKind, AttributeName, AttributeValueNode, DataContent, DataExpr,
    DataExprValue, DataModifierPart, DataModifiers, ElementBody, ElementNode, KvEntries, KvEntry,
    Node, ParenExpr, ParenExprBody, Toggle,
    component::{ComponentAttribute, ComponentAttributeValue, ComponentDefaultAttributes},
    control::{self, ControlBlock},
};
use syn::{
    Expr, Ident, MacroDelimiter, Token, punctuated::Punctuated, spanned::Spanned, visit::Visit,
};

/// Whether `expr` holds a macro with a non-empty body in braces, which is printed over lines.
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

/// The length of `expr` on one line, if it is printed on one.
fn expr_len(expr: &Expr) -> Option<usize> {
    if has_braced_macro(expr) {
        return None;
    }
    span_len(expr)
}

fn paren_expr_len<N: Node>(paren_expr: &ParenExpr<N>) -> Option<usize> {
    let body_len = match &paren_expr.body {
        ParenExprBody::Unit => 0,
        ParenExprBody::Expr(expr) => expr_len(expr)?,
        ParenExprBody::Tuple(elems) => {
            let mut len = 0;

            for (idx, expr) in elems.iter().enumerate() {
                len += expr_len(expr)?;

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

fn data_expr_len(expr: &DataExpr) -> Option<usize> {
    expr_len(&expr.expr)
        .map(|len| len + expr.mode.prefix_len() + if expr.paren_token.is_some() { 2 } else { 0 })
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

fn component_attr_value_len(value: &ComponentAttributeValue) -> Option<usize> {
    match value {
        ComponentAttributeValue::Literal(literal) => span_len(literal),
        ComponentAttributeValue::Ident(ident) => span_len(ident),
        ComponentAttributeValue::Expr(paren_expr) => paren_expr_len(paren_expr),
    }
}

fn component_attr_len(attr: &ComponentAttribute) -> Option<usize> {
    let mut len = span_len(&attr.name)?;

    if let Some(value) = &attr.value {
        len += 1;
        len += component_attr_value_len(value)?;
    }

    Some(len)
}

pub fn node_len(node: &ElementNode) -> Option<usize> {
    match node {
        ElementNode::Literal(lit) => span_len(lit),
        // Add 2 for `(` + `)`
        ElementNode::Expr(paren_expr) => paren_expr_len(paren_expr),
        _ => None,
    }
}

pub fn element_len(ident: &Ident, attrs: &[Attribute], body: &ElementBody) -> Option<usize> {
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
                        element_len += attribute_value_len(value)?;
                        if let Some(toggle) = toggle {
                            element_len += toggle_len(toggle)?;
                        };
                    }
                    AttributeKind::Option(toggle) => {
                        let len = toggle_len(toggle)?;
                        // `=`
                        element_len += 1 + len;
                    }
                    AttributeKind::Empty(maybe_toggle) => {
                        if let Some(toggle) = maybe_toggle {
                            element_len += toggle_len(toggle)?;
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
                        element_len += data_expr_len(expr)?;
                    }
                    DataContent::Node(attribute_value_node) => {
                        element_len += attribute_value_len(attribute_value_node)?;
                    }
                    DataContent::Signals(decls) => {
                        element_len += data_decl_len_expr(decls)?;
                    }
                    DataContent::Kv(entries) => {
                        element_len += kv_entries_len(entries)?;
                    }
                    DataContent::Computed(decls) => {
                        element_len += data_decl_len_attr_values(decls)?;
                    }
                    DataContent::Show { value, initially } => {
                        // `, initially: `
                        element_len += 13;
                        element_len += attribute_value_len(value)?;
                        element_len += span_len(initially)?;
                    }
                    DataContent::Empty => {}
                    // Printed back verbatim, so it takes its original width (parens counted above).
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

    match body {
        ElementBody::Void { .. } => {
            // `;`
            element_len += 1;
        }
        // An empty body closes on the same line.
        ElementBody::Normal { children, .. } if children.0.is_empty() => {
            element_len += " {}".len();
        }
        ElementBody::Normal { .. } => {
            element_len += " {".len();
        }
    }

    Some(element_len)
}

pub fn component_len(
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

        element_len += component_attr_len(attr)?;
    }

    if let Some(default_attrs) = default_attrs {
        // ` ` + `(` + `)`
        element_len += 3;

        for (idx, attr) in default_attrs.attrs.iter().enumerate() {
            if idx > 0 {
                // ` `
                element_len += 1;
            }

            element_len += component_attr_len(attr)?;
        }
    }

    if dotdot {
        // ` ` + `..`
        element_len += 3;
    }

    match body {
        ElementBody::Void { .. } => {
            // `;`
            element_len += 1;
        }
        // An empty body closes on the same line.
        ElementBody::Normal { children, .. } if children.0.is_empty() => {
            element_len += " {}".len();
        }
        ElementBody::Normal { .. } => {
            element_len += " {".len();
        }
    }

    Some(element_len)
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

fn toggle_len(toggle: &Toggle) -> Option<usize> {
    // add 2 for `[` + `]`
    expr_len(&toggle.expr).map(|len| len + 2)
}

pub fn control_block_len_with<N: Node>(
    block: &ControlBlock<N>,
    node_len: impl Fn(&N) -> Option<usize>,
) -> Option<usize> {
    let mut element_len = 0usize;

    // `{` + ` `
    element_len += 2;

    for node in &block.nodes.0 {
        element_len += node_len(node)?;
        // ` `
        element_len += 1;
    }

    // `}`
    element_len += 1;

    Some(element_len)
}

fn control_attribute_value_len(control: &control::Control<AttributeValueNode>) -> Option<usize> {
    use control::ControlKind;

    let mut len = 1; // `@`

    match &control.kind {
        ControlKind::If(if_) => {
            len += if_len_with(if_, attribute_value_len)?;
            Some(len)
        }
        ControlKind::For(for_) => {
            // `for `
            len += 4;
            len += span_len(&for_.pat)?;
            // ` in `
            len += 4;
            len += expr_len(&for_.expr)?;
            // ` `
            len += 1;

            len += control_block_len_with(&for_.block, attribute_value_len)?;

            Some(len)
        }
        ControlKind::While(while_) => {
            // `while `
            len += 6;
            len += expr_len(&while_.cond)?;
            // ` `
            len += 1;

            // block
            len += control_block_len_with(&while_.block, attribute_value_len)?;

            Some(len)
        }
        ControlKind::Match(_) => None,
        ControlKind::Let(_) => None,
        ControlKind::Async(_) => None,
    }
}

/// The length of `if cond { .. }` and its `@else` branches on one line, without the leading `@`.
fn if_len_with<N: Node>(
    if_: &control::If<N>,
    node_len: impl Fn(&N) -> Option<usize> + Copy,
) -> Option<usize> {
    let cond = expr_len(&if_.cond)?;
    Some("if ".len() + cond + " ".len() + if_blocks_len_with(if_, node_len)?)
}

/// The length of an `@if`'s block and its `@else` branches on one line.
pub fn if_blocks_len_with<N: Node>(
    if_: &control::If<N>,
    node_len: impl Fn(&N) -> Option<usize> + Copy,
) -> Option<usize> {
    let then_block = control_block_len_with(&if_.then_block, node_len)?;
    let else_branch = match if_.else_branch.as_ref().map(|(_, _, v)| &**v) {
        None => 0,
        Some(control::ControlIfOrBlock::If(else_if)) => {
            " @else ".len() + if_len_with(else_if, node_len)?
        }
        Some(control::ControlIfOrBlock::Block(block)) => {
            " @else ".len() + control_block_len_with(block, node_len)?
        }
    };
    Some(then_block + else_branch)
}

pub fn attribute_value_len(value: &AttributeValueNode) -> Option<usize> {
    match value {
        AttributeValueNode::Literal(lit) => span_len(lit),
        AttributeValueNode::Ident(ident) => span_len(ident),
        AttributeValueNode::Expr(paren_expr) => paren_expr_len(paren_expr),
        AttributeValueNode::Group(group) => attribute_value_group_len(&group.nodes.0),
        AttributeValueNode::Control(control) => control_attribute_value_len(control),
    }
}

fn attribute_value_group_len(nodes: &[AttributeValueNode]) -> Option<usize> {
    // start with 2 for `{` and `}`
    let mut len = 2;
    for (i, node) in nodes.iter().enumerate() {
        if i == 0 {
            // first ` ` after `{`
            len += 1;
        }
        len += attribute_value_len(node)?;
        // ` ` after attribute
        len += 1;
    }
    Some(len)
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

pub fn data_decl_len_expr(decls: &Punctuated<DataExprValue<Expr>, Token![,]>) -> Option<usize> {
    comma_separated_len(decls, |d| {
        // `: `
        Some(data_expr_len(&d.ident)? + 2 + expr_len(&d.value)?)
    })
}

pub fn data_decl_len_attr_values(
    decls: &Punctuated<DataExprValue<AttributeValueNode>, Token![,]>,
) -> Option<usize> {
    comma_separated_len(decls, data_decl_len_attr_value)
}

pub fn kv_entries_len(entries: &KvEntries) -> Option<usize> {
    comma_separated_len(entries, |entry| match entry {
        KvEntry::Pair(pair) => data_decl_len_attr_value(pair),
        // `..`
        KvEntry::Spread { value, .. } => Some(2 + data_expr_len(value)?),
    })
}

pub fn data_decl_len_attr_value(decl: &DataExprValue<AttributeValueNode>) -> Option<usize> {
    let mut total_len = 0usize;

    total_len += data_expr_len(&decl.ident)?;

    // `: `
    total_len += 2;

    total_len += attribute_value_len(&decl.value)?;

    Some(total_len)
}
