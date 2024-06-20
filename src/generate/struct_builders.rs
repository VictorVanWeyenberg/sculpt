use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{Field, Fields, ItemStruct};

use crate::generate::{field_to_builder_call, generate_builder_field, get_field_ident_for_field, get_type_ident_for_field, is_field_sculptable, OPTIONS, SculptSet};

pub fn generate_struct_builders(sculpt_set: &SculptSet) -> TokenStream {
    let struct_builders = generate_builders_and_impls(sculpt_set);
    let root_builder = generate_root_builder(sculpt_set);
    let root_builder_impl = generate_root_builder_impl(sculpt_set);
    let root_struct_impl = generate_root_struct_impl(&sculpt_set.root);
    quote! {
        #struct_builders
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
    let first_field_pick_method = get_first_field_pick_method(sculpt_set);
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

fn get_first_field_pick_method(sculpt_set: &SculptSet) -> TokenStream {
    let method = format_ident!("pick_{}", sculpt_set.first().unwrap().ident.to_string().to_lowercase());
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

fn generate_builders_and_impls(sculpt_set: &SculptSet) -> TokenStream {
    sculpt_set.get_all_structs().into_iter()
        .map(|item_struct| generate_builder_and_impl(sculpt_set, item_struct))
        .reduce(|t1, t2| quote!(#t1 #t2))
        .unwrap_or(quote! {})
}

fn generate_builder_and_impl(sculpt_set: &SculptSet, item_struct: &ItemStruct) -> TokenStream {
    let builder = generate_builder(sculpt_set, item_struct);
    let builder_impl = generate_builder_impl(sculpt_set, item_struct);
    quote! {
        #builder
        #builder_impl
    }
}

fn generate_builder(sculpt_set: &SculptSet, item_struct: &ItemStruct) -> TokenStream {
    let builder = format_ident!("{}Builder", item_struct.ident);
    let fields = item_struct.fields.iter()
        .map(|field| generate_struct_builder_field(sculpt_set, field))
        .collect::<Vec<TokenStream>>();
    quote! {
        #[derive(Default)]
        struct #builder {
            #(#fields,)*
        }
    }
}

fn generate_struct_builder_field(sculpt_set: &SculptSet, field: &Field) -> TokenStream {
    if is_field_sculptable(sculpt_set, field) {
        let builder_field_ident =
            format_ident!("{}_builder", get_type_ident_for_field(field).to_string().to_lowercase());
        let builder_type_ident = format_ident!("{}Builder", get_type_ident_for_field(field));
        quote!(#builder_field_ident: #builder_type_ident)
    } else {
        let field_ident = get_field_ident_for_field(field);
        let type_ident = format_ident!("{}{}", get_type_ident_for_field(field), OPTIONS);
        quote!(#field_ident: Option<#type_ident>)
    }
}

fn generate_builder_impl(sculpt_set: &SculptSet, item_struct: &ItemStruct) -> TokenStream {
    let struct_ident = &item_struct.ident;
    let builder = format_ident!("{}Builder", struct_ident);
    let variables = item_struct.fields.iter()
        .map(|field| generate_variables(sculpt_set, field))
        .collect::<Vec<TokenStream>>();
    let constructor = generate_constructor(item_struct);
    quote! {
        impl #builder {
            fn build(self) -> #struct_ident {
                #(#variables;)*
                #constructor
            }
        }
    }
}

fn generate_variables(sculpt_set: &SculptSet, field: &Field) -> TokenStream {
    let field_ident = get_field_ident_for_field(field);
    if is_field_sculptable(sculpt_set, field) {
        let builder_field_ident =
            format_ident!("{}_builder", get_type_ident_for_field(field).to_string().to_lowercase());
        quote!(let #field_ident = self.#builder_field_ident.build())
    } else {
        quote!(let #field_ident = self.#field_ident.unwrap().into())
    }
}

fn generate_constructor(item_struct: &ItemStruct) -> TokenStream {
    let named = match &item_struct.fields {
        Fields::Named(_) => true,
        Fields::Unnamed(_) => false,
        Fields::Unit => panic!("Generating constructor for unit struct.")
    };
    let fields = item_struct.fields.iter()
        .map(get_field_ident_for_field)
        .collect::<Vec<Ident>>();
    let struct_ident = &item_struct.ident;
    if named {
        quote! (#struct_ident { #(#fields,)* })
    } else {
        quote! (#struct_ident ( #(#fields,)* ))
    }
}
