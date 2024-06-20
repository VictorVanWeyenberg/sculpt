use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::ItemEnum;

use crate::generate::{OPTIONS, SculptSet};

pub fn generate_picker_traits(sculpt_set: &SculptSet) -> TokenStream {
    sculpt_set.get_all_enums().into_iter()
        .unique()
        .map(generate_picker_trait)
        .reduce(|t1, t2| quote!(#t1 #t2))
        .unwrap()
}

fn generate_picker_trait(item_enum: &ItemEnum) -> TokenStream {
    let as_picker = format_ident!("{}Picker", item_enum.ident);
    let as_options = format_ident!("{}{}", item_enum.ident, OPTIONS);
    quote! {
        pub trait #as_picker {
            fn options(&self) -> Vec<#as_options> {
                #as_options::VARIANTS.to_vec()
            }
            fn fulfill(&mut self, requirement: &#as_options);
        }
    }
}