use std::fmt;

use anyhow::{bail, Error};
use syn::{Attribute, Lit, Meta, MetaList, MetaNameValue, NestedMeta};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Single,
    Repeated,
    Optional,
    Oneof,
}

#[derive(Clone)]
pub struct Field {
    pub kind: Kind,
    pub tags: Vec<u32>,
}

impl Field {
    pub fn new(attrs: Vec<Attribute>) -> Result<Self, Error> {
        let attrs = noproto_attrs(attrs);

        let mut tag = None;
        let mut tags = None;
        let mut kind = None;
        let mut unknown_attrs = Vec::new();

        for attr in &attrs {
            if let Some(x) = tag_attr(attr)? {
                set_option(&mut tag, x, "duplicate tag attributes")?;
            } else if let Some(x) = tags_attr(attr)? {
                set_option(&mut tags, x, "duplicate tags attributes")?;
            } else if let Some(x) = kind_attr(attr) {
                set_option(&mut kind, x, "duplicate kind attribute")?;
            } else {
                unknown_attrs.push(attr);
            }
        }

        match unknown_attrs.len() {
            0 => (),
            1 => bail!("unknown attribute: {:?}", unknown_attrs[0]),
            _ => bail!("unknown attributes: {:?}", unknown_attrs),
        }

        let kind = kind.unwrap_or(Kind::Single);
        let tags = match kind {
            Kind::Oneof => {
                if tag.is_some() {
                    bail!("tag attribute must not be set in oneof.")
                }
                match tags {
                    Some(tags) => tags,
                    None => bail!("missing tags attribute in oneof"),
                }
            }
            _ => match tag {
                Some(tag) => vec![tag],
                None => bail!("missing tag attribute"),
            },
        };

        Ok(Self { tags, kind })
    }
}

#[derive(Clone)]
pub struct OneofVariant {
    pub tag: u32,
}

impl OneofVariant {
    pub fn new(attrs: Vec<Attribute>) -> Result<Self, Error> {
        let attrs = noproto_attrs(attrs);

        let mut tag = None;
        let mut unknown_attrs = Vec::new();

        for attr in &attrs {
            if let Some(x) = tag_attr(attr)? {
                set_option(&mut tag, x, "duplicate tag attributes")?;
            } else {
                unknown_attrs.push(attr);
            }
        }

        match unknown_attrs.len() {
            0 => (),
            1 => bail!("unknown attribute: {:?}", unknown_attrs[0]),
            _ => bail!("unknown attributes: {:?}", unknown_attrs),
        }

        let tag = match tag {
            Some(tag) => tag,
            None => bail!("missing tag attribute"),
        };

        Ok(Self { tag })
    }
}

pub(super) fn tag_attr(attr: &Meta) -> Result<Option<u32>, Error> {
    if !attr.path().is_ident("tag") {
        return Ok(None);
    }
    match *attr {
        Meta::List(ref meta_list) => {
            // TODO(rustlang/rust#23121): slice pattern matching would make this much nicer.
            if meta_list.nested.len() == 1 {
                if let NestedMeta::Lit(Lit::Int(ref lit)) = meta_list.nested[0] {
                    return Ok(Some(lit.base10_parse()?));
                }
            }
            bail!("invalid tag attribute: {:?}", attr);
        }
        Meta::NameValue(ref meta_name_value) => match meta_name_value.lit {
            Lit::Str(ref lit) => lit.value().parse::<u32>().map_err(Error::from).map(Option::Some),
            Lit::Int(ref lit) => Ok(Some(lit.base10_parse()?)),
            _ => bail!("invalid tag attribute: {:?}", attr),
        },
        _ => bail!("invalid tag attribute: {:?}", attr),
    }
}

fn tags_attr(attr: &Meta) -> Result<Option<Vec<u32>>, Error> {
    if !attr.path().is_ident("tags") {
        return Ok(None);
    }
    match *attr {
        Meta::List(ref meta_list) => {
            let mut tags = Vec::with_capacity(meta_list.nested.len());
            for item in &meta_list.nested {
                if let NestedMeta::Lit(Lit::Int(ref lit)) = *item {
                    tags.push(lit.base10_parse()?);
                } else {
                    bail!("invalid tag attribute: {:?}", attr);
                }
            }
            Ok(Some(tags))
        }
        Meta::NameValue(MetaNameValue {
            lit: Lit::Str(ref lit), ..
        }) => lit
            .value()
            .split(',')
            .map(|s| s.trim().parse::<u32>().map_err(Error::from))
            .collect::<Result<Vec<u32>, _>>()
            .map(Some),
        _ => bail!("invalid tag attribute: {:?}", attr),
    }
}

fn kind_attr(attr: &Meta) -> Option<Kind> {
    let Meta::Path(ref path) = *attr else { return None };

    if path.is_ident("repeated") {
        Some(Kind::Repeated)
    } else if path.is_ident("optional") {
        Some(Kind::Optional)
    } else if path.is_ident("oneof") {
        Some(Kind::Oneof)
    } else {
        None
    }
}

pub fn set_option<T: fmt::Debug>(option: &mut Option<T>, value: T, message: &str) -> Result<(), Error> {
    if let Some(ref existing) = *option {
        bail!("{}: {:?} and {:?}", message, existing, value);
    }
    *option = Some(value);
    Ok(())
}

/// Get the items belonging to the 'noproto' list attribute, e.g. `#[noproto(foo, bar="baz")]`.
fn noproto_attrs(attrs: Vec<Attribute>) -> Vec<Meta> {
    attrs
        .iter()
        .flat_map(Attribute::parse_meta)
        .flat_map(|meta| match meta {
            Meta::List(MetaList { path, nested, .. }) => {
                if path.is_ident("noproto") {
                    nested.into_iter().collect()
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        })
        .flat_map(|attr| -> Result<_, _> {
            match attr {
                NestedMeta::Meta(attr) => Ok(attr),
                NestedMeta::Lit(lit) => bail!("invalid noproto attribute: {:?}", lit),
            }
        })
        .collect()
}
