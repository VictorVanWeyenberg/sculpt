use std::collections::HashMap;
use proc_macro2::Ident;
use syn::{Attribute, Field, Fields, Item, ItemStruct, Meta, Token, Variant};
use syn::parse::{Parse, ParseStream};

#[derive(Default)]
pub struct Aliases<'a> {
    pub variant_aliases: HashMap<&'a Variant, HashMap<&'a Field, Ident>>,
    pub struct_aliases: HashMap<&'a ItemStruct, HashMap<&'a Field, Ident>>,
}

pub fn find_aliases(items: &Vec<Item>) -> Result<Aliases, String> {
    let mut aliases = Aliases {
        variant_aliases: HashMap::new(),
        struct_aliases: HashMap::new(),
    };
    for item in items {
        match item {
            Item::Struct(item_struct) => {
                if let Some(map) = find_aliases_item_struct(&item_struct)? {
                    aliases.struct_aliases.insert(item_struct, map);
                }
            }
            Item::Enum(item_enum) => {
                for variant in &item_enum.variants {
                    if let Some(map) = find_aliases_variant(variant)? {
                        aliases.variant_aliases.insert(variant, map);
                    }
                }
            }
            _ => continue
        }
    }
    Ok(aliases)
}

fn find_aliases_item_struct<'a>(item_struct: &'a ItemStruct) -> Result<Option<HashMap<&'a Field, Ident>>, String> {
    find_aliases_attributes(&item_struct.attrs, &item_struct.fields)
}

fn find_aliases_variant<'a>(variant: &'a Variant) -> Result<Option<HashMap<&'a Field, Ident>>, String> {
    find_aliases_attributes(&variant.attrs, &variant.fields)
}

fn find_aliases_attributes<'a>(attributes: &'a Vec<Attribute>, fields: &'a Fields) -> Result<Option<HashMap<&'a Field, Ident>>, String> {
    let aliases = attributes.iter()
        .map(|attr| find_field_to_ident_alias_from_attr(attr, &fields))
        .collect::<Result<Vec<Option<HashMap<&'a Field, Ident>>>, String>>()?;
    Ok(aliases.into_iter()
        .filter_map(|map| map)
        .reduce(|mut m1, m2| {
            m1.extend(m2);
            m1
        }))
}

fn find_field_to_ident_alias_from_attr<'a>(attr: &'a Attribute, fields: &'a Fields) -> Result<Option<HashMap<&'a Field, Ident>>, String> {
    if let Meta::List(meta_list) = &attr.meta {
        if !meta_list.path.is_ident("sculpt_alias") {
            println!("Not sculpt_alias");
            return Ok(None);
        }
        match syn::parse::Parser::parse2(AliasesVector::parse, meta_list.tokens.clone().into()) {
            Ok(aliases_vector) => {
                match aliases_vector.aliases.into_iter()
                    .map(|alias| get_field_with_name(alias, fields))
                    .collect::<Result<Vec<(&'a Field, Ident)>, String>>() {
                    Ok(pairs) => Ok(Some(pairs.into_iter().collect())),
                    Err(err) => Err(err)
                }
            }
            Err(_) => return Err("Could not parse arms.".to_string())
        }
    } else {
        println!("Not meta list");
        Ok(None)
    }
}

fn get_field_with_name(alias: Alias, fields: &Fields) -> Result<(&Field, Ident), String> {
    match fields {
        Fields::Unnamed(_) => return Err("Attribute sculpt_alias does not support unnamed fields.".to_string()),
        Fields::Unit => return Err("Attribute sculpt_alias does not support unit fields.".to_string()),
        _ => {}
    }
    fields.into_iter()
        .find(|field| field.ident.clone().is_some_and(|field| field == alias.field))
        .map(|field| (field, alias.alias.clone()))
        .ok_or(format!("Could not find field with name {}.", alias.field))
}

struct Alias {
    field: Ident,
    _arrow: Token![=>],
    alias: Ident,
}

impl Parse for Alias {
    fn parse(input: ParseStream) -> Result<Alias, syn::Error> {
        Ok(Alias {
            field: input.parse()?,
            _arrow: input.parse()?,
            alias: input.parse()?,
        })
    }
}

struct AliasesVector {
    aliases: Vec<Alias>,
}

impl Parse for AliasesVector {
    fn parse(input: ParseStream) -> Result<AliasesVector, syn::Error> {
        let mut aliases = Vec::new();

        while !input.is_empty() {
            let alias: Alias = input.parse()?;
            aliases.push(alias);

            // Consume the comma if present
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            } else {
                break;
            }
        }

        Ok(AliasesVector { aliases })
    }
}