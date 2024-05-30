use std::collections::HashMap;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{Item, Variant};
use crate::{item_to_ident, OPTIONS};
use crate::type_link::FieldItemOrVariantIdent;

pub struct LinkCompiler {
    root: Ident,
    links: HashMap<Vec<FieldItemOrVariantIdent>, HashMap<Variant, Option<Item>>>,
}

impl LinkCompiler {
    pub fn new(root: Ident, links: HashMap<Vec<FieldItemOrVariantIdent>, HashMap<Variant, Option<Item>>>) -> LinkCompiler {
        LinkCompiler { root, links }
    }

    pub fn compile(self) -> Vec<TokenStream> {
        self.links.iter()
            .map(|(path, variant_to_next)| {
                self.entry_to_impl_block(path, variant_to_next)
            })
            .collect::<Vec<TokenStream>>()
    }

    fn entry_to_impl_block(&self, path: &Vec<FieldItemOrVariantIdent>, variant_to_next: &HashMap<Variant, Option<Item>>) -> TokenStream {
        let enum_is_sculptable = variant_to_next.iter().any(|(variant, _)| !variant.fields.is_empty());
        let last_item_type = LinkCompiler::get_last_item_type(&path);
        let path = LinkCompiler::compile_path(&path, enum_is_sculptable);
        let arms = variant_to_next.iter()
            .map(|(variant, next)| LinkCompiler::compile_arm(&last_item_type, variant, next))
            .collect::<Vec<TokenStream>>();
        let fulfill_method = LinkCompiler::compile_fulfill_method(&last_item_type, path, arms);
        LinkCompiler::compile_impl_block(&self.root, &last_item_type, fulfill_method)
    }

    fn get_last_item_type(path: &Vec<FieldItemOrVariantIdent>) -> Ident {
        path.iter()
            .rev()
            .find_map(|ident| match ident {
                FieldItemOrVariantIdent::FieldItemIdent { item_ident, .. } => Some(item_ident),
                FieldItemOrVariantIdent::VariantIdent { .. } => None
            })
            .unwrap()
            .clone()
    }

    fn compile_path(path: &Vec<FieldItemOrVariantIdent>, sculptable: bool) -> TokenStream {
        let (last, builders) = path.split_last().unwrap();
        let mut builders = builders.iter()
            .map(|b| b.builder_ident())
            .collect::<Vec<Ident>>();
        let last = if sculptable {
            let last_builder = last.builder_ident();
            builders.push(last_builder);
            last.item_as_field_ident()
        } else {
            last.field_ident()
        };
        quote! {
            self.#(#builders.)*#last = Some(requirement.clone());
        }
    }

    fn compile_arm(enum_type: &Ident, variant: &Variant, next: &Option<Item>) -> TokenStream {
        let options_enum_type = format_ident!("{}{}", enum_type, OPTIONS);
        let variant_ident = variant.ident.clone();
        let pick_next_call = if let Some(next) = next {
            let pick_next = format_ident!("pick_{}", item_to_ident(next).unwrap().to_string().to_lowercase());
            quote!(self.callbacks.#pick_next(self))
        } else {
            quote!({})
        };
        quote! {
            #options_enum_type::#variant_ident => #pick_next_call
        }
    }

    fn compile_fulfill_method(enum_type: &Ident, path: TokenStream, arms: Vec<TokenStream>) -> TokenStream {
        let options_enum_type = format_ident!("{}{}", enum_type, OPTIONS);
        quote! {
            fn fulfill(&mut self, requirement: &#options_enum_type) {
                #path
                match requirement {
                    #(#arms,)*
                }
            }
        }
    }

    fn compile_impl_block(root_ident: &Ident, enum_type: &Ident, fulfill_method: TokenStream) -> TokenStream {
        let root_builder = format_ident!("{}Builder", root_ident);
        let root_builder_callbacks = format_ident!("{}Callbacks", root_builder);
        let enum_picker = format_ident!("{}Picker", enum_type);
        quote! {
            impl<'a, T: #root_builder_callbacks> #enum_picker for #root_builder<'a, T> {
                #fulfill_method
            }
        }
    }
}