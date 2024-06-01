use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{Field, Item, Type};

const OPTIONS: &str = "Options";

use sculpt_set::SculptSet;
use crate::generate::callback_trait::generate_callback_trait;
use crate::generate::options_enums::generate_options_enums;
use crate::generate::pickable_builders::generate_pickable_builders;
use crate::generate::picker_traits::generate_picker_traits;
use crate::generate::root_builder_picker_impls::generate_root_builder_picker_impls;
use crate::generate::struct_builders::generate_struct_builders;
use crate::generate::variant_builders::generate_variant_builders;

mod sculpt_set;
mod callback_trait;
mod picker_traits;
mod options_enums;
mod variant_builders;
mod pickable_builders;
mod struct_builders;
mod root_builder_picker_impls;

pub fn generate(items: Vec<Item>) -> Result<TokenStream, String> {
    SculptSet::new(items).map(generate_tokens)
}

fn generate_tokens(sculpt_set: SculptSet) -> TokenStream {
    [
        generate_callback_trait,
        generate_picker_traits,
        generate_options_enums,
        generate_variant_builders,
        generate_pickable_builders,
        generate_struct_builders,
        generate_root_builder_picker_impls
    ].iter()
        .map(|f| f(&sculpt_set))
        .reduce(|t1, t2| quote!(#t1 #t2))
        .unwrap()
}

fn generate_builder_field(sculpt_set: &SculptSet, field: &Field) -> TokenStream {
    let field_type_ident = get_type_ident_for_field(field);
    if is_field_sculptable(sculpt_set, field) {
        let builder_field = format_ident!("{}_builder", field_type_ident.to_string().to_lowercase());
        let builder_type = format_ident!("{}Builder", field_type_ident);
        quote! {
            #builder_field: #builder_type
        }
    } else {
        let option_field = get_field_ident_for_field(field);
        let option_type = format_ident!("{}{}", field_type_ident, OPTIONS);
        quote! {
            #option_field: Option<#option_type>
        }
    }
}

fn is_field_sculptable(sculpt_set: &SculptSet, field: &Field) -> bool {
    match sculpt_set.type_links.get(field).unwrap() {
        Item::Struct(_) => true,
        Item::Enum(item_enum) => item_enum.variants.iter()
            .any(|variant| !variant.fields.is_empty()),
        _ => panic!("Field references something that's not an enum or a struct. Not supported.")
    }
}

fn get_type_ident_for_field(field: &Field) -> Ident {
    match &field.ty {
        Type::Path(type_path) => type_path.path.get_ident()
            .expect("Could not get identifier of field path type.").clone(),
        _ => panic!("None path types are not supported in the sculpt tree.")
    }
}

fn get_field_ident_for_field(field: &Field) -> Ident {
    let ident = field.ident.clone().unwrap_or(match &field.ty {
        Type::Path(type_path) => type_path.path.get_ident()
            .expect("Could not get identifier of field path type.").clone(),
        _ => panic!("None path types are not supported in the sculpt tree.")
    });
    format_ident!("{}", ident.to_string().to_lowercase())
}

fn field_to_builder_call(sculpt_set: &SculptSet, item_ident: &Ident, field: &Field) -> TokenStream {
    let variable = get_field_ident_for_field(field);
    if is_field_sculptable(sculpt_set, field) {
        let field_type_ident = get_type_ident_for_field(field);
        let builder_field = format_ident!("{}_builder", field_type_ident.to_string().to_lowercase());
        quote! {
            let #variable = self.#builder_field.build()
        }
    } else {
        let builder_type = format_ident!("{}Builder", item_ident);
        let message = format!("Field {} not set in {}.", variable, builder_type);
        quote! {
            let #variable = self.#variable.expect(#message).into()
        }
    }
}