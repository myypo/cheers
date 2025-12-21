use cheers_ast::{
    Attribute, AttributeKind, AttributeName, AttributeValueNode, DataContent, DataDecl,
    ElementBody, ElementNode, Node, ParenExpr, Toggle,
    component::{ComponentAttribute, ComponentAttributeValue},
    control::ControlBlock,
};
use syn::{Ident, spanned::Spanned};

fn paren_expr_len<N: Node>(paren_expr: &ParenExpr<N>) -> Option<usize> {
    span_len(&paren_expr.expr).map(|len| len + 2)
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
                            element_len += toggle_len(&toggle)?;
                        };
                    }
                    AttributeKind::Option(toggle) => {
                        let len = toggle_len(&toggle)?;
                        // `=`
                        element_len += 1 + len;
                    }
                    AttributeKind::Empty(maybe_toggle) => {
                        if let Some(toggle) = maybe_toggle {
                            element_len += toggle_len(&toggle)?;
                        }

                        // Empty attribute with no toggle - no additional length
                    }
                }
            }
            Attribute::Data(data) => {
                // `!`
                element_len += 1;
                element_len += attribute_name_len(&data.name)?;

                // `(` + `)` = 2
                element_len += 2;

                match &data.content {
                    DataContent::Bind(expr) => {
                        element_len += span_len(expr)?;
                    }
                    DataContent::Node(attribute_value_node) => {
                        element_len += attribute_value_len(attribute_value_node)?;
                    }
                    DataContent::Signals(decls) | DataContent::IdentDecl(decls) => {
                        element_len += data_decl_len(decls)?;
                    }
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

        element_len += span_len(&attr.name)?;
        let Some(value) = &attr.value else {
            continue;
        };

        // `=`
        element_len += 1;

        match &value {
            ComponentAttributeValue::Literal(literal) => element_len += span_len(literal)?,
            ComponentAttributeValue::Ident(ident) => element_len += span_len(ident)?,
            ComponentAttributeValue::Expr(paren_expr) => {
                element_len += span_len(&paren_expr.expr).map(|len| len + 2)?
            }
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

pub fn control_block_len(block: &ControlBlock<ElementNode>) -> Option<usize> {
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

pub fn attribute_value_len(value: &AttributeValueNode) -> Option<usize> {
    match value {
        AttributeValueNode::Literal(lit) => span_len(lit),
        AttributeValueNode::Ident(ident) => span_len(ident),
        AttributeValueNode::Expr(paren_expr) => paren_expr_len(paren_expr),
        AttributeValueNode::Group(group) => attribute_value_group_len(&group.0.0),
        AttributeValueNode::Control(_) => None,
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

pub fn data_decl_len(
    decls: &syn::punctuated::Punctuated<DataDecl, syn::Token![,]>,
) -> Option<usize> {
    let mut total_len = 0usize;
    let mut first = true;
    for d in decls {
        if first {
            first = false;
        } else {
            // `, ` between declarations
            total_len += 2;
        }
        total_len += span_len(&d.ident)?;

        // `: `
        total_len += 2;

        total_len += span_len(&d.value)?;
    }
    Some(total_len)
}
