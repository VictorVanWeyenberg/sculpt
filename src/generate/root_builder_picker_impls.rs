use std::collections::HashMap;

use itertools::{EitherOrBoth, Itertools};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{Field, Fields, Item, ItemEnum, Variant};

use crate::generate::{get_field_ident_for_field, get_type_ident_for_field, OPTIONS};
use crate::generate::sculpt_set::SculptSet;
use crate::item_to_ident;

#[derive(Clone)]
enum FieldOrVariant {
    Field(Field),
    Variant(Variant),
}

impl FieldOrVariant {
    fn builder_field(&self) -> Ident {
        format_ident!("{}_builder", match self {
            FieldOrVariant::Field(field) => get_type_ident_for_field(field),
            FieldOrVariant::Variant(variant) => variant.ident.clone()
        }.to_string().to_lowercase())
    }
}

pub fn generate_root_builder_picker_impls(sculpt_set: &SculptSet) -> TokenStream {
    let routes = // Move to SculptSet?
        generate_routes(sculpt_set, vec![], &sculpt_set.root.fields)
        .into_iter()
        .collect::<HashMap<Field, Vec<FieldOrVariant>>>();
    let nexts = generate_nexts(sculpt_set, None, &sculpt_set.root.fields)
        .into_iter()
        .collect::<HashMap<Variant, Option<Item>>>();
    sculpt_set.type_links.iter()
        .filter_map(|(field, item)| match item {
            Item::Enum(item_enum) => Some((field, item_enum)),
            _ => None
        })
        .map(|(field, item_enum)| {
            generate_root_builder_picker_impl(sculpt_set, field, item_enum, routes.get(field).unwrap(), &nexts)
        })
        .reduce(|t1, t2| quote!(#t1 #t2))
        .unwrap_or(quote! {})
}

fn generate_nexts(sculpt_set: &SculptSet, next: Option<&Item>, fields: &Fields) -> Vec<(Variant, Option<Item>)> {
    fields.iter()
        .zip_longest(fields.iter().skip(1))
        .map(|pair| match pair {
            EitherOrBoth::Both(f1, f2) => (f1, sculpt_set.type_links.get(f2)),
            EitherOrBoth::Left(f1) => (f1, next),
            EitherOrBoth::Right(_) => panic!("Iterator of preceding fields is longer than the iterator of following fields.")
        })
        .map(|(field, next)| generate_nexts_field(sculpt_set, next, field))
        .concat()
}

fn generate_nexts_field(sculpt_set: &SculptSet, next: Option<&Item>, field: &Field) -> Vec<(Variant, Option<Item>)> {
    match sculpt_set.type_links.get(field).unwrap() {
        Item::Enum(item_enum) => item_enum.variants.iter()
            .map(|variant| if variant.fields.is_empty() {
                vec![(variant.clone(), next.cloned())]
            } else {
                let mut nexts = generate_nexts(sculpt_set, next, &variant.fields);
                let first_item = variant.fields.iter().next()
                    .map(|field| sculpt_set.type_links.get(field).unwrap())
                    .cloned();
                nexts.push((variant.clone(), first_item));
                nexts
            }).concat(),
        Item::Struct(item_struct) => generate_nexts(sculpt_set, next, &item_struct.fields),
        _ => vec![]
    }
}

fn generate_root_builder_picker_impl(sculpt_set: &SculptSet,
                                     field: &Field,
                                     item_enum: &ItemEnum,
                                     route: &Vec<FieldOrVariant>,
                                     nexts: &HashMap<Variant, Option<Item>>) -> TokenStream {
    let fulfill_method = generate_fulfill_method(sculpt_set, field, item_enum, route, nexts);
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
                           route: &Vec<FieldOrVariant>,
                           nexts: &HashMap<Variant, Option<Item>>) -> TokenStream {
    let options_type = format_ident!("{}{}", item_enum.ident, OPTIONS);
    let set_path = generate_set_path(sculpt_set, field, route);
    let match_arms = generate_match_arms(item_enum, nexts);
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

fn generate_match_arms(item_enum: &ItemEnum,
                       nexts: &HashMap<Variant, Option<Item>>) -> Vec<TokenStream> {
    let options_enum = format_ident!("{}{}", item_enum.ident, OPTIONS);
    item_enum.variants.iter()
        .map(|variant| {
            let variant_ident = &variant.ident;
            if let Some(item) = nexts.get(variant).unwrap() {
                let item_ident = item_to_ident(&item).unwrap();
                let pick_method = format_ident!("pick_{}", item_ident.to_string().to_lowercase());
                quote!(#options_enum::#variant_ident => self.callbacks.#pick_method(self))
            } else {
                quote!(#options_enum::#variant_ident => {})
            }
        }).collect()
}

fn generate_routes(sculpt_set: &SculptSet, from: Vec<FieldOrVariant>, fields: &Fields)
                   -> Vec<(Field, Vec<FieldOrVariant>)> {
    fields.iter()
        .map(|field| traverse_field(sculpt_set, from.clone(), field))
        .concat()
}

fn traverse_field(sculpt_set: &SculptSet, from: Vec<FieldOrVariant>, field: &Field)
    -> Vec<(Field, Vec<FieldOrVariant>)> {
    let mut from_clone = from.clone();
    from_clone.push(FieldOrVariant::Field(field.clone()));
    let mut routes = match sculpt_set.type_links.get(field).unwrap() {
        Item::Enum(item_enum) => traverse_enum(sculpt_set, from_clone, item_enum),
        Item::Struct(item_struct) => generate_routes(sculpt_set, from_clone, &item_struct.fields),
        _ => panic!("Item that's not an enum or a struct are not supposed to be present in a SculptSet.")
    };
    routes.push((field.clone(), from));
    routes
}

fn traverse_enum(sculpt_set: &SculptSet, from: Vec<FieldOrVariant>, item_enum: &ItemEnum) -> Vec<(Field, Vec<FieldOrVariant>)> {
    item_enum.variants.iter()
        .map(|variant| traverse_variant(sculpt_set, from.clone(), variant))
        .concat()
}

fn traverse_variant(sculpt_set: &SculptSet, from: Vec<FieldOrVariant>, variant: &Variant) -> Vec<(Field, Vec<FieldOrVariant>)> {
    let mut from_clone = from.clone();
    from_clone.push(FieldOrVariant::Variant(variant.clone()));
    generate_routes(sculpt_set, from_clone, &variant.fields)
}