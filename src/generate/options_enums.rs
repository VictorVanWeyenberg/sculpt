use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{ItemEnum, Variant};
use crate::OPTIONS;
use crate::generate::SculptSet;

pub fn generate_options_enums(sculpt_set: &SculptSet) -> TokenStream {
    sculpt_set.get_all_enums().into_iter()
        .map(generate_options_enum_blocks)
        .reduce(|t1, t2| quote!(#t1 #t2))
        .unwrap()
}

fn generate_options_enum_blocks(item_enum: &ItemEnum) -> TokenStream {
    [
        generate_options_enum,
        generate_options_enum_impl,
        generate_options_enum_into
    ].into_iter()
        .map(|f| f(item_enum))
        .reduce(|t1, t2| quote!(#t1 #t2))
        .unwrap()
}

fn generate_options_enum(item_enum: &ItemEnum) -> TokenStream {
    let options_type = format_ident!("{}{}", item_enum.ident, OPTIONS);
    let options = item_enum.variants.iter()
        .map(|variant| &variant.ident)
        .collect::<Vec<&Ident>>();
    quote! {
        #[derive(Clone, Copy)]
        pub enum #options_type {
            #(#options,)*
        }
    }
}

fn generate_options_enum_impl(item_enum: &ItemEnum) -> TokenStream {
    let options_type = format_ident!("{}{}", item_enum.ident, OPTIONS);
    let variants = item_enum.variants.iter()
        .map(|variant| &variant.ident)
        .map(|ident| quote!(#options_type::#ident))
        .collect::<Vec<TokenStream>>();
    quote! {
        impl #options_type {
            const VARIANTS: &'static [Self] = &[
                #(#variants,)*
            ];
        }
    }
}

fn generate_options_enum_into(item_enum: &ItemEnum) -> TokenStream {
    let enum_ident = &item_enum.ident;
    let options_type = format_ident!("{}{}", enum_ident, OPTIONS);
    let conversions = item_enum.variants.iter()
        .map(|variant| generate_conversion(enum_ident, variant))
        .collect::<Vec<TokenStream>>();
    quote! {
        impl Into<#enum_ident> for #options_type {
            fn into(self) -> #enum_ident {
                match self {
                    #(#conversions,)*
                }
            }
        }
    }
}

fn generate_conversion(enum_ident: &Ident, variant: &Variant) -> TokenStream {
    let options_type = format_ident!("{}{}", enum_ident, OPTIONS);
    let variant_ident = &variant.ident;
    let rhs = if variant.fields.is_empty() {
        quote!(#enum_ident::#variant_ident)
    } else {
        let message = stringify!(
            Cannot turn #options_type::#variant_ident into #enum_ident without dependencies.);
        quote!(panic!(#message))
    };
    quote! {
        #options_type::#variant_ident => #rhs
    }
}