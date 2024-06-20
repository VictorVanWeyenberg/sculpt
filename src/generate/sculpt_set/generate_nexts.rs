use std::collections::HashMap;
use itertools::{EitherOrBoth, Itertools};
use syn::{Field, Fields, Item, Variant};

pub fn generate_nexts<'a>(type_links: &HashMap<&'a Field, &'a Item>, fields: &'a Fields) -> HashMap<&'a Variant, Option<&'a Item>> {
    generate_nexts_fields(type_links, None, fields).into_iter()
        .collect::<HashMap<&'a Variant, Option<&'a Item>>>()
}

pub fn generate_nexts_fields<'a>(type_links: &HashMap<&'a Field, &'a Item>, next: Option<&'a Item>, fields: &'a Fields) -> Vec<(&'a Variant, Option<&'a Item>)> {
    fields.into_iter()
        .zip_longest(fields.into_iter().skip(1))
        .map(move |pair| match pair {
            EitherOrBoth::Both(f1, f2) => (f1, type_links.get(f2).cloned()),
            EitherOrBoth::Left(f1) => (f1, next),
            EitherOrBoth::Right(_) => panic!("Iterator of preceding fields is longer than the iterator of following fields.")
        })
        .map(|(field, next)| generate_nexts_field(type_links, next, field))
        .concat()
}

fn generate_nexts_field<'a>(type_links: &HashMap<&'a Field, &'a Item>, next: Option<&'a Item>, field: &'a Field) -> Vec<(&'a Variant, Option<&'a Item>)> {
    match type_links.get(field).unwrap() {
        Item::Enum(item_enum) => item_enum.variants.iter()
            .map(|variant| generate_nexts_variant(type_links, next, variant)).concat(),
        Item::Struct(item_struct) => generate_nexts_fields(type_links, next, &item_struct.fields),
        _ => vec![]
    }
}

fn generate_nexts_variant<'a>(type_links: &HashMap<&'a Field, &'a Item>, next: Option<&'a Item>, variant: &'a Variant) -> Vec<(&'a Variant, Option<&'a Item>)> {
    if variant.fields.is_empty() {
        vec![(variant, next)]
    } else {
        let mut nexts = generate_nexts_fields(type_links, next, &variant.fields);
        let first_item = variant.fields.iter().next()
            .map(|field| type_links.get(field).unwrap())
            .cloned();
        nexts.push((variant, first_item));
        nexts
    }
}