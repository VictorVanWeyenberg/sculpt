use std::collections::HashMap;

use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{Field, Item, ItemEnum, ItemStruct, Type, Variant};
use crate::OPTIONS;
use crate::sculpt_set::callback_trait::generate_callback_trait;
use crate::sculpt_set::options_enums::generate_options_enums;
use crate::sculpt_set::pickable_builders::generate_pickable_builders;
use crate::sculpt_set::picker_traits::generate_picker_traits;
use crate::sculpt_set::struct_builders::generate_struct_builders;
use crate::sculpt_set::variant_builders::generate_variant_builders;

mod callback_trait;
mod picker_traits;
mod options_enums;
mod variant_builders;
mod pickable_builders;
mod struct_builders;

fn generate_builder_field(sculpt_set: &SculptSet, field: &Field) -> TokenStream {
    let field_type_ident = get_type_ident_for_field(field);
    if is_field_sculptable(sculpt_set, field) {
        let builder_field = format_ident!("{}_builder", field_type_ident.to_string().to_lowercase());
        let builder_type = format_ident!("{}Builder", field_type_ident);
        quote! {
            #builder_field: #builder_type
        }
    } else {
        let option_field = get_field_ident_for_field(field);
        let option_type = format_ident!("{}{}", field_type_ident, OPTIONS);
        quote! {
            #option_field: Option<#option_type>
        }
    }
}

fn is_field_sculptable(sculpt_set: &SculptSet, field: &Field) -> bool {
    match sculpt_set.type_links.get(field).unwrap() {
        Item::Struct(_) => true,
        Item::Enum(item_enum) => item_enum.variants.iter()
            .any(|variant| !variant.fields.is_empty()),
        _ => panic!("Field references something that's not an enum or a struct. Not supported.")
    }
}

fn get_type_ident_for_field(field: &Field) -> Ident {
    match &field.ty {
        Type::Path(type_path) => type_path.path.get_ident()
            .expect("Could not get identifier of field path type.").clone(),
        _ => panic!("None path types are not supported in the sculpt tree.")
    }
}

fn get_field_ident_for_field(field: &Field) -> Ident {
    let ident = field.ident.clone().unwrap_or(match &field.ty {
        Type::Path(type_path) => type_path.path.get_ident()
            .expect("Could not get identifier of field path type.").clone(),
        _ => panic!("None path types are not supported in the sculpt tree.")
    });
    format_ident!("{}", ident.to_string().to_lowercase())
}

fn field_to_builder_call(sculpt_set: &SculptSet, item_ident: &Ident, field: &Field) -> TokenStream {
    let variable = get_field_ident_for_field(field);
    if is_field_sculptable(sculpt_set, field) {
        let field_type_ident = get_type_ident_for_field(field);
        let builder_field = format_ident!("{}_builder", field_type_ident.to_string().to_lowercase());
        quote! {
            let #variable = self.#builder_field.build()
        }
    } else {
        let builder_type = format_ident!("{}Builder", item_ident);
        let message = format!("Field {} not set in {}.", variable, builder_type);
        quote! {
            let #variable = self.#variable.expect(#message).into()
        }
    }
}

pub struct SculptSet {
    root: ItemStruct,
    type_links: HashMap<Field, Item>,
}

impl SculptSet {
    pub fn new(items: Vec<Item>) -> Option<SculptSet> {
        items.clone().iter()
            .find_map(has_sculpt_attribute)
            .map(|root| SculptSet {
                root: root.clone(),
                type_links: link_item_struct(&items, &root).into_iter().collect(),
            })
    }

    #[allow(unused)]
    pub fn get_all_structs(&self) -> Vec<&ItemStruct> {
        self.type_links.iter()
            .filter_map(|(_, item)| match item {
                Item::Struct(item_struct) => Some(item_struct),
                _ => None
            })
            .collect()
    }

    pub fn get_all_enums(&self) -> Vec<&ItemEnum> {
        self.type_links.iter()
            .filter_map(|(_, item)| match item {
                Item::Enum(item_enum) => Some(item_enum),
                _ => None
            })
            .collect()
    }

    pub fn compile(self) -> TokenStream {
        [
            generate_callback_trait,
            generate_picker_traits,
            generate_options_enums,
            generate_variant_builders,
            generate_pickable_builders,
            generate_struct_builders
        ].iter()
            .map(|f| f(&self))
            .reduce(|t1, t2| quote!(#t1 #t2))
            .unwrap()
    }
}

fn has_sculpt_attribute(item: &Item) -> Option<ItemStruct> {
    match item {
        Item::Struct(item_struct) => {
            if item_struct.attrs.iter()
                .any(|attr| attr.path().is_ident("sculpt")) {
                Some(item_struct.clone())
            } else {
                None
            }
        }
        _ => return None
    }
}

fn link_item(items: &Vec<Item>, item: &Item) -> Vec<(Field, Item)> {
    match item {
        Item::Struct(item_struct) => link_item_struct(items, item_struct),
        Item::Enum(item_enum) => link_item_enum(items, item_enum),
        _ => vec![]
    }
}

fn link_item_struct(items: &Vec<Item>, item_struct: &ItemStruct) -> Vec<(Field, Item)> {
    item_struct.fields.iter()
        .map(|field| link_field(items, field))
        .concat()
}

fn link_item_enum(items: &Vec<Item>, item_enum: &ItemEnum) -> Vec<(Field, Item)> {
    item_enum.variants.iter()
        .map(|variant| link_variant(items, variant))
        .concat()
}

fn link_variant(items: &Vec<Item>, variant: &Variant) -> Vec<(Field, Item)> {
    variant.fields.iter()
        .map(|field| link_field(items, field))
        .concat()
}

fn link_field(items: &Vec<Item>, field: &Field) -> Vec<(Field, Item)> {
    let type_ident = match &field.ty {
        Type::Path(type_path) => type_path.path.get_ident()
            .expect(&format!("Type path has no identifier: {:?}. Not supported.", field.ty)),
        _ => panic!("Type is no path type: {:?}. Not supported", field.ty)
    };
    let item = items.iter()
        .find_map(|item| item_has_ident(item, type_ident))
        .expect(&format!("Could not find item with type {}. Could it be in another file?", type_ident));
    let mut links = link_item(items, item);
    links.push((field.clone(), item.clone()));
    links
}

fn item_has_ident<'a>(item: &'a Item, ident: &Ident) -> Option<&'a Item> {
    if match item {
        Item::Struct(item_struct) => &item_struct.ident,
        Item::Enum(item_enum) => &item_enum.ident,
        _ => return None
    }.eq(ident) {
        Some(item)
    } else {
        None
    }
}