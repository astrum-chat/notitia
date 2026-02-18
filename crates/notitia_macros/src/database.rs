use std::{
    borrow::Borrow,
    collections::HashSet,
    hash::{Hash, Hasher},
};

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    Attribute, Error, Fields, GenericArgument, Ident, ItemStruct, PathArguments, Result, Token,
    Type, TypePath, parse::ParseBuffer, parse_macro_input,
};

pub fn impl_database(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let database_name = &input.ident;
    let vis = &input.vis;
    let generics = &input.generics;

    let Fields::Named(fields_named) = &input.fields else {
        panic!("Database attribute only works on structs with named fields");
    };

    let module_name = Ident::new(&format!("notitia_{}", database_name), Span::call_site());

    let mut used_tables: HashSet<RecordTyWithName> = HashSet::new();

    let mut fields = vec![];
    let mut field_initializers = vec![];
    let mut foreign_relationships = vec![];
    let mut tables_method_items = vec![];
    let mut embedding_table_entries: Vec<(String, &Type)> = vec![];
    let _ = &embedding_table_entries; // suppress unused warning when embeddings feature is off

    let mut table_kinds = vec![];
    let mut table_kinds_consts = vec![];
    let mut table_kinds_enum_to_str = vec![];
    let table_kinds_enum_name = Ident::new(
        &format!("{}TableKind", database_name.to_string()),
        Span::call_site(),
    );

    for field in fields_named.named.iter() {
        let mut table_field_attrs = field.attrs.iter().collect::<Vec<_>>();
        let table_field_name = &field.ident;
        let table_field_vis = &field.vis;
        let table_field_ty = &field.ty;

        let record_ty = match parse_table_type(table_field_ty) {
            Some(record_ty) => record_ty,

            None => {
                panic!("Fields inside of the database can only have the `Table<Record>` type.")
            }
        };

        if let Some(table_field_name) = table_field_name {
            let table_field_name_string = table_field_name.to_string();

            let upper_snake_table_field_name_string = Ident::new(
                &table_field_name_string.to_case(Case::UpperSnake),
                Span::call_site(),
            );
            let pascal_table_field_name_string = Ident::new(
                &table_field_name_string.to_case(Case::Pascal),
                Span::call_site(),
            );
            table_kinds.push(quote! { #pascal_table_field_name_string });
            table_kinds_consts.push(quote! {
                pub const #upper_snake_table_field_name_string: notitia::StrongTableKind<#database_name, Table<#record_ty, #database_name>> =
                    notitia::StrongTableKind::new(#module_name::#table_kinds_enum_name::#pascal_table_field_name_string)
            });
            table_kinds_enum_to_str.push(quote! {
                Self::#pascal_table_field_name_string => #table_field_name_string
            });

            tables_method_items.push(quote! {
                (#table_field_name_string, self.#table_field_name.rows_self())
            });

            field_initializers.push(quote! {
                #table_field_name: Table::new(#table_field_name_string)
            });

            let mut inner_foreign_relationships = Vec::new();
            for relationship in
                get_foreign_key_attrs(table_field_attrs.as_slice(), "db", "foreign_key")
                    .rev()
                    .collect::<Vec<_>>()
            {
                let (
                    foreign_key_idx,
                    local_field,
                    foreign_table,
                    foreign_field,
                    on_delete,
                    on_update,
                ) = match relationship {
                    Ok(relationship) => relationship,
                    Err(err) => return err.to_compile_error().into(),
                };

                table_field_attrs.remove(foreign_key_idx);

                let local_field_str = local_field.to_string();
                let foreign_table_str = foreign_table.to_string();
                let foreign_field_str = foreign_field.to_string();

                if table_field_name_string == foreign_table_str {
                    let start = foreign_table.span();
                    let end = foreign_field.span();

                    let span = start.join(end).unwrap_or(end);

                    return syn::Error::new(
                        span,
                        &format!(
                            "The foreign key '{}.{}' cannot reference its own table '{}'.",
                            foreign_table_str, foreign_field_str, table_field_name_string
                        ),
                    )
                    .to_compile_error()
                    .into();
                }

                inner_foreign_relationships.push(quote! {
                    #local_field_str => {
                        #[allow(deprecated)]
                        fn _check_fields(db: #database_name) {
                            /// Throws error if the local field doesn't exist.
                            let _ = db.#table_field_name.test_type().#local_field;

                            /// Throws error if the foreign field doesn't exist.
                            let _ = db.#foreign_table.test_type().#foreign_field;
                        }

                        notitia::ForeignRelationship::new(#foreign_table_str, #foreign_field_str, #on_delete, #on_update)
                    }
                })
            }

            if inner_foreign_relationships.len() != 0 {
                foreign_relationships.push(quote! {
                    #table_field_name_string => {
                        use notitia::phf;

                        phf::phf_map! {
                            #(#inner_foreign_relationships),*
                        }
                    }
                });
            }

            embedding_table_entries.push((table_field_name_string.clone(), record_ty));

            let record_ty_with_name = RecordTyWithName::new(record_ty, table_field_name_string);

            if used_tables.contains(&record_ty_with_name) {
                return syn::Error::new_spanned(record_ty, "You can only use the same record type once in a database to prevent ambiguities with types.")
                    .to_compile_error()
                    .into();
            }
            used_tables.insert(record_ty_with_name);

            fields.push(quote! {
                #(#table_field_attrs)*
                #table_field_vis #table_field_name: notitia::Table<#record_ty, #database_name>
            });
        }
    }

    let fields_of_database = used_tables
        .iter()
        .filter_map(|RecordTyWithName { ty, name }| {
            let record_name = type_name(ty)?;

            let record_mod = Ident::new(&format!("notitia_{}", record_name), Span::call_site());
            let record_field_name = Ident::new(&format!("{}Field", record_name), Span::call_site());

            Some(quote! {
                impl notitia::FieldKindOfDatabase<#database_name> for #record_mod::#record_field_name {
                    fn table_name() -> &'static str {
                        #name
                    }
                }
            })
        });

    // Generate embedded_tables() override and embedder-aware connect, gated on embeddings feature.
    #[cfg(feature = "embeddings")]
    let embedded_tables_override = {
        let items = embedding_table_entries
            .iter()
            .map(|(table_name, record_ty)| {
                quote! {
                    if !#record_ty::_EMBEDDED_FIELDS.is_empty() {
                        tables.push(notitia::EmbeddedTableDef {
                            table_name: #table_name,
                            embedded_fields: #record_ty::_EMBEDDED_FIELDS,
                            pk_field: #record_ty::_PK_FIELD,
                        });
                    }
                }
            });
        quote! {
            fn embedded_tables(&self) -> Vec<notitia::EmbeddedTableDef> {
                let mut tables = Vec::new();
                #(#items)*
                tables
            }
        }
    };

    #[cfg(not(feature = "embeddings"))]
    let embedded_tables_override = quote! {};

    let expanded = quote! {
        #vis struct #database_name #generics {
            #(#fields),*
        }

        impl notitia::Database for #database_name {
            type TableKind = #module_name::#table_kinds_enum_name;

            const _FOREIGN_RELATIONSHIPS: notitia::phf::Map<&'static str, notitia::phf::Map<&'static str, notitia::ForeignRelationship>> = {
                use notitia::phf;

                phf::phf_map! {
                    #(#foreign_relationships),*
                }
            };

            fn new() -> Self {
                Self {
                    #(#field_initializers),*
                }
            }

            #[allow(deprecated)]
            fn tables(&self) -> impl Iterator<Item = (&'static str, notitia::FieldsDef)> {
                [#(#tables_method_items),*].into_iter()
            }

            #embedded_tables_override
        }

        impl #generics #database_name #generics {
            #(#table_kinds_consts;)*
        }

        #(#fields_of_database)*

        #[doc(hidden)]
        mod #module_name {
            #[derive(Debug)]
            #[doc(hidden)]
            pub enum #table_kinds_enum_name {
                #(#table_kinds),*
            }

            impl notitia::TableKind for #table_kinds_enum_name {
                fn name(&self) -> &'static str {
                    match self {
                        #(#table_kinds_enum_to_str),*
                    }
                }
            }
        }
    };

    TokenStream::from(expanded)
}

