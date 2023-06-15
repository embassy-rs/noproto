// The `quote!` macro requires deep recursion.
#![recursion_limit = "4096"]

extern crate alloc;
extern crate proc_macro;

use anyhow::{bail, Error};
use field::{Kind, OneofVariant};
use itertools::Itertools;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{Data, DataEnum, DataStruct, DeriveInput, Expr, Fields, FieldsNamed, FieldsUnnamed, Ident, Index, Variant};

mod field;
use crate::field::Field;

fn try_message(input: TokenStream) -> Result<TokenStream, Error> {
    let input: DeriveInput = syn::parse(input)?;

    let ident = input.ident;

    let variant_data = match input.data {
        Data::Struct(variant_data) => variant_data,
        Data::Enum(..) => bail!("Message can not be derived for an enum"),
        Data::Union(..) => bail!("Message can not be derived for a union"),
    };

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let (_is_struct, fields) = match variant_data {
        DataStruct {
            fields: Fields::Named(FieldsNamed { named: fields, .. }),
            ..
        } => (true, fields.into_iter().collect()),
        DataStruct {
            fields: Fields::Unnamed(FieldsUnnamed { unnamed: fields, .. }),
            ..
        } => (false, fields.into_iter().collect()),
        DataStruct {
            fields: Fields::Unit, ..
        } => (false, Vec::new()),
    };

    let mut fields = fields
        .into_iter()
        .enumerate()
        .map(|(i, field)| {
            let field_ident = field.ident.map(|x| quote!(#x)).unwrap_or_else(|| {
                let index = Index {
                    index: i as u32,
                    span: Span::call_site(),
                };
                quote!(#index)
            });
            match Field::new(field.attrs) {
                Ok(field) => Ok((field_ident, field)),
                Err(err) => Err(err.context(format!("invalid message field {}.{}", ident, field_ident))),
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Sort the fields by tag number so that fields will be encoded in tag order.
    // TODO: This encodes oneof fields in the position of their lowest tag,
    // regardless of the currently occupied variant, is that consequential?
    // See: https://developers.google.com/protocol-buffers/docs/encoding#order
    fields.sort_by_key(|&(_, ref field)| field.tags.iter().copied().min().unwrap());
    let fields = fields;

    let mut tags = fields.iter().flat_map(|(_, field)| &field.tags).collect::<Vec<_>>();
    let num_tags = tags.len();
    tags.sort_unstable();
    tags.dedup();
    if tags.len() != num_tags {
        bail!("message {} has fields with duplicate tags", ident);
    }

    let write = fields.iter().map(|&(ref field_ident, ref field)| {
        let tag = field.tags[0];
        let ident = quote!(self.#field_ident);
        match field.kind {
            Kind::Single => quote!(w.write_field(#tag, &#ident)?;),
            Kind::Repeated => quote!(w.write_repeated(#tag, &#ident)?;),
            Kind::Optional => quote!(w.write_optional(#tag, &#ident)?;),
            Kind::Oneof => quote!(w.write_oneof(&#ident)?;),
        }
    });

    let read = fields.iter().map(|&(ref field_ident, ref field)| {
        let ident = quote!(self.#field_ident);
        let read = match field.kind {
            Kind::Single => quote!(r.read(&mut #ident)?;),
            Kind::Repeated => quote!(r.read_repeated(&mut #ident)?;),
            Kind::Optional => quote!(r.read_optional(&mut #ident)?;),
            Kind::Oneof => quote!(r.read_oneof(&mut #ident)?;),
        };

        let tags = field.tags.iter().map(|&tag| quote!(#tag));
        let tags = Itertools::intersperse(tags, quote!(|));

        quote!(#(#tags)* => { #read })
    });

    let expanded = quote! {
        impl #impl_generics ::noproto::Message for #ident #ty_generics #where_clause {
            const WIRE_TYPE: ::noproto::WireType = ::noproto::WireType::LengthDelimited;

            fn write_raw(&self, w: &mut ::noproto::encoding::ByteWriter) -> Result<(), ::noproto::WriteError> {
                #(#write)*
                Ok(())
            }

            fn read_raw(&mut self, r: &mut ::noproto::encoding::ByteReader) -> Result<(), ::noproto::ReadError> {
                for r in r.read_fields() {
                    let r = r?;
                    match r.tag() {
                        #(#read)*
                        _ => {}
                    }
                }
                Ok(())
            }
        }
    };

    Ok(expanded.into())
}

#[proc_macro_derive(Message, attributes(noproto))]
pub fn message(input: TokenStream) -> TokenStream {
    try_message(input).unwrap()
}

fn try_enumeration(input: TokenStream) -> Result<TokenStream, Error> {
    let input: DeriveInput = syn::parse(input)?;
    let ident = input.ident;

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let punctuated_variants = match input.data {
        Data::Enum(DataEnum { variants, .. }) => variants,
        Data::Struct(_) => bail!("Enumeration can not be derived for a struct"),
        Data::Union(..) => bail!("Enumeration can not be derived for a union"),
    };

    // Map the variants into 'fields'.
    let mut variants: Vec<(Ident, Expr)> = Vec::new();
    for Variant {
        ident,
        fields,
        discriminant,
        ..
    } in punctuated_variants
    {
        match fields {
            Fields::Unit => (),
            Fields::Named(_) | Fields::Unnamed(_) => {
                bail!("Enumeration variants may not have fields")
            }
        }

        match discriminant {
            Some((_, expr)) => variants.push((ident, expr)),
            None => bail!("Enumeration variants must have a discriminant"),
        }
    }

    if variants.is_empty() {
        panic!("Enumeration must have at least one variant");
    }

    let _default = variants[0].0.clone();

    let _is_valid = variants.iter().map(|&(_, ref value)| quote!(#value => true));

    let write = variants
        .iter()
        .map(|(variant, value)| quote!(#ident::#variant => #value));

    let read = variants
        .iter()
        .map(|(variant, value)| quote!(#value => #ident::#variant ));

    let expanded = quote! {
        impl #impl_generics  ::noproto::Message for #ident #ty_generics #where_clause {

            const WIRE_TYPE: ::noproto::WireType = ::noproto::WireType::Varint;

            fn write_raw(&self, w: &mut ::noproto::encoding::ByteWriter) -> Result<(), ::noproto::WriteError> {
                let val = match self {
                    #(#write,)*
                };
                w.write_varuint32(*self as _)
            }

            fn read_raw(&mut self, r: &mut ::noproto::encoding::ByteReader) -> Result<(), ::noproto::ReadError> {
                *self = match r.read_varuint32()? {
                    #(#read,)*
                    _ => return Err(::noproto::ReadError),
                };
                Ok(())
            }
        }
    };

    Ok(expanded.into())
}

#[proc_macro_derive(Enumeration, attributes(noproto))]
pub fn enumeration(input: TokenStream) -> TokenStream {
    try_enumeration(input).unwrap()
}

fn try_oneof(input: TokenStream) -> Result<TokenStream, Error> {
    let input: DeriveInput = syn::parse(input)?;

    let ident = input.ident;

    let variants = match input.data {
        Data::Enum(DataEnum { variants, .. }) => variants,
        Data::Struct(..) => bail!("Oneof can not be derived for a struct"),
        Data::Union(..) => bail!("Oneof can not be derived for a union"),
    };

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Map the variants
    let mut oneof_variants: Vec<(Ident, OneofVariant)> = Vec::new();
    for Variant {
        attrs,
        ident: variant_ident,
        fields: variant_fields,
        ..
    } in variants
    {
        let variant_fields = match variant_fields {
            Fields::Unit => Punctuated::new(),
            Fields::Named(FieldsNamed { named: fields, .. })
            | Fields::Unnamed(FieldsUnnamed { unnamed: fields, .. }) => fields,
        };
        if variant_fields.len() != 1 {
            bail!("Oneof enum variants must have a single field");
        }

        match OneofVariant::new(attrs) {
            Ok(variant) => oneof_variants.push((variant_ident, variant)),
            Err(err) => bail!("invalid oneof variant {}.{}: {}", ident, variant_ident, err),
        }
    }

    let mut tags = oneof_variants.iter().map(|(_, v)| v.tag).collect::<Vec<_>>();
    tags.sort_unstable();
    tags.dedup();
    if tags.len() != oneof_variants.len() {
        panic!("invalid oneof {}: variants have duplicate tags", ident);
    }

    let write = oneof_variants.iter().map(|(variant_ident, variant)| {
        let tag = variant.tag;
        quote!(#ident::#variant_ident(value) => { w.write_field(#tag, value)?; })
    });

    let read = oneof_variants.iter().map(|(variant_ident, variant)| {
        let tag = variant.tag;
        quote!(#tag => {
            *self = #ident::#variant_ident(r.read_oneof_variant()?);
        })
    });

    let read_option = oneof_variants.iter().map(|(variant_ident, variant)| {
        let tag = variant.tag;
        quote!(#tag => {
            *this = Some(#ident::#variant_ident(r.read_oneof_variant()?));
        })
    });

    let expanded = quote! {
        impl #impl_generics ::noproto::Oneof for #ident #ty_generics #where_clause {
            fn write_raw(&self, w: &mut ::noproto::encoding::ByteWriter) -> Result<(), ::noproto::WriteError> {
                match self {
                    #(#write)*
                }
                Ok(())
            }

            fn read_raw(&mut self, r: ::noproto::encoding::FieldReader) -> Result<(), ::noproto::ReadError> {
                match r.tag() {
                    #(#read)*
                    _ => return Err(::noproto::ReadError),
                }
                Ok(())
            }

            fn read_raw_option(this: &mut Option<Self>, r: ::noproto::encoding::FieldReader) -> Result<(), ::noproto::ReadError> {
                match r.tag() {
                    #(#read_option)*
                    _ => return Err(::noproto::ReadError),
                }
                Ok(())
            }
        }
    };

    Ok(expanded.into())
}

#[proc_macro_derive(Oneof, attributes(noproto))]
pub fn oneof(input: TokenStream) -> TokenStream {
    try_oneof(input).unwrap()
}
