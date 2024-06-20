use std::collections::HashMap;

use proc_macro2::Ident;
use quote::format_ident;
use syn::{Field, Item, ItemEnum, ItemStruct, Variant};

use crate::generate::get_type_ident_for_field;
use crate::generate::sculpt_set::filter_items::filter_item_struct;
use crate::generate::sculpt_set::find_aliases::{Aliases, find_aliases};
use crate::generate::sculpt_set::find_first::find_first;
use crate::generate::sculpt_set::generate_nexts::generate_nexts;
use crate::generate::sculpt_set::generate_routes::generate_routes;
use crate::generate::sculpt_set::link_items::link_items;

mod filter_items;
mod link_items;
mod generate_routes;
mod generate_nexts;
mod find_first;
mod find_aliases;

pub struct SculptSet {
    pub root:       ItemStruct,
    pub items:      Vec<Item>,
}

impl SculptSet {
    pub fn new(items: Vec<Item>) -> Result<SculptSet, String> {
        let (root, items) = find_root_and_set(items)?;
        Ok(SculptSet { root, items })
    }

    pub fn get_all_structs(&self) -> Vec<&ItemStruct> {
        self.type_links().into_iter()
            .filter_map(|(_, item)| match item {
                Item::Struct(item_struct) => Some(item_struct),
                _ => None
            })
            .collect()
    }

    pub fn get_all_enums(&self) -> Vec<&ItemEnum> {
        self.type_links().into_iter()
            .filter_map(|(_, item)| match item {
                Item::Enum(item_enum) => Some(item_enum),
                _ => None
            })
            .collect()
    }

    pub fn type_links(&self) -> HashMap<&Field, &Item> {
        link_items(&self.items, &self.root).unwrap_or_else(|_| HashMap::new())
    }

    pub fn routes(&self) -> HashMap<&Field, Vec<FieldOrVariant>> {
        generate_routes(&self.type_links(), &self.root.fields)
    }

    pub fn nexts(&self) -> HashMap<&Variant, Option<&Item>> {
        generate_nexts(&self.type_links(), &self.root.fields)
    }

    pub fn first(&self) -> Option<&ItemEnum> {
        find_first(&self.root, &self.type_links())
    }

    pub fn aliases(&self) -> Result<Aliases, String> {
        find_aliases(&self.items)
    }
}

#[derive(Clone)]
pub enum FieldOrVariant<'a> {
    Field(&'a Field),
    Variant(&'a Variant),
}

impl<'a> FieldOrVariant<'a> {
    pub(crate) fn builder_field(&self) -> Ident {
        format_ident!("{}_builder", match self {
            FieldOrVariant::Field(field) => get_type_ident_for_field(field),
            FieldOrVariant::Variant(variant) => variant.ident.clone()
        }.to_string().to_lowercase())
    }
}

// -------------------------------------------------------------------------------------------------
// SculptSet helper methods
// -------------------------------------------------------------------------------------------------

fn find_root_and_set(items: Vec<Item>) -> Result<(ItemStruct, Vec<Item>), String> {
    let (mut roots, mut items): (Vec<Item>, Vec<Item>) = items.into_iter()
        .partition(|item| has_sculpt_attribute(item));
    let root = if roots.is_empty() {
        return Err("Could not find sculpt attribute for root item.".to_string());
    } else if roots.len() > 1 {
        return Err("Multiple sculpt attributes found.".to_string());
    } else {
        if let Item::Struct(item_struct) = roots.remove(0) {
            item_struct
        } else {
            return Err("Root must be a struct".to_string());
        }
    };
    let items = filter_item_struct(&root, &mut items);
    Ok((root, items))
}

fn has_sculpt_attribute(item: &Item) -> bool {
    if let Item::Struct(item_struct) = &item {
        return item_struct.attrs.iter()
            .any(|attr| attr.path().is_ident("sculpt"));
    }
    false
}
