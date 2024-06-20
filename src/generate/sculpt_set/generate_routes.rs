use std::collections::HashMap;
use itertools::Itertools;
use syn::{Field, Fields, Item, ItemEnum, Variant};
use crate::generate::sculpt_set::FieldOrVariant;

pub fn generate_routes<'a>(type_links: &HashMap<&'a Field, &'a Item>, fields: &'a Fields)
                           -> HashMap<&'a Field, Vec<FieldOrVariant<'a>>> {
    generate_routes_fields(type_links, vec![], fields).into_iter()
        .collect::<HashMap<&'a Field, Vec<FieldOrVariant<'a>>>>()
}

fn generate_routes_fields<'a>(type_links: &HashMap<&'a Field, &'a Item>, from: Vec<FieldOrVariant<'a>>, fields: &'a Fields)
                       -> Vec<(&'a Field, Vec<FieldOrVariant<'a>>)> {
    fields.iter()
        .map(|field| traverse_field(type_links, from.clone(), field))
        .concat()
}

fn traverse_field<'a>(type_links: &HashMap<&'a Field, &'a Item>, from: Vec<FieldOrVariant<'a>>, field: &'a Field)
                      -> Vec<(&'a Field, Vec<FieldOrVariant<'a>>)> {
    let mut from_clone = from.clone();
    from_clone.push(FieldOrVariant::Field(field));
    let mut routes = match type_links.get(field).unwrap() {
        Item::Enum(item_enum) => traverse_enum(type_links, from_clone, item_enum),
        Item::Struct(item_struct) => generate_routes_fields(type_links, from_clone, &item_struct.fields),
        _ => panic!("Item that's not an enum or a struct are not supposed to be present in a SculptSet.")
    };
    routes.push((field, from));
    routes
}

fn traverse_enum<'a>(type_links: &HashMap<&'a Field, &'a Item>, from: Vec<FieldOrVariant<'a>>, item_enum: &'a ItemEnum)
                     -> Vec<(&'a Field, Vec<FieldOrVariant<'a>>)> {
    item_enum.variants.iter()
        .map(|variant| traverse_variant(type_links, from.clone(), variant))
        .concat()
}

fn traverse_variant<'a>(type_links: &HashMap<&'a Field, &'a Item>, from: Vec<FieldOrVariant<'a>>, variant: &'a Variant)
                        -> Vec<(&'a Field, Vec<FieldOrVariant<'a>>)> {
    let mut from_clone = from.clone();
    from_clone.push(FieldOrVariant::Variant(variant));
    generate_routes_fields(type_links, from_clone, &variant.fields)
}