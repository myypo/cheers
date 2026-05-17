use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, Error, Field, Generics, Ident, ItemStruct, LitStr, Meta, Path, Token, Type,
    Visibility,
    parse::{Parse, ParseStream, Parser},
    parse_quote, parse2,
    spanned::Spanned,
};

use crate::{
    cheers::{filter_outer_attrs, to_owned_type},
    shared::{filter_generics, parse_named_type},
};

struct FormArgs {
    name: Ident,
    ty: Type,
    attrs: Option<TokenStream>,
}

struct FormDerives {
    tokens: TokenStream,
    projection_bounds: Vec<TokenStream>,
}

struct FormFieldArgs {
    flatten: bool,
    attrs: Vec<TokenStream>,
}

struct InnerFormField {
    field_ident: Ident,
    form_ident: Ident,
    vis: Visibility,
    ty: Type,
    attrs: Vec<TokenStream>,
    flatten: bool,
}

enum FormNameEntry {
    Simple { ident: Ident, field_name: LitStr },
    Flatten { ident: Ident, ty: Box<Type> },
}

struct FormFieldDecl {
    decl: TokenStream,
    ty: Type,
}

struct FlattenedForms {
    component_types: Vec<Type>,
}

struct FormBounds<'a> {
    flattened_forms: &'a FlattenedForms,
    derive_projection_bounds: &'a [TokenStream],
}

struct FormNamesOutput {
    tokens: TokenStream,
    ty: TokenStream,
    value: TokenStream,
}

impl Parse for FormArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let (name, ty) = parse_named_type(
            input,
            r#"expected a colon and type after form field name, like #[form(name: Type)]"#,
        )?;

        Ok(Self {
            name,
            ty,
            attrs: if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
                Some(input.parse()?)
            } else {
                None
            },
        })
    }
}

fn new_form_names_field_ident(ident: &Ident) -> Ident {
    let mut s = String::from("form_");
    s.push_str(&ident.to_string());

    Ident::new(&s, ident.span())
}

impl FormNameEntry {
    fn ident(&self) -> &Ident {
        match self {
            Self::Simple { ident, .. } | Self::Flatten { ident, .. } => ident,
        }
    }

