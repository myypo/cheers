use syn::{Error, Meta, punctuated::Punctuated, spanned::Spanned};

#[derive(Default)]
pub struct ReactFieldAttr {
    pub granular: bool,
}

impl TryFrom<&Meta> for ReactFieldAttr {
    type Error = Error;

    fn try_from(value: &Meta) -> Result<Self, Self::Error> {
        let list = match value {
            Meta::List(list) => list,
            Meta::Path(_) => return Ok(Self::default()),
            _ => {
                return Err(Error::new(
                    value.span(),
                    "Unsupported signal attribute format. Expected `#[react]` or `#[react(...)]`",
                ));
            }
        };

        let opts = list.parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated)?;
        let opts = opts.into_iter().find_map(|o| {
            if o.path().is_ident("granular") {
                Some(Self { granular: true })
            } else {
                None
            }
        });

        Ok(opts.unwrap_or_default())
    }
}
