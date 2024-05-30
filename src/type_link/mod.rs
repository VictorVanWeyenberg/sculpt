use std::collections::HashMap;
use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::format_ident;
use syn::{Field, Fields, Item, ItemEnum, Type, Variant};
use crate::{is_item_struct_root, item_to_ident};
use crate::type_link::link_compile::LinkCompiler;

mod link_compile;

pub fn to_type_linker(ast: syn::File) -> TypeLinker {
    let items = ast.items;
    let root = items.iter()
        .find(|item| match item {
            Item::Struct(item_struct) => is_item_struct_root(item_struct),
            _ => false
        }).expect("").clone();
    TypeLinker::new(items, root)
}

pub struct TypeLinker {
    root: Item,
    items: Vec<Item>,
    links: HashMap<Vec<FieldItemOrVariantIdent>, HashMap<Variant, Option<Item>>>,
}

#[derive(Clone, Hash, PartialEq, Eq)]
enum FieldItemOrVariantIdent {
    FieldItemIdent {
        field_ident: Ident,
        item_ident: Ident
    },
    VariantIdent {
        variant_ident: Ident
    }
}

impl FieldItemOrVariantIdent {
    fn builder_ident(&self) -> Ident {
        format_ident!("{}_builder", match self {
            FieldItemOrVariantIdent::FieldItemIdent { item_ident, .. } => item_ident,
            FieldItemOrVariantIdent::VariantIdent { variant_ident } => variant_ident
        }.to_string().to_lowercase())
    }

    fn field_ident(&self) -> Ident {
        format_ident!("{}", match self {
            FieldItemOrVariantIdent::FieldItemIdent { field_ident, .. } => field_ident,
            FieldItemOrVariantIdent::VariantIdent { variant_ident } => variant_ident
        }.to_string().to_lowercase())
    }

    fn item_as_field_ident(&self) -> Ident {
        format_ident!("{}", match self {
            FieldItemOrVariantIdent::FieldItemIdent { item_ident, .. } => item_ident,
            FieldItemOrVariantIdent::VariantIdent { .. } => panic!("Requesting item ident from variant.")
        }.to_string().to_lowercase())
    }
}

impl TypeLinker {
    fn new(items: Vec<Item>, root: Item) -> TypeLinker {
        TypeLinker { items, root, links: HashMap::default() }
    }

    fn get_item_by_field(&self, field: &Field) -> Item {
        let field_ident = match &field.ty {
            Type::Path(type_path) => type_path.path.get_ident().unwrap().clone(),
            _ => panic!("Cannot get type ident of non path field type.")
        };
        self.items.iter()
            .find(|item| match item {
                Item::Enum(item_enum) => item_enum.ident == field_ident,
                Item::Struct(item_struct) => item_struct.ident == field_ident,
                _ => false
            })
            .expect(&format!("Cannot find item with type {}.", field_ident))
            .clone()
    }

    pub fn extrapolate(mut self) -> Vec<TokenStream> {
        self.extrapolate_item(vec![], self.root.clone(), None);
        LinkCompiler::new(item_to_ident(&self.root).unwrap(), self.links).compile()
    }

    fn extrapolate_item(&mut self, from: Vec<FieldItemOrVariantIdent>, item: Item, next: Option<Item>) {
        match item {
            Item::Enum(item_enum) => {
                self.extrapolate_enum(from, item_enum, next)
            }
            Item::Struct(item_struct) => {
                self.extrapolate_fields(from, item_struct.fields, next);
            }
            _ => {}
        };
    }

    fn extrapolate_enum(&mut self, from: Vec<FieldItemOrVariantIdent>, item_enum: ItemEnum, next: Option<Item>) {
        item_enum.variants.pairs()
            .map(|pair| pair.into_value().clone())
            .for_each(|variant| self.store_link(from.clone(), variant, next.clone()));
    }

    fn store_link(&mut self, from: Vec<FieldItemOrVariantIdent>, variant: Variant, next: Option<Item>) {
        let mut from_clone = from.clone();
        from_clone.push(FieldItemOrVariantIdent::VariantIdent { variant_ident: variant.ident.clone() });
        let next = self.extrapolate_fields(from_clone, variant.fields.clone(), next.clone());
        let mut inner_map = self.links.remove(&from).unwrap_or(HashMap::new());
        inner_map.insert(variant, next);
        self.links.insert(from.clone(), inner_map);
    }

    fn extrapolate_fields(&mut self, mut from: Vec<FieldItemOrVariantIdent>, fields: Fields, next: Option<Item>) -> Option<Item> {
        if fields.is_empty() {
            return next;
        }
        let fields = match fields {
            Fields::Named(fields_named) => fields_named.named,
            Fields::Unnamed(fields_unnamed) => fields_unnamed.unnamed,
            Fields::Unit => panic!("Unsupported unit field.")
        };
        let first_field = fields.first().unwrap().clone();
        let last_field = fields.last().unwrap().clone();
        let last_item = self.get_item_by_field(&last_field);
        fields.into_pairs()
            .map(|pair| pair.into_value())
            .tuple_windows()
            .for_each(|(f1, f2)| self.link_fields(from.clone(), f1, f2));
        let field_item_ident = FieldItemOrVariantIdent::FieldItemIdent {
            field_ident: TypeLinker::struct_field_to_builder_field_name(&last_field),
            item_ident: item_to_ident(&self.get_item_by_field(&last_field)).unwrap(),
        };
        from.push(field_item_ident);
        self.extrapolate_item(from, last_item, next);
        Some(self.get_item_by_field(&first_field))
    }

    fn link_fields(&mut self, mut from: Vec<FieldItemOrVariantIdent>, f1: Field, f2: Field) {
        let f1_ident = TypeLinker::struct_field_to_builder_field_name(&f1);
        let i1 = self.get_item_by_field(&f1);
        let i2 = self.get_item_by_field(&f2);
        from.push(FieldItemOrVariantIdent::FieldItemIdent { field_ident: f1_ident, item_ident: item_to_ident(&i1).unwrap() });
        self.extrapolate_item(from, i1, Some(i2));
    }

    fn struct_field_to_builder_field_name(field: &Field) -> Ident {
        match &field.ident {
            None => {
                match &field.ty {
                    Type::Path(type_path) => match type_path.path.get_ident() {
                        None => panic!("Path type has no ident."),
                        Some(ident) => ident.clone()
                    },
                    _ => panic!("Field type is no path type")
                }
            }
            Some(ident) => ident.clone()
        }
    }
}