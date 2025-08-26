use std::fmt::{Display, Formatter};

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Error, Fields, Ident, Path, Visibility};

use crate::helpers::{
    DelayedField, NamedField, complete_ident, lifetimes, partition_delayed_immediate_fields,
};

pub struct SupportedAttributes;

impl SupportedAttributes {
    const DELAYED: &str = "suspense";

    const LIST: &[&str] = &[Self::DELAYED];

    pub fn delayed(path: &Path) -> bool {
        path.is_ident(Self::DELAYED)
    }

    fn validate(fields: &Fields) -> Result<(), Error> {
        fields
            .iter()
            .flat_map(|f| f.attrs.iter())
            .find_map(|f| {
                f.path()
                    .get_ident()
                    .map(|ident| ident.to_string())
                    .filter(|name| !SupportedAttributes::LIST.contains(&name.as_str()))
                    .map(|name| {
                        Error::new_spanned(
                            f,
                            format!(
                                "Unknown attribute `{name}`. Supported attributes: {}",
                                SupportedAttributes::LIST.join(", ")
                            ),
                        )
                    })
            })
            .map_or(Ok(()), Err)
    }
}

impl Display for SupportedAttributes {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::LIST.join(", "))
    }
}

pub struct Shared<'a> {
    pub ident: &'a Ident,
    pub vis: Visibility,
    pub attrs: &'a [Attribute],
    pub generic_params: Vec<TokenStream>,
    pub immediate_fields: Vec<NamedField<'a>>,
    pub delayed_ident: Ident,
    pub boxed_delayed_ident: Ident,
    pub delayed_fields: Vec<DelayedField<'a>>,
    pub complete_ident: Ident,
    pub lifetimes: TokenStream,
}

pub fn shared<'a>(input: &'a DeriveInput) -> Result<Shared<'a>, Error> {
    let ident = &input.ident;
    let vis = input.vis.clone();
    let attrs = &input.attrs;

    let data_struct = match &input.data {
        Data::Struct(fields) => fields,
        _ => {
            return Err(Error::new_spanned(
                ident,
                "Suspense can be created only from regular structs with named fields",
            ));
        }
    };
    SupportedAttributes::validate(&data_struct.fields)?;
    let lifetimes = lifetimes(&input.generics);

    let named_fields = NamedField::from_fields(&data_struct.fields)?;
    let (delayed_fields, immediate_fields) = partition_delayed_immediate_fields(named_fields);

    let generic_params: Vec<TokenStream> = delayed_fields
        .iter()
        .map(|DelayedField { future, .. }| {
            quote! { #future }
        })
        .collect();

    let delayed_ident = Ident::new(&format!("{ident}Delayed"), ident.span());

    let complete_ident = complete_ident(&ident);
    let boxed_delayed_ident = Ident::new(&format!("{ident}BoxedDelayed"), ident.span());

    Ok(Shared {
        ident,
        vis,
        attrs,
        immediate_fields,
        generic_params,
        delayed_ident,
        boxed_delayed_ident,
        delayed_fields,
        complete_ident,
        lifetimes,
    })
}
