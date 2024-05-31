use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{Field, ItemStruct};

use crate::sculpt_set::{field_to_builder_call, generate_builder_field, get_field_ident_for_field, get_type_ident_for_field, is_field_sculptable, SculptSet};

pub fn generate_struct_builders(sculpt_set: &SculptSet) -> TokenStream {
    let root_builder = generate_root_builder(sculpt_set);
    let root_builder_impl = generate_root_builder_impl(sculpt_set);
    let root_struct_impl = generate_root_struct_impl(&sculpt_set.root);
    quote! {
        #root_builder
        #root_builder_impl
        #root_struct_impl
    }
}

fn generate_root_builder(sculpt_set: &SculptSet) -> TokenStream {
    let builder_name = format_ident!("{}Builder", sculpt_set.root.ident);
    let callbacks_name = format_ident!("{}Callbacks", builder_name);
    let fields = sculpt_set.root.fields.iter()
        .map(|field| generate_builder_field(sculpt_set, field))
        .collect::<Vec<TokenStream>>();
    quote! {
        pub struct #builder_name<'a, T: #callbacks_name> {
            #(#fields,)*
            callbacks: &'a T
        }
    }
}

fn generate_root_builder_impl(sculpt_set: &SculptSet) -> TokenStream {
    let root = &sculpt_set.root;
    let sculptable_ident = &root.ident;
    let builder_name = format_ident!("{}Builder", sculptable_ident);
    let callbacks_name = format_ident!("{}Callbacks", builder_name);
    let field_initializers = root.fields.iter()
        .map(|field| field_to_field_initializer(sculpt_set, field))
        .collect::<Vec<TokenStream>>();
    let first_field_pick_method = get_first_field_pick_method(root);
    let field_builders = root.fields.iter()
        .map(|field| field_to_builder_call(sculpt_set, sculptable_ident, field))
        .collect::<Vec<TokenStream>>();
    let field_names: Vec<Ident> = root.fields.iter()
        .map(get_field_ident_for_field)
        .collect::<Vec<Ident>>();
    quote! {
        impl<'a, T: #callbacks_name> #builder_name<'a, T> {
            pub fn new(t: &'a mut T) -> #builder_name<T> {
                #builder_name {
                    #(#field_initializers,)*
                    callbacks: t
                }
            }

            pub fn build(mut self) -> #sculptable_ident {
                self.callbacks.#first_field_pick_method(&mut self);
                #(#field_builders;)*
                #sculptable_ident { #(#field_names,)* }
            }
        }
    }
}

fn get_first_field_pick_method(item_struct: &ItemStruct) -> TokenStream {
    let field = item_struct.fields.iter()
        .next()
        .expect("No fields for root sculptor.");
    let type_name = get_type_ident_for_field(field);
    let method = format_ident!("pick_{}", type_name.to_string().to_lowercase());
    quote! { #method }
}

fn field_to_field_initializer(sculpt_set: &SculptSet, field: &Field) -> TokenStream {
    if is_field_sculptable(sculpt_set, field) {
        let type_ident = get_type_ident_for_field(field);
        let builder_name = format_ident!("{}_builder", type_ident.to_string().to_lowercase());
        let builder_type = format_ident!("{}Builder", type_ident);
        quote! { #builder_name: #builder_type::default() }
    } else {
        let option_name = format_ident!("{}", get_field_ident_for_field(field));
        quote! { #option_name: None }
    }
}

fn generate_root_struct_impl(item_struct: &ItemStruct) -> TokenStream {
    let sculptable_ident = &item_struct.ident;
    let builder_name = format_ident!("{}Builder", sculptable_ident);
    let callbacks_name = format_ident!("{}Callbacks", builder_name);
    quote! {
        impl #sculptable_ident {
            pub fn build<T: #callbacks_name>(t: &mut T) -> #sculptable_ident {
                #builder_name::<T>::new(t).build()
            }
        }
    }
}
