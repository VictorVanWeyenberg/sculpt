use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::ItemEnum;
use crate::sculpt_set::SculptSet;

pub fn generate_callback_trait(sculpt_set: &SculptSet) -> TokenStream {
    let root_builder_callbacks = format_ident!("{}BuilderCallbacks", sculpt_set.root.ident);
    let pick_methods = sculpt_set.get_all_enums().into_iter()
        .map(generate_pick_method)
        .collect::<Vec<TokenStream>>();
    quote! {
        pub trait #root_builder_callbacks {
            #(#pick_methods)*
        }
    }
}

fn generate_pick_method(item_enum: &ItemEnum) -> TokenStream {
    let pick_method = format_ident!("pick_{}", item_enum.ident.to_string().to_lowercase());
    let picker_trait = format_ident!("{}Picker", item_enum.ident);
    quote! {
        fn #pick_method(&self, picker: &mut impl #picker_trait) {
            let choice = picker.options()[0];
            picker.fulfill(&choice);
        }
    }
}