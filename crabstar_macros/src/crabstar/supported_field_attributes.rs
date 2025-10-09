use std::fmt::{Display, Formatter};

use quote::ToTokens;
use syn::{Error, Meta, Path, Token, punctuated::Punctuated, spanned::Spanned};

use crate::crabstar::fields::NamedField;

fn unsupported_attribute_error(
    a: &impl ToTokens,
    field_ident: impl Display,
    supported_list: &'static [&'static str],
) -> Error {
    Error::new_spanned(
        a,
        format!(
            "Unknown attribute `{field_ident}`. Supported attributes: `{}`",
            supported_list.join(", ")
        ),
    )
}

pub struct FieldAttributes;

impl FieldAttributes {
    const SUSPENSE: &str = "suspense";
    const SIGNAL: &str = "signal";

    const LIST: &[&str] = &[Self::SUSPENSE, Self::SIGNAL];

    pub fn is_suspense(path: &Path) -> bool {
        path.is_ident(Self::SUSPENSE)
    }

    pub fn is_signal(path: &Path) -> bool {
        path.is_ident(Self::SIGNAL)
    }

    pub fn is_handled_attribute(path: &Path) -> bool {
        Self::LIST.iter().any(|&attribute| path.is_ident(attribute))
    }

    pub fn validate<'a>(fields: impl IntoIterator<Item = &'a NamedField>) -> Result<(), Error> {
        fields
            .into_iter()
            .flat_map(|f| f.attrs.iter())
            .find_map(|a| {
                a.path()
                    .get_ident()
                    .filter(|field_ident| !Self::LIST.contains(&field_ident.to_string().as_str()))
                    .map(|field_ident| unsupported_attribute_error(a, field_ident, Self::LIST))
            })
            .map_or(Ok(()), Err)
    }
}

impl Display for FieldAttributes {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::LIST.join(", "))
    }
}

#[derive(Default)]
pub struct SignalFieldAttributes {
    pub id: bool,
}

impl SignalFieldAttributes {
    const ID: &str = "id";

    const LIST: &[&str] = &[Self::ID];

    fn is_id(path: &Path) -> bool {
        path.is_ident(Self::ID)
    }
}

impl TryFrom<&Meta> for SignalFieldAttributes {
    type Error = Error;

    fn try_from(value: &Meta) -> Result<Self, Self::Error> {
        let list = match value {
            Meta::List(list) => list,
            Meta::Path(_) => return Ok(Self::default()),
            _ => {
                return Err(Error::new(
                    value.span(),
                    "Unsupported signal attribute format. Expected `#[signal]` or `#[signal(...)]`",
                ));
            }
        };

        let metas = list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;

        let mut id = false;

        for m in metas {
            match m {
                Meta::Path(path) if Self::is_id(&path) => {
                    id = true;
                }
                _ => {
                    return Err(unsupported_attribute_error(
                        &m,
                        value.path().to_token_stream(),
                        Self::LIST,
                    ));
                }
            }
        }

        Ok(SignalFieldAttributes { id })
    }
}