    fn value_tokens(&self) -> TokenStream {
        match self {
            Self::Simple { field_name, .. } => {
                quote! { ::cheers::prelude::FormName::__static(#field_name) }
            }
            Self::Flatten { ty, .. } => {
                let ty = ty.as_ref();
                quote! { <#ty as ::cheers::__internal::FormComponent>::__FORM_NAMES }
            }
        }
    }

    fn ty_tokens(&self) -> TokenStream {
        match self {
            Self::Simple { .. } => quote! { ::cheers::prelude::FormName },
            Self::Flatten { ty, .. } => {
                let ty = ty.as_ref();
                quote! { <#ty as ::cheers::__internal::FormComponent>::FormNames }
            }
        }
    }

    fn flattened_component_ty(&self) -> Option<&Type> {
        match self {
            Self::Simple { .. } => None,
            Self::Flatten { ty, .. } => Some(ty.as_ref()),
        }
    }
}

impl FlattenedForms {
    fn from_entries(entries: &[FormNameEntry]) -> Self {
        let component_types = entries
            .iter()
            .filter_map(FormNameEntry::flattened_component_ty)
            .cloned()
            .collect();

        Self { component_types }
    }

    fn component_types(&self) -> &[Type] {
        &self.component_types
    }

    fn is_empty(&self) -> bool {
        self.component_types.is_empty()
    }
}

impl<'a> FormBounds<'a> {
    fn new(flattened_forms: &'a FlattenedForms, form_derives: Option<&'a FormDerives>) -> Self {
        Self {
            flattened_forms,
            derive_projection_bounds: form_derives
                .map(|derives| derives.projection_bounds.as_slice())
                .unwrap_or(&[]),
        }
    }

    fn add_component_bounds(&self, generics: &mut Generics) {
        if self.flattened_forms.is_empty() {
            return;
        }

        let where_clause = generics.make_where_clause();
        for ty in self.flattened_forms.component_types() {
            where_clause
                .predicates
                .push(parse_quote!(#ty: ::cheers::__internal::FormComponent));
        }
    }

    fn add_derive_projection_bounds(&self, generics: &mut Generics) {
        if self.flattened_forms.is_empty() || self.derive_projection_bounds.is_empty() {
            return;
        }

        let where_clause = generics.make_where_clause();
        for ty in self.flattened_forms.component_types() {
            for bound in self.derive_projection_bounds {
                where_clause
                    .predicates
                    .push(parse_quote!(<#ty as ::cheers::__internal::FormComponent>::Form: #bound));
            }
        }
    }

    fn add_form_bounds(&self, generics: &mut Generics) {
        self.add_component_bounds(generics);
        self.add_derive_projection_bounds(generics);
    }

    fn names_generics(&self, generics: Generics) -> Generics {
        let mut generics = filter_generics(generics, self.flattened_forms.component_types(), false);
        self.add_component_bounds(&mut generics);
        generics
    }

    fn form_generics<'b>(
        &self,
        generics: Generics,
        form_field_types: impl IntoIterator<Item = &'b Type>,
    ) -> Generics {
        let mut generics =
            filter_generics(generics, form_field_types, self.flattened_forms.is_empty());
        self.add_form_bounds(&mut generics);
        generics
    }

    fn form_names_impl_generics(&self, generics: Generics) -> Generics {
        let mut generics = generics;
        self.add_component_bounds(&mut generics);
        generics
    }

    fn form_component_impl_generics(&self, generics: Generics) -> Generics {
        let mut generics = generics;
        self.add_form_bounds(&mut generics);
        generics
    }
}

fn standard_derive_bound(path: &Path) -> Option<TokenStream> {
    if path.leading_colon.is_some() || path.segments.len() != 1 {
        return None;
    }

    let segment = path.segments.first()?;
    if !segment.arguments.is_empty() {
        return None;
    }

    match segment.ident.to_string().as_str() {
        "Clone" => Some(quote! { ::std::clone::Clone }),
        "Copy" => Some(quote! { ::std::marker::Copy }),
        "Debug" => Some(quote! { ::std::fmt::Debug }),
        "Default" => Some(quote! { ::std::default::Default }),
        "Eq" => Some(quote! { ::std::cmp::Eq }),
        "Hash" => Some(quote! { ::std::hash::Hash }),
        "Ord" => Some(quote! { ::std::cmp::Ord }),
        "PartialEq" => Some(quote! { ::std::cmp::PartialEq }),
        "PartialOrd" => Some(quote! { ::std::cmp::PartialOrd }),
        _ => None,
    }
}

fn form_derive_projection_bound(path: &Path) -> Option<TokenStream> {
    if path
        .segments
        .last()
        .is_some_and(|s| s.ident == "Deserialize")
    {
        return None;
    }

    standard_derive_bound(path).or_else(|| Some(quote! { #path }))
}

fn parse_form_derives(tokens: TokenStream) -> Result<FormDerives, Error> {
    let paths =
        syn::punctuated::Punctuated::<Path, Token![,]>::parse_terminated.parse2(tokens.clone())?;
    let projection_bounds = paths
        .iter()
        .filter_map(form_derive_projection_bound)
        .collect();

    Ok(FormDerives {
        tokens,
        projection_bounds,
    })
}

fn parse_form_field_args(attr: Attribute) -> Result<FormFieldArgs, Error> {
    let Meta::List(meta_list) = attr.meta else {
        return Err(Error::new_spanned(attr, "expected #[form] or #[form(...)]"));
    };

    let metas = syn::punctuated::Punctuated::<Meta, Token![,]>::parse_terminated
        .parse2(meta_list.tokens)?;
    let mut flatten = false;
    let mut attrs = Vec::new();

    for meta in metas {
        if let Meta::Path(path) = &meta
            && path.is_ident("flatten")
        {
            if flatten {
                return Err(Error::new_spanned(path, "duplicate form flatten flag"));
            }
            flatten = true;
            continue;
        }

        attrs.push(quote! { #[#meta] });
    }

    Ok(FormFieldArgs { flatten, attrs })
}

fn find_form_derives(item: &mut ItemStruct) -> Option<Result<FormDerives, Error>> {
    let (form_derive_attrs, remaining) = std::mem::take(&mut item.attrs)
        .into_iter()
        .partition(|a| a.path().is_ident("form_derive"));
    item.attrs = remaining;

    let mut form_derive_attrs = form_derive_attrs.into_iter();
    match (form_derive_attrs.next(), form_derive_attrs.next()) {
        (Some(_), Some(duplicate_attr)) => Some(Err(Error::new_spanned(
            duplicate_attr,
            "only one #[form_derive(...)] attribute is allowed",
        ))),
        (Some(attr), None) => Some(if let Meta::List(ml) = attr.meta {
            parse_form_derives(ml.tokens)
        } else {
            Err(Error::new_spanned(attr, "expected #[form_derive(...)]"))
        }),
        (None, _) => None,
    }
}

fn process_form_outer_attrs(
    vis: &Visibility,
    form_outer_attrs: Vec<Attribute>,
    form_field_decls: &mut Vec<FormFieldDecl>,
    form_name_entries: &mut Vec<FormNameEntry>,
) -> Result<(), Error> {
    for a in form_outer_attrs {
        let args: FormArgs = match a.meta {
            Meta::List(meta_list) => parse2(meta_list.tokens),
            _ => Err(Error::new_spanned(a, r#"expected #[form(...)]"#)),
        }?;

        let ty = &args.ty;
        let attrs = &args.attrs.map(|a| quote! { #[#a] });
        let ident = &args.name;
        form_field_decls.push(FormFieldDecl {
            decl: quote! {
                #attrs
                #vis #ident: #ty
            },
            ty: ty.clone(),
        });

        let field_name = LitStr::new(&ident.to_string(), ident.span());
        let ident = new_form_names_field_ident(&args.name);
        form_name_entries.push(FormNameEntry::Simple { ident, field_name });
    }

    Ok(())
}

fn get_form_inner_fields(
    fields: impl IntoIterator<Item = Field>,
) -> Result<Vec<InnerFormField>, Error> {
    let mut form_inner_fields = Vec::new();
    for mut f in fields {
        let Some(i) = f.attrs.iter().position(|a| a.path().is_ident("form")) else {
            continue;
        };
        let attr = f.attrs.swap_remove(i);
        if let Some(duplicate) = f.attrs.iter().find(|a| a.path().is_ident("form")) {
            return Err(Error::new_spanned(
                duplicate,
                "only one #[form] attribute is allowed on a field",
            ));
        }
        let args = match &attr.meta {
            Meta::Path(_) => FormFieldArgs {
                flatten: false,
                attrs: Vec::new(),
            },
            Meta::List(_) => parse_form_field_args(attr)?,
            _ => {
                return Err(Error::new_spanned(
                    &attr,
                    "expected #[form] or #[form(...)]",
                ));
            }
        };

        let Some(field_ident) = f.ident.clone() else {
            if args.flatten {
                return Err(Error::new_spanned(
                    f,
                    "#[form(flatten)] requires a named field",
                ));
            }

            form_inner_fields.push(InnerFormField {
                field_ident: Ident::new("value", f.span()),
                form_ident: Ident::new("form_value", f.span()),
                vis: f.vis,
                ty: f.ty,
                attrs: args.attrs,
                flatten: false,
            });
            continue;
        };

        let form_ident = new_form_names_field_ident(&field_ident);
        form_inner_fields.push(InnerFormField {
            field_ident,
            form_ident,
            vis: f.vis,
            ty: f.ty,
            attrs: args.attrs,
            flatten: args.flatten,
        });
    }

    Ok(form_inner_fields)
}

fn process_form_inner_fields(
    form_inner_fields: &[InnerFormField],
    form_field_decls: &mut Vec<FormFieldDecl>,
    form_name_entries: &mut Vec<FormNameEntry>,
) {
    for f in form_inner_fields {
        let ty = to_owned_type(&f.ty);
        let vis = &f.vis;
        let field_ident = &f.field_ident;
        let attrs = &f.attrs;

        if f.flatten {
            let serde_deserialize_bound = LitStr::new(
                &format!(
                    "<{} as ::cheers::__internal::FormComponent>::Form: ::cheers::__internal::serde::Deserialize<'de>",
                    quote! { #ty },
                ),
                ty.span(),
            );
            form_field_decls.push(FormFieldDecl {
                decl: quote! {
                    #(#attrs)*
                    #[serde(flatten)]
                    #[serde(bound(deserialize = #serde_deserialize_bound))]
                    #vis #field_ident: <#ty as ::cheers::__internal::FormComponent>::Form
                },
                ty: parse_quote!(<#ty as ::cheers::__internal::FormComponent>::Form),
            });
            form_name_entries.push(FormNameEntry::Flatten {
                ident: f.form_ident.clone(),
                ty: Box::new(ty),
            });
        } else {
            form_field_decls.push(FormFieldDecl {
                decl: quote! {
                    #(#attrs)*
                    #vis #field_ident: #ty
                },
                ty: ty.clone(),
            });

            let field_name_lit = LitStr::new(&field_ident.to_string(), field_ident.span());
            form_name_entries.push(FormNameEntry::Simple {
                ident: f.form_ident.clone(),
                field_name: field_name_lit,
            });
        }
    }
}

fn generate_form_names(
    vis: &Visibility,
    struct_ident: &Ident,
    generics: &Generics,
    ident_str: &str,
    form_name_entries: &[FormNameEntry],
    form_bounds: &FormBounds<'_>,
) -> FormNamesOutput {
    if form_name_entries.is_empty() {
        return FormNamesOutput {
            tokens: TokenStream::new(),
            ty: quote! { () },
            value: quote! { () },
        };
    }

    let form_names_ident = Ident::new(&format!("{}Names", ident_str), struct_ident.span());
    let names_generics = form_bounds.names_generics(generics.clone());
    let (names_decl_generics, names_ty_generics, names_where_clause) =
        names_generics.split_for_impl();
    let ty = quote! { #form_names_ident #names_ty_generics };

    let impl_generics_with_bounds = form_bounds.form_names_impl_generics(generics.clone());
    let (impl_generics, ty_generics, impl_where_clause) =
        impl_generics_with_bounds.split_for_impl();

    let entry_values = form_name_entries
        .iter()
        .map(FormNameEntry::value_tokens)
        .collect::<Vec<_>>();
    let entry_idents = form_name_entries
        .iter()
        .map(FormNameEntry::ident)
        .collect::<Vec<_>>();
    let entry_types = form_name_entries
        .iter()
        .map(FormNameEntry::ty_tokens)
        .collect::<Vec<_>>();

    let form_names_struct = quote! {
        #vis struct #form_names_ident #names_decl_generics #names_where_clause {
            #( #vis #entry_idents: #entry_types, )*
        }
    };

    let struct_impl = quote! {
        impl #impl_generics #struct_ident #ty_generics #impl_where_clause {
            const fn __cheers_form_names() -> #form_names_ident #names_ty_generics {
                #form_names_ident {
                    #( #entry_idents: #entry_values, )*
                }
            }

            #vis const fn form_names(&self) -> #form_names_ident #names_ty_generics {
                Self::__cheers_form_names()
            }
        }
    };

    FormNamesOutput {
        tokens: quote! {
            #form_names_struct

            #struct_impl
        },
        ty,
        value: quote! { Self::__cheers_form_names() },
    }
}

fn generate_form_component_impl(
    struct_ident: &Ident,
    generics: &Generics,
    form_ident: &Ident,
    form_ty_generics: TokenStream,
    form_names: &FormNamesOutput,
    form_bounds: &FormBounds<'_>,
) -> TokenStream {
    let impl_generics_with_bounds = form_bounds.form_component_impl_generics(generics.clone());
    let (impl_generics, ty_generics, where_clause) = impl_generics_with_bounds.split_for_impl();
    let form_names_ty = &form_names.ty;
    let form_names_value = &form_names.value;

    quote! {
        impl #impl_generics ::cheers::__internal::FormComponent for #struct_ident #ty_generics #where_clause {
            type Form = #form_ident #form_ty_generics;
            type FormNames = #form_names_ty;

            const __FORM_NAMES: Self::FormNames = #form_names_value;
        }
    }
}

pub(crate) fn generate_form_impl(item: &mut ItemStruct) -> Result<TokenStream, Error> {
    let form_outer_attrs = filter_outer_attrs(item, "form");
    let form_derives = find_form_derives(item).transpose()?;
    let form_derive_tokens = form_derives.as_ref().map(|d| &d.tokens);

    let mut ident_str = item.ident.to_string();
    let vis = &item.vis;
    let form_ident = {
        ident_str.push_str("Form");
        Ident::new(&ident_str, item.ident.span())
    };

    let mut form_name_entries: Vec<FormNameEntry> = Vec::new();
    let mut form_field_decls = Vec::new();

    process_form_outer_attrs(
        vis,
        form_outer_attrs,
        &mut form_field_decls,
        &mut form_name_entries,
    )?;

    let form_inner_fields = get_form_inner_fields(item.fields.iter().cloned())?;
    process_form_inner_fields(
        &form_inner_fields,
        &mut form_field_decls,
        &mut form_name_entries,
    );

    if form_inner_fields.is_empty() && form_name_entries.is_empty() && form_derives.is_none() {
        return Ok(TokenStream::new());
    }

    let flattened_forms = FlattenedForms::from_entries(&form_name_entries);
    let form_bounds = FormBounds::new(&flattened_forms, form_derives.as_ref());
    let form_names = generate_form_names(
        vis,
        &item.ident,
        &item.generics,
        &ident_str,
        &form_name_entries,
        &form_bounds,
    );

    let form_field_types = form_field_decls
        .iter()
        .map(|field| &field.ty)
        .collect::<Vec<_>>();
    let filtered_generics = form_bounds.form_generics(item.generics.clone(), form_field_types);
    let (form_decl_generics, form_ty_generics, form_where_clause) =
        filtered_generics.split_for_impl();
    let form_ty_generics_tokens = quote! { #form_ty_generics };
    let form_field_decl_tokens = form_field_decls.iter().map(|field| &field.decl);

    let form_struct = {
        quote! {
            #[derive(::cheers::__internal::serde::Deserialize, #form_derive_tokens)]
            #[serde(crate = "::cheers::__internal::serde")]
            #vis struct #form_ident #form_decl_generics #form_where_clause {
                #(#form_field_decl_tokens,)*
            }
        }
    };

    let form_component_impl = generate_form_component_impl(
        &item.ident,
        &item.generics,
        &form_ident,
        form_ty_generics_tokens,
        &form_names,
        &form_bounds,
    );
    let form_names_tokens = &form_names.tokens;

    Ok(quote! {
        #form_names_tokens

        #form_struct

        #form_component_impl
    })
}
