use syn::{Attribute, Error, Fields, Ident, Type, TypePath, Visibility, spanned::Spanned};

pub struct NamedField {
    pub ident: Ident,
    pub ty: Type,
    pub ty_path: TypePath,
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
}

impl NamedField {
    pub fn from_fields(fields: Fields) -> Result<Vec<Self>, Error> {
        let named_fields = fields
            .into_iter()
            .map(|f| match f.ident {
                Some(ident) => {
                    let ty = match f.ty {
                        Type::Reference(ref type_ref) => &*type_ref.elem,
                        _ => &f.ty,
                    };
                    let Type::Path(type_path) = &ty else {
                        return Err(Error::new(
                            ty.span(),
                            "Only named fields with explicit types are supported",
                        ));
                    };

                    Ok(NamedField {
                        ident,
                        ty: f.ty.clone(),
                        ty_path: type_path.clone(),
                        attrs: f.attrs,
                        vis: f.vis,
                    })
                }
                None => Err(Error::new(f.span(), "Tuple structs are not supported")),
            })
            .collect::<Result<Vec<NamedField>, Error>>()?;

        Ok(named_fields)
    }
}
