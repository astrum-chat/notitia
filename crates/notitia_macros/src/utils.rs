use std::borrow::Borrow;

use syn::{Attribute, Ident, Token, parse::Parse, spanned::Spanned};

pub fn attr_is(attr: &Attribute, ident: &str, name: &str) -> bool {
    if !attr.path().is_ident(ident) {
        return false;
    }

    let mut is = false;

    let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident(name) {
            is = true;

            return Err(syn::Error::new(meta.path.span(), "is"));
        }

        Ok(())
    });

    return is;
}

pub fn get_attr_idx<T>(attrs: &[T], ident: &str, name: &str) -> Option<usize>
where
    T: Borrow<Attribute>,
{
    for (attr_idx, attr) in attrs.iter().enumerate() {
        if !attr_is(attr.borrow(), ident, name) {
            continue;
        }
        return Some(attr_idx);
    }
    None
}

/// Result of parsing `#[db(embed)]` or `#[db(embed(metric = Cosine))]`.
#[cfg(feature = "embeddings")]
pub struct EmbedAttr {
    /// The metric string: "cosine", "l2", "ip", or "default".
    pub metric: String,
}

/// Try to parse an `embed` attribute from `#[db(embed)]` or `#[db(embed(metric = Variant))]`.
///
/// Returns `Some((attr_index, EmbedAttr))` if found, `None` otherwise.
#[cfg(feature = "embeddings")]
pub fn get_embed_attr<T>(attrs: &[T], ident: &str) -> Option<(usize, EmbedAttr)>
where
    T: Borrow<Attribute>,
{
    use syn::Ident;

    for (attr_idx, attr) in attrs.iter().enumerate() {
        let attr = attr.borrow();

        if !attr.path().is_ident(ident) {
            continue;
        }

        let mut found = false;
        let mut metric = String::from("default");

        let _ = attr.parse_nested_meta(|meta| {
            if !meta.path.is_ident("embed") {
                return Ok(());
            }

            found = true;

            // Check for parenthesized args: embed(metric = Cosine)
            if meta.input.peek(syn::token::Paren) {
                let content;
                syn::parenthesized!(content in meta.input);

                let key: Ident = content.parse()?;
                if key != "metric" {
                    return Err(syn::Error::new_spanned(key, "expected `metric`"));
                }

                content.parse::<syn::Token![=]>()?;
                let variant: Ident = content.parse()?;

                metric = match variant.to_string().as_str() {
                    "Cosine" => "cosine".to_string(),
                    "L2" => "l2".to_string(),
                    "Ip" => "ip".to_string(),
                    _ => {
                        return Err(syn::Error::new_spanned(
                            variant,
                            "expected `Cosine`, `L2`, or `Ip`",
                        ));
                    }
                };
            }

            Ok(())
        });

        if found {
            return Some((attr_idx, EmbedAttr { metric }));
        }
    }

    None
}

/// Parse `migrate_from(ident1, ident2, ...)` from `#[db(...)]` attributes on a field.
/// Returns `Some((attr_index, Vec<String>))` if found, `None` otherwise.
pub fn get_migrate_from_attr<T>(attrs: &[T], ident: &str) -> Option<(usize, Vec<String>)>
where
    T: Borrow<Attribute>,
{
    for (attr_idx, attr) in attrs.iter().enumerate() {
        let attr = attr.borrow();

        if !attr.path().is_ident(ident) {
            continue;
        }

        let mut found = false;
        let mut names = Vec::new();

        let _ = attr.parse_nested_meta(|meta| {
            if !meta.path.is_ident("migrate_from") {
                return Ok(());
            }

            found = true;

            let content;
            syn::parenthesized!(content in meta.input);

            let idents: syn::punctuated::Punctuated<Ident, Token![,]> =
                content.parse_terminated(Ident::parse, Token![,])?;
            for id in idents {
                names.push(id.to_string());
            }

            Ok(())
        });

        if found {
            return Some((attr_idx, names));
        }
    }

    None
}

/// Parse a parenthesized list of idents from a `TokenStream`.
/// Used for `#[record(removed_fields(a, b))]` and `#[database(removed_tables(a, b))]`.
pub fn parse_ident_list_attr(
    attr: proc_macro::TokenStream,
    expected_name: &str,
) -> Vec<String> {
    use syn::parse::Parser;

    let mut names = Vec::new();

    let parser = |input: syn::parse::ParseStream| -> syn::Result<()> {
        while !input.is_empty() {
            let meta_ident: Ident = input.parse()?;

            if meta_ident == expected_name {
                let content;
                syn::parenthesized!(content in input);
                let idents =
                    content.parse_terminated(Ident::parse, Token![,])?;
                for id in idents {
                    names.push(id.to_string());
                }
            }

            // Skip comma between top-level items
            let _ = input.parse::<Token![,]>();
        }
        Ok(())
    };

    let _ = parser.parse(attr);
    names
}
