use itertools::Itertools;
use syn::{Field, Fields, Item, ItemEnum, ItemStruct, Type};

fn filter_items(root: &Item, items: &mut Vec<Item>) -> Vec<Item> {
    match root {
        Item::Struct(item_struct) => filter_item_struct(item_struct, items),
        Item::Enum(item_enum) => filter_item_enum(item_enum, items),
        _ => vec![]
    }
}

pub fn filter_item_struct(item_struct: &ItemStruct, items: &mut Vec<Item>) -> Vec<Item> {
    let mut filtered: Vec<Item> = filter_fields(&item_struct.fields, items);
    let extended = filtered.iter()
        .map(|item| filter_items(item, items))
        .flatten()
        .collect::<Vec<Item>>();
    filtered.extend(extended);
    filtered
}

fn filter_fields(fields: &Fields, items: &mut Vec<Item>) -> Vec<Item> {
    let mut filtered: Vec<Item> = fields.iter()
        .filter_map(|field| find_item_for_field(field, items))
        .collect();
    for item in filtered.clone() {
        filtered.extend(filter_items(&item, items));
    }
    filtered
}

fn find_item_for_field(field: &Field, items: &mut Vec<Item>) -> Option<Item> {
    let field_type_path = match &field.ty {
        Type::Path(type_path) => Some(&type_path.path),
        _ => None
    }?;
    let (index, _item) = items.into_iter()
        .find_position(|item| match item {
            Item::Struct(item_struct) => field_type_path.is_ident(&item_struct.ident),
            Item::Enum(item_enum) => field_type_path.is_ident(&item_enum.ident),
            _ => false
        })?;
    Some(items.remove(index))
}

fn filter_item_enum(item_enum: &ItemEnum, items: &mut Vec<Item>) -> Vec<Item> {
    (&item_enum.variants).into_iter()
        .map(|variant| filter_fields(&variant.fields, items))
        .flatten()
        .collect()
}