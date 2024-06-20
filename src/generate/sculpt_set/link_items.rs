use std::collections::HashMap;

use proc_macro2::Ident;
use syn::{Field, Item, ItemEnum, ItemStruct, Type, Variant};

pub fn link_items<'a>(items: &'a Vec<Item>, item_struct: &'a ItemStruct) -> Result<HashMap<&'a Field, &'a Item>, String> {
    Ok(link_item_struct(items, item_struct)?.into_iter().collect())
}

fn link_item<'a>(items: &'a Vec<Item>, item: &'a Item) -> Result<Vec<(&'a Field, &'a Item)>, String> {
    match item {
        Item::Struct(item_struct) => link_item_struct(items, item_struct),
        Item::Enum(item_enum) => link_item_enum(items, item_enum),
        _ => Ok(vec![])
    }
}

fn link_item_struct<'a>(items: &'a Vec<Item>, item_struct: &'a ItemStruct) -> Result<Vec<(&'a Field, &'a Item)>, String> {
    item_struct.fields.iter()
        .map(|field| link_field(items, field))
        .collect::<Result<Vec<Vec<(&'a Field, &'a Item)>>, String>>()
        .map(|pairs| pairs.concat())
}

fn link_item_enum<'a>(items: &'a Vec<Item>, item_enum: &'a ItemEnum) -> Result<Vec<(&'a Field, &'a Item)>, String> {
    item_enum.variants.iter()
        .map(|variant| link_variant(items, variant))
        .collect::<Result<Vec<Vec<(&'a Field, &'a Item)>>, String>>()
        .map(|pairs| pairs.concat())
}

fn link_variant<'a>(items: &'a Vec<Item>, variant: &'a Variant) -> Result<Vec<(&'a Field, &'a Item)>, String> {
    variant.fields.iter()
        .map(|field| link_field(items, field))
        .collect::<Result<Vec<Vec<(&'a Field, &'a Item)>>, String>>()
        .map(|pairs| pairs.concat())
}

fn link_field<'a>(items: &'a Vec<Item>, field: &'a Field) -> Result<Vec<(&'a Field, &'a Item)>, String> {
    let type_ident = match &field.ty {
        Type::Path(type_path) => type_path.path.get_ident()
            .ok_or(format!("Type path has no identifier: {:?}. Not supported.", field.ty))?,
        _ => return Err(format!("Type is no path type: {:?}. Not supported.", field.ty))
    };
    let item = items.iter()
        .find_map(|item| item_has_ident(item, type_ident))
        .ok_or(format!("Could not find item with type {}. Could it be in another file?", type_ident))?;
    let mut links = link_item(items, item)?;
    links.push((field, item));
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