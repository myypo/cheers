use syn::{Attribute, Error, Fields, Ident, Type, TypePath, Visibility, spanned::Spanned};

pub struct NamedField {
    pub ident: Ident,
    pub ty: TypePath,
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
}

impl NamedField {
    pub fn from_fields(fields: Fields) -> Result<Vec<Self>, Error> {
        let named_fields = fields
            .into_iter()
            .map(|f| match f.ident {
                Some(ident) => match f.ty {
                    Type::Path(type_path) => Ok(NamedField {
                        ident,
                        ty: type_path,
                        attrs: f.attrs,
                        vis: f.vis,
                    }),
                    _ => Err(Error::new(ident.span(), "Only regular types are supported")),
                },
                None => Err(Error::new(f.span(), "Tuple structs are not supported")),
            })
            .collect::<Result<Vec<NamedField>, Error>>()?;

        Ok(named_fields)
    }
}
