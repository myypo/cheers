use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, Error, Expr, Field, Fields, Generics, Ident, ItemStruct, Meta, Type,
    ext::IdentExt,
    parse::{Parse, ParseStream},
};

use crate::shared::{filter_generics, to_pascal_case};

struct PropArgs {
    default: Expr,
}

impl Parse for PropArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let kind: Ident = input.parse()?;
        if kind != "default" {
            return Err(Error::new_spanned(kind, "expected #[prop(default(...))]"));
        }

        let content;
        syn::parenthesized!(content in input);
        let default = content.parse()?;

        if !input.is_empty() {
            return Err(Error::new_spanned(
                input.parse::<TokenStream>()?,
                "expected #[prop(default(...))]",
            ));
        }

        Ok(Self { default })
    }
}

struct FieldSpec {
    ident: Ident,
    ty: Type,
}

struct RequiredFieldSpec {
    field: FieldSpec,
    sort_key: String,
    wrapper_ident: Ident,
    method_ident: Ident,
}

struct DefaultedFieldSpec {
    field: FieldSpec,
    default: Expr,
    docs: Vec<Attribute>,
}

struct PropFields {
    required: Vec<RequiredFieldSpec>,
    defaulted: Vec<DefaultedFieldSpec>,
    children: Option<FieldSpec>,
}

fn helper_prop_method_ident(ident: &Ident) -> syn::Result<Ident> {
    syn::parse_str(&format!("__cheers_prop_{}", ident.unraw()))
}

fn helper_prop_type_ident(struct_ident: &Ident, field_ident: &Ident) -> Ident {
    Ident::new(
        &format!(
            "__Cheers{}Prop{}",
            struct_ident,
            to_pascal_case(&field_ident.unraw().to_string())
        ),
        field_ident.span(),
    )
}

fn helper_required_ident(struct_ident: &Ident) -> Ident {
    Ident::new(
        &format!("__Cheers{}PropsRequired", struct_ident),
        struct_ident.span(),
    )
}

fn helper_builder_ident(struct_ident: &Ident) -> Ident {
    Ident::new(
        &format!("{}PropsBuilder", struct_ident),
        struct_ident.span(),
    )
}

