use std::collections::HashMap;
use syn::{Field, Item, ItemEnum, ItemStruct};

pub fn find_first<'a>(root: &'a ItemStruct, type_links: &HashMap<&'a Field, &'a Item>) -> Option<&'a ItemEnum> {
    root.fields.iter()
        .find_map(|field| find_first_from_field(type_links, field))
}

fn find_first_from_field<'a>(type_links: &HashMap<&'a Field, &'a Item>, field: &'a Field) -> Option<&'a ItemEnum> {
    match type_links.get(field).unwrap() {
        Item::Enum(item_enum) => Some(item_enum),
        Item::Struct(item_struct) => find_first(item_struct, type_links),
        _ => None
    }
}