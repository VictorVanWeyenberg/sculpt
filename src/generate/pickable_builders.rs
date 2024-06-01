use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{ItemEnum, Variant};

use crate::generate::{OPTIONS, SculptSet};

pub fn generate_pickable_builders(sculpt_set: &SculptSet) -> TokenStream {
    sculpt_set.get_all_enums().into_iter()
        .filter_map(generate_pickable_builder_and_impl)
        .reduce(|t1, t2| quote!(#t1 #t2))
        .unwrap_or(quote! {})
}

fn generate_pickable_builder_and_impl(item_enum: &ItemEnum) -> Option<TokenStream> {
    if item_enum.variants.iter()
        .all(|variant| variant.fields.is_empty()) {
        return None
    }
    let pickable_builer = generate_pickable_builder(item_enum);
    let pickable_builder_impl = generate_pickable_builder_impl(item_enum);
    Some(quote! {
        #pickable_builer
        #pickable_builder_impl
    })
}

fn generate_pickable_builder(item_enum: &ItemEnum) -> TokenStream {
    let builder_type = format_ident!("{}Builder", item_enum.ident);
    let option_field = format_ident!("{}", item_enum.ident.to_string().to_lowercase());
    let options_type = format_ident!("{}{}", item_enum.ident, OPTIONS);
    let variant_builders = generate_variant_builder_fields(item_enum);
    quote! {
        #[derive(Default)]
        struct #builder_type {
            #option_field: Option<#options_type>,
            #(#variant_builders,)*
        }
    }
}

fn generate_variant_builder_fields(item_enum: &ItemEnum) -> Vec<TokenStream> {
    item_enum.variants.iter()
        .filter_map(generate_variant_builder_field)
        .collect()
}

fn generate_variant_builder_field(variant: &Variant) -> Option<TokenStream> {
    if variant.fields.is_empty() {
        return None
    }
    let builder_field = format_ident!("{}_builder", variant.ident.to_string().to_lowercase());
    let builder_type = format_ident!("{}Builder", variant.ident);
    Some(quote! {
        #builder_field: #builder_type
    })
}

fn generate_pickable_builder_impl(item_enum: &ItemEnum) -> TokenStream {
    let builder_type = format_ident!("{}Builder", item_enum.ident);
    let simple_type = &item_enum.ident;
    let option_field = format_ident!("{}", item_enum.ident.to_string().to_lowercase());
    let pickable_builder_build_calls = item_enum.variants.iter()
        .map(|variant| generate_build_call(&item_enum.ident, variant))
        .collect::<Vec<TokenStream>>();
    quote! {
        impl #builder_type {
            fn build(self) -> #simple_type {
                match self.#option_field.unwrap() {
                    #(#pickable_builder_build_calls,)*
                }
            }
        }
    }
}

fn generate_build_call(enum_ident: &Ident, variant: &Variant) -> TokenStream {
    let option_field = format_ident!("{}", enum_ident.to_string().to_lowercase());
    let options_type = format_ident!("{}{}", enum_ident, OPTIONS);
    let simple_variant = &variant.ident;
    if variant.fields.is_empty() {
        let message = format!("Field {} not set in {}Builder", option_field, enum_ident);
        quote! {
            #options_type::#simple_variant => self.#option_field.expect(#message).into()
        }
    } else {
        let builder_field = format_ident!("{}_builder", simple_variant.to_string().to_lowercase());
        quote! {
            #options_type::#simple_variant => self.#builder_field.build()
        }
    }
}