pub fn get_foreign_key_attrs<T>(
    attrs: &[T],
    ident: &str,
    name: &str,
) -> impl DoubleEndedIterator<
    Item = Result<(
        usize,
        Ident,
        Ident,
        Ident,
        proc_macro2::TokenStream,
        proc_macro2::TokenStream,
    )>,
>
where
    T: Borrow<Attribute>,
{
    attrs.iter().enumerate().filter_map(|(idx, attr)| {
        let attr = attr.borrow();

        if !attr.path().is_ident(ident) {
            return None;
        }

        let mut found: Option<
            Result<(
                Ident,
                Ident,
                Ident,
                proc_macro2::TokenStream,
                proc_macro2::TokenStream,
            )>,
        > = None;

        let result = attr.parse_nested_meta(|meta| {
            if !meta.path.is_ident(name) {
                return Ok(());
            }

            let content;
            syn::parenthesized!(content in meta.input);

            let local_field: Ident = content.parse()?;
            content.parse::<Token![,]>()?;

            let foreign_table: Ident = content.parse()?;
            content.parse::<Token![.]>()?;

            let foreign_field: Ident = content.parse()?;

            let (on_delete, on_update) = parse_on_actions(&content)?;

            found = Some(Ok((
                local_field,
                foreign_table,
                foreign_field,
                on_delete,
                on_update,
            )));
            Ok(())
        });

        if let Err(err) = result {
            return Some(Err(err));
        }

        found.map(|res| {
            res.map(
                |(local_field, foreign_table, foreign_field, on_delete, on_update)| {
                    (
                        idx,
                        local_field,
                        foreign_table,
                        foreign_field,
                        on_delete,
                        on_update,
                    )
                },
            )
        })
    })
}

