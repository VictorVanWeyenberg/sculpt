use std::collections::HashMap;

use proc_macro2::Ident;
use syn::{Field, Item, ItemEnum, ItemStruct, Type, Variant};

pub struct SculptSet {
    pub root: ItemStruct,
    pub type_links: HashMap<Field, Item>,
}

impl SculptSet {
    pub fn new(items: Vec<Item>) -> Result<SculptSet, String> {
        let root = items.iter()
            .find_map(has_sculpt_attribute)
            .ok_or("Could not find `sculpt` attribute in file.".to_string())?;
        let type_links = link_item_struct(&items, &root)?.into_iter().collect();
        Ok(SculptSet { root, type_links })
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

fn link_item(items: &Vec<Item>, item: &Item) -> Result<Vec<(Field, Item)>, String> {
    match item {
        Item::Struct(item_struct) => link_item_struct(items, item_struct),
        Item::Enum(item_enum) => link_item_enum(items, item_enum),
        _ => Ok(vec![])
    }
}

fn link_item_struct(items: &Vec<Item>, item_struct: &ItemStruct) -> Result<Vec<(Field, Item)>, String> {
    item_struct.fields.iter()
        .map(|field| link_field(items, field))
        .collect::<Result<Vec<Vec<(Field, Item)>>, String>>()
        .map(|pairs| pairs.concat())
}

fn link_item_enum(items: &Vec<Item>, item_enum: &ItemEnum) -> Result<Vec<(Field, Item)>, String> {
    item_enum.variants.iter()
        .map(|variant| link_variant(items, variant))
        .collect::<Result<Vec<Vec<(Field, Item)>>, String>>()
        .map(|pairs| pairs.concat())
}

fn link_variant(items: &Vec<Item>, variant: &Variant) -> Result<Vec<(Field, Item)>, String> {
    variant.fields.iter()
        .map(|field| link_field(items, field))
        .collect::<Result<Vec<Vec<(Field, Item)>>, String>>()
        .map(|pairs| pairs.concat())
}

fn link_field(items: &Vec<Item>, field: &Field) -> Result<Vec<(Field, Item)>, String> {
    let type_ident = match &field.ty {
        Type::Path(type_path) => type_path.path.get_ident()
            .ok_or(format!("Type path has no identifier: {:?}. Not supported.", field.ty))?,
        _ => return Err(format!("Type is no path type: {:?}. Not supported.", field.ty))
    };
    let item = items.iter()
        .find_map(|item| item_has_ident(item, type_ident))
        .ok_or(format!("Could not find item with type {}. Could it be in another file?", type_ident))?;
    let mut links = link_item(items, item)?;
    links.push((field.clone(), item.clone()));
    Ok(links)
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