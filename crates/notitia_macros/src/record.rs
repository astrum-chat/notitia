use convert_case::Casing;
use proc_macro::TokenStream;

use proc_macro2::Span;
use quote::quote;
use syn::{Fields, GenericArgument, Ident, ItemStruct, PathArguments, Type, parse_macro_input};

use crate::utils::get_attr_idx;

/// If `ty` is `Option<T>`, returns `Some(T)`. Otherwise returns `None`.
fn extract_option_inner(ty: &Type) -> Option<&Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };

    let segment = type_path.path.segments.last()?;

    if segment.ident != "Option" {
        return None;
    }

    let PathArguments::AngleBracketed(ref args) = segment.arguments else {
        return None;
    };

    if let Some(GenericArgument::Type(inner)) = args.args.first() {
        Some(inner)
    } else {
        None
    }
}

pub fn impl_record(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    let vis = &input.vis;
    let generics = &input.generics;

    let Fields::Named(ref fields_named) = input.fields else {
        panic!("Record attribute only works on structs with named fields");
    };

    let module_name = Ident::new(&format!("notitia_{}", name), Span::call_site());

    let field_datatype_kinds = fields_named.named.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap().to_string();
        let field_ty = &field.ty;
        let field_attrs = field.attrs.as_slice();

        if get_attr_idx(field_attrs, "db", "primary_key").is_some() {
            quote! {
                (#field_name, <notitia::PrimaryKey<#field_ty> as notitia::AsDatatypeKind>::as_datatype_kind())
            }
        } else if get_attr_idx(field_attrs, "db", "unique").is_some() {
            quote! {
                (#field_name, <notitia::Unique<#field_ty> as notitia::AsDatatypeKind>::as_datatype_kind())
            }
        } else {
            quote! {
                (#field_name, <#field_ty as notitia::AsDatatypeKind>::as_datatype_kind())
            }
        }
    });

    let field_into_datatypes = fields_named.named.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let field_name_string = field_name.to_string();

        quote! {
            (#field_name_string, self.#field_name.into())
        }
    });

    let constructor_fields = fields_named.named.iter().map(|field| {
        let field_name = &field.ident;
        let field_vis = &field.vis;
        let field_ty = &field.ty;

        let mut field_attrs = field.attrs.iter().collect::<Vec<_>>();

        if let Some(attr_idx) = get_attr_idx(field_attrs.as_slice(), "db", "primary_key") {
            field_attrs.remove(attr_idx);

            quote! {
                #(#field_attrs)*
                #field_vis #field_name: notitia::PrimaryKey<#field_ty>
            }
        } else if let Some(attr_idx) = get_attr_idx(field_attrs.as_slice(), "db", "unique") {
            field_attrs.remove(attr_idx);

            quote! {
                #(#field_attrs)*
                #field_vis #field_name: notitia::Unique<#field_ty>
            }
        } else {
            quote! {
                #(#field_attrs)*
                #field_vis #field_name: #field_ty
            }
        }
    });

    let table_field_enum_name = Ident::new(&format!("{}{}", name, "Field"), Span::call_site());

    let enum_fields = fields_named.named.iter().filter_map(|field| {
        let Some(field_name) = field.ident.as_ref() else {
            return None;
        };

        Some(Ident::new(
            &field_name.to_string().to_case(convert_case::Case::Pascal),
            Span::call_site(),
        ))
    });

    let enum_field_consts = fields_named.named.iter().filter_map(|field| {
        let Some(field_name) = field.ident.as_ref() else {
            return None;
        };

        let field_ty = &field.ty;

        let pascal_field_name = Ident::new(
            &field_name.to_string().to_case(convert_case::Case::Pascal),
            Span::call_site(),
        );

        let upper_snake_field_name = Ident::new(
            &field_name
                .to_string()
                .to_case(convert_case::Case::UpperSnake),
            Span::call_site(),
        );

        Some(quote! {
            pub const #upper_snake_field_name: notitia::StrongFieldKind<#module_name::#table_field_enum_name, #field_ty> =
                notitia::StrongFieldKind::new(#module_name::#table_field_enum_name::#pascal_field_name)
        })
    });

    let enum_to_names = fields_named.named.iter().filter_map(|field| {
        let Some(field_name) = field.ident.as_ref() else {
            return None;
        };

        let field_name_string = field_name.to_string();

        let pascal_field_name = Ident::new(
            &field_name_string.to_case(convert_case::Case::Pascal),
            Span::call_site(),
        );

        Some(quote! {
            Self::#pascal_field_name => #field_name_string
        })
    });

    // --- Builder generation ---

    let builder_name = Ident::new(&format!("{}Builder", name), Span::call_site());

    struct BuilderFieldInfo {
        field_name: Ident,
        generic_ident: Ident,
        raw_ty: proc_macro2::TokenStream,
        is_primary_key: bool,
        is_unique: bool,
        is_optional: bool,
        option_inner_ty: Option<proc_macro2::TokenStream>,
    }

    let builder_fields: Vec<BuilderFieldInfo> = fields_named
        .named
        .iter()
        .filter_map(|field| {
            let field_name = field.ident.as_ref()?.clone();
            let field_ty = &field.ty;
            let field_attrs = field.attrs.as_slice();

            let generic_ident = Ident::new(&format!("T_{}", field_name), Span::call_site());

            let is_primary_key = get_attr_idx(field_attrs, "db", "primary_key").is_some();
            let is_unique = get_attr_idx(field_attrs, "db", "unique").is_some();

            let raw_ty = quote! { #field_ty };

            let option_inner = extract_option_inner(field_ty);
            let is_optional = option_inner.is_some();
            let option_inner_ty = option_inner.map(|inner| quote! { #inner });

            Some(BuilderFieldInfo {
                field_name,
                generic_ident,
                raw_ty,
                is_primary_key,
                is_unique,
                is_optional,
                option_inner_ty,
            })
        })
        .collect();

    // Builder struct generic params with defaults (only for non-optional fields)
    let builder_generic_params_with_defaults: Vec<_> = builder_fields
        .iter()
        .filter(|f| !f.is_optional)
        .map(|f| {
            let gi = &f.generic_ident;
            quote! { #gi = notitia::UnsetField }
        })
        .collect();

    // Builder struct fields: optional fields use Option<FieldExpr>, others use generics
    let builder_struct_fields = builder_fields.iter().map(|f| {
        let fname = &f.field_name;
        if f.is_optional {
            quote! { #fname: Option<notitia::FieldExpr> }
        } else {
            let gi = &f.generic_ident;
            quote! { #fname: #gi }
        }
    });

    let builder_generic_idents: Vec<_> = builder_fields
        .iter()
        .filter(|f| !f.is_optional)
        .map(|f| &f.generic_ident)
        .collect();

    // Setter methods accept `impl Into<FieldExpr>`, transition generic to FieldExpr.
    let builder_setter_methods = builder_fields.iter().enumerate().map(|(idx, f)| {
        let fname = &f.field_name;

        let return_generics: Vec<_> = builder_fields
            .iter()
            .enumerate()
            .filter(|(_, fj)| !fj.is_optional)
            .map(|(j, fj)| {
                if j == idx && !f.is_optional {
                    quote! { notitia::FieldExpr }
                } else {
                    let gi = &fj.generic_ident;
                    quote! { #gi }
                }
            })
            .collect();

        let struct_init_fields = builder_fields.iter().enumerate().map(|(j, fj)| {
            let fj_name = &fj.field_name;
            if j == idx {
                if f.is_optional {
                    quote! { #fj_name: Some(value.into()) }
                } else {
                    quote! { #fj_name: value.into() }
                }
            } else {
                quote! { #fj_name: self.#fj_name }
            }
        });

        quote! {
            pub fn #fname(self, value: impl Into<notitia::FieldExpr>) -> #builder_name<#(#return_generics),*> {
                #builder_name {
                    #(#struct_init_fields),*
                }
            }
        }
    });

    // BuiltRecord: all non-optional fields are FieldExpr
    let builder_concrete_types: Vec<_> = builder_fields
        .iter()
        .filter(|f| !f.is_optional)
        .map(|_| quote! { notitia::FieldExpr })
        .collect();

    // finish() extracts Literal values via TryFrom<Datatype>
    let finish_fields = builder_fields.iter().map(|f| {
        let fname = &f.field_name;
        let raw_ty = &f.raw_ty;
        if f.is_primary_key {
            quote! {
                #fname: {
                    let notitia::FieldExpr::Literal(val) = self.#fname else {
                        panic!("BuiltRecord::finish only supports literal field values");
                    };
                    notitia::PrimaryKey::new(<#raw_ty as TryFrom<notitia::Datatype>>::try_from(val).unwrap())
                }
            }
        } else if f.is_unique {
            quote! {
                #fname: {
                    let notitia::FieldExpr::Literal(val) = self.#fname else {
                        panic!("BuiltRecord::finish only supports literal field values");
                    };
                    notitia::Unique::new(<#raw_ty as TryFrom<notitia::Datatype>>::try_from(val).unwrap())
                }
            }
        } else if f.is_optional {
            let inner_ty = f.option_inner_ty.as_ref().unwrap();
            quote! {
                #fname: self.#fname.and_then(|expr| {
                    let notitia::FieldExpr::Literal(val) = expr else {
                        panic!("BuiltRecord::finish only supports literal field values");
                    };
                    match val {
                        notitia::Datatype::Null => None,
                        other => Some(<#inner_ty as TryFrom<notitia::Datatype>>::try_from(other).unwrap()),
                    }
                })
            }
        } else {
            quote! {
                #fname: {
                    let notitia::FieldExpr::Literal(val) = self.#fname else {
                        panic!("BuiltRecord::finish only supports literal field values");
                    };
                    <#raw_ty as TryFrom<notitia::Datatype>>::try_from(val).unwrap()
                }
            }
        }
    });

    // PartialRecord impl
    let partial_record_generic_params: Vec<_> = builder_fields
        .iter()
        .filter(|f| !f.is_optional)
        .map(|f| {
            let gi = &f.generic_ident;
            quote! { #gi: notitia::MaybeSetExpr }
        })
        .collect();

    let partial_record_generic_args: Vec<_> = builder_fields
        .iter()
        .filter(|f| !f.is_optional)
        .map(|f| {
            let gi = &f.generic_ident;
            quote! { #gi }
        })
        .collect();

    let partial_record_field_pushes: Vec<_> = builder_fields
        .iter()
        .map(|f| {
            let fname = &f.field_name;
            let fname_str = fname.to_string();
            if f.is_optional {
                quote! {
                    if let Some(expr) = self.#fname {
                        fields.push((#fname_str, expr));
                    }
                }
            } else {
                quote! {
                    if let Some(expr) = notitia::MaybeSetExpr::into_field_expr(self.#fname) {
                        fields.push((#fname_str, expr));
                    }
                }
            }
        })
        .collect();

    let build_init_fields = builder_fields.iter().map(|f| {
        let fname = &f.field_name;
        if f.is_optional {
            quote! { #fname: None }
        } else {
            quote! { #fname: notitia::UnsetField }
        }
    });

    let expanded = quote! {
        #[derive(Clone)]
        #vis struct #name #generics {
            #(#constructor_fields),*
        }

        impl #generics #name #generics {
            #(#enum_field_consts;)*
        }

        impl #generics notitia::Record for #name #generics {
            type FieldKind = #module_name::#table_field_enum_name;

            const _FIELDS: std::sync::LazyLock<Box<[(&'static str, notitia::DatatypeKind)]>> =
                std::sync::LazyLock::new(|| Box::new([#(#field_datatype_kinds),*]));

            fn into_datatypes(self) -> Vec<(&'static str, notitia::Datatype)> {
                vec![#(#field_into_datatypes),*]
            }
        }

        #[doc(hidden)]
        mod #module_name {
            #[derive(Clone, Copy, Debug)]
            #[doc(hidden)]
            pub enum #table_field_enum_name {
                #(#enum_fields),*
            }

            impl notitia::FieldKind for #table_field_enum_name {
                fn name(&self) -> &'static str {
                    match self {
                        #(#enum_to_names),*
                    }
                }
            }
        }

        #[derive(Clone)]
        #vis struct #builder_name<#(#builder_generic_params_with_defaults),*> {
            #(#builder_struct_fields),*
        }

        impl<#(#builder_generic_idents),*> #builder_name<#(#builder_generic_idents),*> {
            #(#builder_setter_methods)*
        }

        impl<#(#partial_record_generic_params),*> notitia::PartialRecord for #builder_name<#(#partial_record_generic_args),*> {
            type FieldKind = #module_name::#table_field_enum_name;

            fn into_set_fields(self) -> Vec<(&'static str, notitia::FieldExpr)> {
                let mut fields = Vec::new();
                #(#partial_record_field_pushes)*
                fields
            }
        }

        impl notitia::BuiltRecord for #builder_name<#(#builder_concrete_types),*> {
            type Record = #name;

            fn finish(self) -> #name {
                #name {
                    #(#finish_fields),*
                }
            }
        }

        impl #generics #name #generics {
            pub fn build() -> #builder_name {
                #builder_name {
                    #(#build_init_fields),*
                }
            }
        }
    };

    TokenStream::from(expanded)
}