fn parse_on_actions(
    content: &ParseBuffer<'_>,
) -> Result<(proc_macro2::TokenStream, proc_macro2::TokenStream)> {
    let mut on_delete = None;
    let mut on_update = None;

    while content.parse::<Token![,]>().is_ok() {
        let key: Ident = content.parse()?;
        content.parse::<Token![=]>()?;
        let variant: Ident = content.parse()?;

        if key == "on_delete" {
            if on_delete.is_some() {
                return Err(Error::new_spanned(key, "duplicate `on_delete`"));
            }
            on_delete = Some(quote! { notitia::OnAction::#variant });
        } else if key == "on_update" {
            if on_update.is_some() {
                return Err(Error::new_spanned(key, "duplicate `on_update`"));
            }
            on_update = Some(quote! { notitia::OnAction::#variant });
        } else if key == "on_actions" {
            if on_delete.is_some() || on_update.is_some() {
                return Err(Error::new_spanned(
                    key,
                    "`on_actions` cannot be used with `on_delete` or `on_update`",
                ));
            }
            let action = quote! { notitia::OnAction::#variant };
            on_delete = Some(action.clone());
            on_update = Some(action);
        } else {
            return Err(Error::new_spanned(
                key,
                "expected `on_delete`, `on_update`, or `on_actions`",
            ));
        }
    }

    Ok((
        on_delete.unwrap_or_else(|| quote! { notitia::OnAction::NoAction }),
        on_update.unwrap_or_else(|| quote! { notitia::OnAction::NoAction }),
    ))
}

fn parse_table_type(ty: &Type) -> Option<&Type> {
    let Type::Path(ty_path) = ty else { return None };

    let segment = ty_path.path.segments.last()?;
    if segment.ident != "Table" {
        return None;
    }

    let args = match &segment.arguments {
        PathArguments::AngleBracketed(args) => &args.args,
        _ => return None,
    };

    match args.first()? {
        GenericArgument::Type(inner_ty) => Some(inner_ty),
        _ => None,
    }
}

#[derive(Eq, PartialEq)]
struct RecordTyWithName<'a> {
    ty: &'a Type,
    name: String,
}

impl<'a> RecordTyWithName<'a> {
    fn new(ty: &'a Type, name: String) -> Self {
        Self { ty, name }
    }
}

impl<'a> Hash for RecordTyWithName<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ty.hash(state);
    }
}

fn type_name(ty: &Type) -> Option<String> {
    match ty {
        Type::Path(TypePath { path, .. }) => path.get_ident().map(|ident| ident.to_string()),
        _ => None,
    }
}
