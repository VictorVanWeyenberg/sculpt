use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Field, Item, ItemEnum};

use crate::generate::{get_field_ident_for_field, get_type_ident_for_field, OPTIONS};
use crate::generate::sculpt_set::{FieldOrVariant, SculptSet};

pub fn generate_root_builder_picker_impls(sculpt_set: &SculptSet) -> TokenStream {
    sculpt_set.type_links.iter()
        .filter_map(|(field, item)| match item {
            Item::Enum(item_enum) => Some((field, item_enum)),
            _ => None
        })
        .map(|(field, item_enum)| {
            generate_root_builder_picker_impl(
                sculpt_set,
                field,
                item_enum,
                sculpt_set.routes.get(field).unwrap()
            )
        })
        .reduce(|t1, t2| quote!(#t1 #t2))
        .unwrap_or(quote! {})
}

fn generate_root_builder_picker_impl(sculpt_set: &SculptSet,
                                     field: &Field,
                                     item_enum: &ItemEnum,
                                     route: &Vec<FieldOrVariant>) -> TokenStream {
    let fulfill_method = generate_fulfill_method(sculpt_set, field, item_enum, route);
    let builder_type = format_ident!("{}Builder", sculpt_set.root.ident);
    let picker_type = format_ident!("{}Picker", item_enum.ident);
    let callbacks_type = format_ident!("{}Callbacks", builder_type);
    quote! {
        impl<'a, T: #callbacks_type> #picker_type for #builder_type<'a, T> {
            #fulfill_method
        }
    }
}

fn generate_fulfill_method(sculpt_set: &SculptSet,
                           field: &Field,
                           item_enum: &ItemEnum,
                           route: &Vec<FieldOrVariant>) -> TokenStream {
    let options_type = format_ident!("{}{}", item_enum.ident, OPTIONS);
    let set_path = generate_set_path(sculpt_set, field, route);
    let match_arms = generate_match_arms(sculpt_set, item_enum);
    quote! {
        fn fulfill(&mut self, requirement: &#options_type) {
            self.#set_path = Some(requirement.clone());
            match requirement {
                #(#match_arms,)*
            }
        }
    }
}

fn generate_set_path(sculpt_set: &SculptSet, field: &Field, route: &Vec<FieldOrVariant>) -> TokenStream {
    let sculptable = match sculpt_set.type_links.get(field).unwrap() {
        Item::Enum(item_enum) => item_enum.variants.iter()
            .any(|variant| !variant.fields.is_empty()),
        Item::Struct(_) => true,
        _ => false
    };
    let route = route.iter()
        .map(|fov| fov.builder_field());
    if sculptable {
        let field_ident = format_ident!("{}", get_type_ident_for_field(field)
            .to_string().to_lowercase());
        let builder_field = format_ident!("{}_builder", field_ident);
        quote!(#(#route.)*#builder_field.#field_ident)
    } else {
        let field_ident = get_field_ident_for_field(field);
        quote!(#(#route.)*#field_ident)
    }
}

fn generate_match_arms(sculpt_set: &SculptSet, item_enum: &ItemEnum) -> Vec<TokenStream> {
    let options_enum = format_ident!("{}{}", item_enum.ident, OPTIONS);
    item_enum.variants.iter()
        .map(|variant| {
            let variant_ident = &variant.ident;
            let next_enum = sculpt_set.nexts.get(variant).unwrap().as_ref()
                .map(|item| find_next_enum_from(sculpt_set, item))
                .unwrap_or(None);
            if let Some(item) = next_enum {
                let item_ident = &item.ident;
                let pick_method = format_ident!("pick_{}", item_ident.to_string().to_lowercase());
                quote!(#options_enum::#variant_ident => self.callbacks.#pick_method(self))
            } else {
                quote!(#options_enum::#variant_ident => {})
            }
        }).collect()
}

fn find_next_enum_from(sculpt_set: &SculptSet, item: &Item) -> Option<ItemEnum> {
    match item {
        Item::Enum(item_enum) => Some(item_enum.clone()),
        Item::Struct(item_struct) => item_struct.fields.iter()
            .find_map(|field| find_next_enum_from(sculpt_set, sculpt_set.type_links.get(field).unwrap())),
        _ => None
    }
}