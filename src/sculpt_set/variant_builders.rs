use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{Fields, ItemEnum, Variant};

use crate::sculpt_set::{field_to_builder_call, generate_builder_field, get_field_ident_for_field, SculptSet};

pub fn generate_variant_builders(sculpt_set: &SculptSet) -> TokenStream {
    sculpt_set.get_all_enums().into_iter()
        .map(|item| generate_variants_builder_and_impl(sculpt_set, item))
        .reduce(|t1, t2| quote!(#t1 #t2))
        .unwrap_or(quote! {})
}

fn generate_variants_builder_and_impl(sculpt_set: &SculptSet, item_enum: &ItemEnum) -> TokenStream {
    item_enum.variants.iter()
        .filter_map(|variant| generate_variant_builder_and_impl(sculpt_set, &item_enum.ident, variant))
        .reduce(|t1, t2| quote!(#t1 #t2))
        .unwrap_or(quote! {})
}

fn generate_variant_builder_and_impl(sculpt_set: &SculptSet, enum_ident: &Ident, variant: &Variant) -> Option<TokenStream> {
    if variant.fields.is_empty() {
        return None
    }
    let variant_builder = generate_variant_builder(sculpt_set, variant);
    let variant_builder_impl = generate_variant_builder_impl(sculpt_set, enum_ident, variant);
    Some(quote! {
        #variant_builder
        #variant_builder_impl
    })
}

fn generate_variant_builder(sculpt_set: &SculptSet, variant: &Variant) -> TokenStream {
    let builder_type = format_ident!("{}Builder", variant.ident);
    let builder_fields = variant.fields.iter()
        .map(|field| generate_builder_field(sculpt_set, field))
        .collect::<Vec<TokenStream>>();
    quote! {
        #[derive(Default)]
        struct #builder_type {
            #(#builder_fields,)*
        }
    }
}

fn generate_variant_builder_impl(sculpt_set: &SculptSet, enum_ident: &Ident, variant: &Variant) -> TokenStream {
    let builder_type = format_ident!("{}Builder", variant.ident);
    let builder_calls = variant.fields.iter()
        .map(|field| field_to_builder_call(sculpt_set, enum_ident, field))
        .collect::<Vec<TokenStream>>();
    let constructor = generate_constructor_call(enum_ident, variant);
    quote! {
        impl #builder_type {
            pub fn build(self) -> #enum_ident {
                #(#builder_calls;)*
                #constructor
            }
        }
    }
}

fn generate_constructor_call(enum_ident: &Ident, variant: &Variant) -> TokenStream {
    let fields = match &variant.fields {
        Fields::Named(fields_named) => &fields_named.named,
        Fields::Unnamed(fields_unnamed) => &fields_unnamed.unnamed,
        Fields::Unit => panic!("Generating constructor call for RAW enum type.")
    }.iter()
        .map(get_field_ident_for_field)
        .collect::<Vec<Ident>>();
    let variant_ident = &variant.ident;
    match &variant.fields {
        Fields::Named(_) => quote! { #enum_ident::#variant_ident { #(#fields,)* } },
        Fields::Unnamed(_) => quote! { #enum_ident::#variant_ident ( #(#fields,)* ) },
        Fields::Unit => quote! { panic!("Calling constructor for RAW enum type.") }
    }
}