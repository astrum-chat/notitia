use std::borrow::Borrow;

use syn::{Attribute, spanned::Spanned};

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