fn marker_ty(generics: &Generics) -> TokenStream {
    let marker_parts = generics
        .params
        .iter()
        .map(|param| match param {
            syn::GenericParam::Lifetime(lifetime) => {
                let lifetime = &lifetime.lifetime;
                quote! { &#lifetime () }
            }
            syn::GenericParam::Type(ty) => {
                let ident = &ty.ident;
                quote! { #ident }
            }
            syn::GenericParam::Const(const_param) => {
                let ident = &const_param.ident;
                quote! { [(); #ident] }
            }
        })
        .collect::<Vec<_>>();

    if marker_parts.is_empty() {
        quote! { () }
    } else {
        quote! { ( #(#marker_parts,)* ) }
    }
}

fn decl_generics(generics: &Generics) -> TokenStream {
    let params = &generics.params;

    if params.is_empty() {
        TokenStream::new()
    } else {
        quote! { <#params> }
    }
}

fn take_prop_default(field: &mut Field) -> Result<Option<Expr>, Error> {
    let prop_indices = field
        .attrs
        .iter()
        .enumerate()
        .filter_map(|(idx, attr)| attr.path().is_ident("prop").then_some(idx))
        .collect::<Vec<_>>();

    let Some(first_idx) = prop_indices.first().copied() else {
        return Ok(None);
    };

    if prop_indices.len() > 1 {
        return Err(Error::new_spanned(
            &field.ident,
            "only one #[prop(...)] attribute is allowed on a field",
        ));
    }

    let attr = field.attrs.swap_remove(first_idx);
    match attr.meta {
        Meta::List(meta_list) => Ok(Some(syn::parse2::<PropArgs>(meta_list.tokens)?.default)),
        _ => Err(Error::new_spanned(attr, "expected #[prop(default(...))]")),
    }
}

fn collect_prop_fields(item: &mut ItemStruct, struct_ident: &Ident) -> Result<PropFields, Error> {
    let mut required = Vec::<RequiredFieldSpec>::new();
    let mut defaulted = Vec::<DefaultedFieldSpec>::new();
    let mut children = None::<FieldSpec>;

    for field in item.fields.iter_mut() {
        let Some(field_ident) = field.ident.clone() else {
            continue;
        };

        let prop_default = take_prop_default(field)?;
        let field_ty = field.ty.clone();

        if field_ident == "children" {
            if prop_default.is_some() {
                return Err(Error::new_spanned(
                    field,
                    "the `children` field is special and does not support #[prop(...)]",
                ));
            }

            children = Some(FieldSpec {
                ident: field_ident,
                ty: field_ty,
            });
            continue;
        }

        let field_spec = FieldSpec {
            ident: field_ident.clone(),
            ty: field_ty,
        };

        match prop_default {
            Some(default) => {
                let docs = field
                    .attrs
                    .iter()
                    .filter(|attr| attr.path().is_ident("doc"))
                    .cloned()
                    .collect();

                defaulted.push(DefaultedFieldSpec {
                    field: field_spec,
                    default,
                    docs,
                });
            }
            None => required.push(RequiredFieldSpec {
                sort_key: field_ident.unraw().to_string(),
                wrapper_ident: helper_prop_type_ident(struct_ident, &field_ident),
                method_ident: helper_prop_method_ident(&field_ident)?,
                field: field_spec,
            }),
        }
    }

    Ok(PropFields {
        required,
        defaulted,
        children,
    })
}

fn sorted_required_fields(required_fields: &[RequiredFieldSpec]) -> Vec<&RequiredFieldSpec> {
    let mut required_fields = required_fields.iter().collect::<Vec<_>>();
    required_fields.sort_by(|lhs, rhs| lhs.sort_key.cmp(&rhs.sort_key));
    required_fields
}

fn builder_fields<'a>(
    required_fields: &'a [RequiredFieldSpec],
    defaulted_fields: &'a [DefaultedFieldSpec],
) -> impl Iterator<Item = &'a FieldSpec> + 'a {
    required_fields
        .iter()
        .map(|field| &field.field)
        .chain(defaulted_fields.iter().map(|field| &field.field))
}

pub(crate) fn generate_prop_impl(item: &mut ItemStruct) -> Result<TokenStream, Error> {
    match &item.fields {
        Fields::Unnamed(fields) if !fields.unnamed.is_empty() => {
            if fields
                .unnamed
                .iter()
                .any(|field| field.attrs.iter().any(|attr| attr.path().is_ident("prop")))
            {
                return Err(Error::new_spanned(
                    &item.fields,
                    "#[prop(default(...))] is only supported on named fields or unit structs",
                ));
            }

            return Ok(TokenStream::new());
        }
        _ => {}
    }

    let vis = item.vis.clone();
    let struct_ident = item.ident.clone();
    let builder_ident = helper_builder_ident(&struct_ident);
    let required_ident = helper_required_ident(&struct_ident);
    let generics = item.generics.clone();
    let marker_ty = marker_ty(&generics);
    let builder_generic_params = decl_generics(&generics);
    let builder_where_clause = &generics.where_clause;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let PropFields {
        required: required_fields,
        defaulted: defaulted_fields,
        children: children_field,
    } = collect_prop_fields(item, &struct_ident)?;

    let required_generics = filter_generics(
        generics.clone(),
        required_fields.iter().map(|field| &field.field.ty),
        false,
    );
    let required_generic_params = decl_generics(&required_generics);
    let required_where_clause = &required_generics.where_clause;
    let (_, required_ty_generics, _) = required_generics.split_for_impl();

    let required_fields_in_signature_order = sorted_required_fields(&required_fields);

    let wrapper_structs = required_fields.iter().map(|field| {
        let wrapper_ident = &field.wrapper_ident;
        let ty = &field.field.ty;

        quote! {
            #[doc(hidden)]
            #vis struct #wrapper_ident #builder_generic_params #builder_where_clause {
                value: #ty,
                __cheers_marker: ::core::marker::PhantomData<#marker_ty>,
            }
        }
    });

    let required_field_decls = required_fields.iter().map(|field| {
        let ident = &field.field.ident;
        let ty = &field.field.ty;

        quote! {
            #vis #ident: #ty
        }
    });

    let builder_field_decls = builder_fields(&required_fields, &defaulted_fields).map(|field| {
        let ident = &field.ident;
        let ty = &field.ty;
        quote! { #ident: #ty }
    });

    let constructor_fields_from_required = required_fields
        .iter()
        .map(|field| {
            let ident = &field.field.ident;
            quote! { #ident: __cheers_required.#ident }
        })
        .chain(defaulted_fields.iter().map(|field| {
            let ident = &field.field.ident;
            let default = &field.default;
            quote! { #ident: #default }
        }));

    let required_param_entries = required_fields_in_signature_order.iter().map(|field| {
        let wrapper_ident = &field.wrapper_ident;
        let ident = &field.field.ident;
        quote! { #ident: #wrapper_ident #ty_generics }
    });

    let required_fields_from_wrappers = required_fields_in_signature_order.iter().map(|field| {
        let ident = &field.field.ident;
        quote! { #ident: #ident.value }
    });

    let prop_constructor_methods = required_fields.iter().map(|field| {
        let method_ident = &field.method_ident;
        let wrapper_ident = &field.wrapper_ident;
        let ident = &field.field.ident;
        let ty = &field.field.ty;

        quote! {
            #[doc(hidden)]
            #[must_use]
            #vis fn #method_ident(#ident: #ty) -> #wrapper_ident #ty_generics {
                #wrapper_ident {
                    value: #ident,
                    __cheers_marker: ::core::marker::PhantomData,
                }
            }
        }
    });

    let placeholder_required_fields = required_fields.iter().map(|field| {
        let ident = &field.field.ident;
        quote! {
            #ident: ::cheers::__internal::__component_placeholder()
        }
    });

    let setter_methods = defaulted_fields.iter().map(|field| {
        let ident = &field.field.ident;
        let ty = &field.field.ty;
        let docs = &field.docs;
        quote! {
            #(#docs)*
            #[must_use]
            #vis fn #ident(mut self, #ident: #ty) -> Self {
                self.#ident = #ident;
                self
            }
        }
    });

    let build_named_field_entries = builder_fields(&required_fields, &defaulted_fields)
        .map(|field| {
            let ident = &field.ident;
            quote! { #ident: self.#ident }
        })
        .collect::<Vec<_>>();

    let build_method = match (&item.fields, &children_field) {
        (Fields::Unit, None) => quote! {
            #vis fn __cheers_build(self) -> #struct_ident #ty_generics {
                #struct_ident
            }
        },
        (Fields::Named(_), None) => quote! {
            #vis fn __cheers_build(self) -> #struct_ident #ty_generics {
                #struct_ident {
                    #(#build_named_field_entries,)*
                }
            }
        },
        (Fields::Named(_), Some(children_field)) => {
            let children_ident = &children_field.ident;
            let children_ty = &children_field.ty;
            quote! {
                #vis fn __cheers_build_with_children(self, #children_ident: #children_ty) -> #struct_ident #ty_generics {
                    #struct_ident {
                        #(#build_named_field_entries,)*
                        #children_ident,
                    }
                }
            }
        }
        (Fields::Unit, Some(_)) | (Fields::Unnamed(_), _) => TokenStream::new(),
    };

    Ok(quote! {
        #(#wrapper_structs)*

        #[doc(hidden)]
        #vis struct #required_ident #required_generic_params #required_where_clause {
            #(#required_field_decls,)*
        }

        #[doc(hidden)]
        #vis struct #builder_ident #builder_generic_params #builder_where_clause {
            #(#builder_field_decls,)*
            __cheers_marker: ::core::marker::PhantomData<#marker_ty>,
        }

        impl #impl_generics #struct_ident #ty_generics #where_clause {
            #(#prop_constructor_methods)*

            #[doc(hidden)]
            #[must_use]
            #vis fn __cheers_required() -> #required_ident #required_ty_generics {
                #required_ident {
                    #(#placeholder_required_fields,)*
                }
            }

            #[doc(hidden)]
            #[must_use]
            #vis fn __cheers_props_from_required(__cheers_required: #required_ident #required_ty_generics) -> #builder_ident #ty_generics {
                #builder_ident {
                    #(#constructor_fields_from_required,)*
                    __cheers_marker: ::core::marker::PhantomData,
                }
            }

            #[doc(hidden)]
            #[must_use]
            #[allow(clippy::too_many_arguments)]
            #vis fn __cheers_props(#(#required_param_entries),*) -> #builder_ident #ty_generics {
                Self::__cheers_props_from_required(#required_ident {
                    #(#required_fields_from_wrappers,)*
                })
            }
        }

        impl #impl_generics #builder_ident #ty_generics #where_clause {
            #(#setter_methods)*
            #build_method
        }
    })
}
