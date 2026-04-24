use ast::{
    Attribute, AttributeKind, AttributeName, AttributeValueNode, DataContent, DataExprValue,
    ElementBody, ElementNode, Node, ParenExpr, Toggle,
    component::{ComponentAttribute, ComponentAttributeValue, ComponentDefaultAttributes},
    control::{self, ControlBlock},
};
use syn::{Expr, Ident, Token, punctuated::Punctuated, spanned::Spanned};

fn paren_expr_len<N: Node>(paren_expr: &ParenExpr<N>) -> Option<usize> {
    span_len(&paren_expr.expr).map(|len| len + 2 + paren_expr.mode.prefix_len())
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
            Attribute::Data(data) => {
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
                        element_len += span_len(expr)?;
                    }
                    DataContent::Node(attribute_value_node) => {
                        element_len += attribute_value_len(attribute_value_node)?;
                    }
                    DataContent::Signals(decls) => {
                        element_len += data_decl_len_expr(decls)?;
                    }
                    DataContent::Kv(decls) => {
                        element_len += data_decl_len_attr_values(decls)?;
                    }
                    DataContent::Computed(decls) => {
                        element_len += data_decl_len_attr_values(decls)?;
                    }
                    DataContent::Empty | DataContent::Recovered => {}
                }
            }
        }
    }

    match body {
        ElementBody::Void => {
            // `;`
            element_len += 1;
        }
        ElementBody::Normal { .. } => {
            // always add open body brace at minimum
            // ` ` + `{`
            element_len += 2;
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
        ElementBody::Void => {
            // `;`
            element_len += 1;
        }
        ElementBody::Normal { .. } => {
            // always add open body brace at minimum
            // ` ` + `{`
            element_len += 2;
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
    span_len(&toggle.expr).map(|len| len + 2)
}

pub fn control_block_len_with<N: Node>(
    block: &ControlBlock<N>,
    node_len: impl Fn(&N) -> Option<usize>,
) -> Option<usize> {
    let mut element_len = 0usize;

    // `{` + ` `
    element_len += 2;

    for node in &block.nodes.0 {
        match node_len(node) {
            Some(value) => element_len += value,
            None => return None,
        }
        // ` `
        element_len += 1;
    }

    // `}`
    element_len += 1;

    Some(element_len)
}

fn control_attribute_value_len(control: &control::Control<AttributeValueNode>) -> Option<usize> {
    use control::{ControlIfOrBlock, ControlKind};

    let mut len = 1; // `@`

    match &control.kind {
        ControlKind::If(if_) => {
            // `if `
            len += 3;
            len += span_len(&if_.cond)?;
            // ` `
            len += 1;

            len += control_block_len_with(&if_.then_block, attribute_value_len)?;

            if let Some((_, if_or_block)) = &if_.else_branch {
                // ` @else `
                len += 7;

                match &**if_or_block {
                    ControlIfOrBlock::If(else_if) => {
                        // Recursively calculate the else-if length (without the leading @)
                        len += if_attribute_value_len(else_if)?;
                    }
                    ControlIfOrBlock::Block(block) => {
                        len += control_block_len_with(block, attribute_value_len)?;
                    }
                }
            }

            Some(len)
        }
        ControlKind::For(for_) => {
            // `for `
            len += 4;
            len += span_len(&for_.pat)?;
            // ` in `
            len += 4;
            len += span_len(&for_.expr)?;
            // ` `
            len += 1;

            len += control_block_len_with(&for_.block, attribute_value_len)?;

            Some(len)
        }
        ControlKind::While(while_) => {
            // `while `
            len += 6;
            len += span_len(&while_.cond)?;
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

fn if_attribute_value_len(if_: &control::If<AttributeValueNode>) -> Option<usize> {
    let mut len = 0;

    // `if `
    len += 3;
    len += span_len(&if_.cond)?;
    // ` `
    len += 1;

    len += control_block_len_with(&if_.then_block, attribute_value_len)?;

    if let Some((_, if_or_block)) = &if_.else_branch {
        // ` @else `
        len += 7;

        match &**if_or_block {
            control::ControlIfOrBlock::If(else_if) => {
                len += if_attribute_value_len(else_if)?;
            }
            control::ControlIfOrBlock::Block(block) => {
                len += control_block_len_with(block, attribute_value_len)?;
            }
        }
    }

    Some(len)
}

pub fn attribute_value_len(value: &AttributeValueNode) -> Option<usize> {
    match value {
        AttributeValueNode::Literal(lit) => span_len(lit),
        AttributeValueNode::Ident(ident) => span_len(ident),
        AttributeValueNode::Expr(paren_expr) => paren_expr_len(paren_expr),
        AttributeValueNode::Group(group) => attribute_value_group_len(&group.0.0),
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

pub fn data_decl_len_expr(decls: &Punctuated<DataExprValue<Expr>, Token![,]>) -> Option<usize> {
    let mut total_len = 0usize;
    let mut first = true;
    for d in decls {
        if first {
            first = false;
        } else {
            // `, `
            total_len += 2;
        }
        total_len += span_len(&d.ident)?;

        // `: `
        total_len += 2;

        total_len += span_len(&d.value)?;
    }
    Some(total_len)
}

pub fn data_decl_len_attr_values(
    decls: &Punctuated<DataExprValue<AttributeValueNode>, Token![,]>,
) -> Option<usize> {
    let mut total_len = 0usize;
    let mut first = true;
    for d in decls {
        if first {
            first = false;
        } else {
            // `, `
            total_len += 2;
        }

        total_len += data_decl_len_attr_value(d)?;
    }
    Some(total_len)
}

pub fn data_decl_len_attr_value(decl: &DataExprValue<AttributeValueNode>) -> Option<usize> {
    let mut total_len = 0usize;

    total_len += span_len(&decl.ident)?;

    // `: `
    total_len += 2;

    total_len += attribute_value_len(&decl.value)?;

    Some(total_len)
}
