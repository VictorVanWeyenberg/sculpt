use std::collections::HashMap;
use itertools::{EitherOrBoth, Itertools};

use proc_macro2::Ident;
use quote::format_ident;
use syn::{Field, Fields, Item, ItemEnum, ItemStruct, Type, Variant};
use crate::generate::get_type_ident_for_field;

pub struct SculptSet {
    pub root: ItemStruct,
    pub type_links: HashMap<Field, Item>,
    pub routes: HashMap<Field, Vec<FieldOrVariant>>,
    pub nexts: HashMap<Variant, Option<Item>>,
    pub first: ItemEnum
}

impl SculptSet {
    pub fn new(items: Vec<Item>) -> Result<SculptSet, String> {
        let root = items.iter()
            .find_map(has_sculpt_attribute)
            .ok_or("Could not find `sculpt` attribute in file.".to_string())?;
        let type_links = link_item_struct(&items, &root)?.into_iter()
            .collect();
        let routes = generate_routes(&type_links, vec![], &root.fields)
            .into_iter()
            .collect::<HashMap<Field, Vec<FieldOrVariant>>>();
        let nexts = generate_nexts(&type_links, None, &root.fields)
            .into_iter()
            .collect::<HashMap<Variant, Option<Item>>>();
        let first = find_first(&root, &type_links);
        Ok(SculptSet { root, type_links, routes, nexts , first})
    }

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

#[derive(Clone)]
pub enum FieldOrVariant {
    Field(Field),
    Variant(Variant),
}

impl FieldOrVariant {
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

// -------------------------------------------------------------------------------------------------
// Routes helper methods
// -------------------------------------------------------------------------------------------------

fn generate_routes(type_links: &HashMap<Field, Item>, from: Vec<FieldOrVariant>, fields: &Fields)
                   -> Vec<(Field, Vec<FieldOrVariant>)> {
    fields.iter()
        .map(|field| traverse_field(type_links, from.clone(), field))
        .concat()
}

fn traverse_field(type_links: &HashMap<Field, Item>, from: Vec<FieldOrVariant>, field: &Field)
                  -> Vec<(Field, Vec<FieldOrVariant>)> {
    let mut from_clone = from.clone();
    from_clone.push(FieldOrVariant::Field(field.clone()));
    let mut routes = match type_links.get(field).unwrap() {
        Item::Enum(item_enum) => traverse_enum(type_links, from_clone, item_enum),
        Item::Struct(item_struct) => generate_routes(type_links, from_clone, &item_struct.fields),
        _ => panic!("Item that's not an enum or a struct are not supposed to be present in a SculptSet.")
    };
    routes.push((field.clone(), from));
    routes
}

fn traverse_enum(type_links: &HashMap<Field, Item>, from: Vec<FieldOrVariant>, item_enum: &ItemEnum) -> Vec<(Field, Vec<FieldOrVariant>)> {
    item_enum.variants.iter()
        .map(|variant| traverse_variant(type_links, from.clone(), variant))
        .concat()
}

fn traverse_variant(type_links: &HashMap<Field, Item>, from: Vec<FieldOrVariant>, variant: &Variant) -> Vec<(Field, Vec<FieldOrVariant>)> {
    let mut from_clone = from.clone();
    from_clone.push(FieldOrVariant::Variant(variant.clone()));
    generate_routes(type_links, from_clone, &variant.fields)
}

// -------------------------------------------------------------------------------------------------
// Nexts helper methods
// -------------------------------------------------------------------------------------------------

fn generate_nexts(type_links: &HashMap<Field, Item>, next: Option<&Item>, fields: &Fields) -> Vec<(Variant, Option<Item>)> {
    fields.iter()
        .zip_longest(fields.iter().skip(1))
        .map(|pair| match pair {
            EitherOrBoth::Both(f1, f2) => (f1, type_links.get(f2)),
            EitherOrBoth::Left(f1) => (f1, next),
            EitherOrBoth::Right(_) => panic!("Iterator of preceding fields is longer than the iterator of following fields.")
        })
        .map(|(field, next)| generate_nexts_field(type_links, next, field))
        .concat()
}

fn generate_nexts_field(type_links: &HashMap<Field, Item>, next: Option<&Item>, field: &Field) -> Vec<(Variant, Option<Item>)> {
    match type_links.get(field).unwrap() {
        Item::Enum(item_enum) => item_enum.variants.iter()
            .map(|variant| generate_nexts_variant(type_links, next, variant)).concat(),
        Item::Struct(item_struct) => generate_nexts(type_links, next, &item_struct.fields),
        _ => vec![]
    }
}

fn generate_nexts_variant(type_links: &HashMap<Field, Item>, next: Option<&Item>, variant: &Variant) -> Vec<(Variant, Option<Item>)> {
    if variant.fields.is_empty() {
        vec![(variant.clone(), next.cloned())]
    } else {
        let mut nexts = generate_nexts(type_links, next, &variant.fields);
        let first_item = variant.fields.iter().next()
            .map(|field| type_links.get(field).unwrap())
            .cloned();
        nexts.push((variant.clone(), first_item));
        nexts
    }
}

// -------------------------------------------------------------------------------------------------
// Find first helper methods
// -------------------------------------------------------------------------------------------------

fn find_first(root: &ItemStruct, type_links: &HashMap<Field, Item>) -> ItemEnum {
    root.fields.iter()
        .find_map(|field| find_first_from_field(type_links, field))
        .unwrap()
}

fn find_first_from_field(type_links: &HashMap<Field, Item>, field: &Field) -> Option<ItemEnum> {
    match type_links.get(field).unwrap() {
        Item::Enum(item_enum) => Some(item_enum.clone()),
        Item::Struct(item_struct) => Some(find_first(item_struct, type_links)),
        _ => None
    }
}