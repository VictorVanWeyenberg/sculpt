use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use proc_macro2::{Ident, TokenStream};
use quote::quote;
use rust_format::{Formatter, RustFmt};
use syn::{Item, ItemStruct};

use crate::generate::generate;

const OPTIONS: &str = "Options";

mod type_link;
mod generate;

pub fn build(path: PathBuf, root_dir: &Path, out_dir: &Path) {
    let source = root_dir.join(&path);
    let destination = out_dir.join(&path);
    let ast = to_ast(source);
    let dt_tokens = generate(ast.items.clone()).unwrap_or(quote! {});
    let tl_tokens = type_link::to_type_linker(ast).extrapolate();
    let tokens = quote! {
        #dt_tokens
        #(#tl_tokens )*
    };
    write_token_stream_to_file(tokens, destination);
}

fn write_token_stream_to_file(tokens: TokenStream, path: PathBuf) {
    let code = format!("{}", tokens);
    let code = RustFmt::default().format_str(code).unwrap();
    let parent = path.parent().unwrap();
    fs::create_dir_all(parent).unwrap();
    match File::create(path) {
        Ok(mut file) => file.write_all(code.as_bytes()).unwrap(),
        Err(error) => println!("{}", error),
    }
}

fn to_ast(path: PathBuf) -> syn::File {
    let mut file = File::open(&path).expect(&format!("Cannot open file. {:?}", path));
    let mut content = String::new();
    file.read_to_string(&mut content)
        .expect(&format!("Cannot read contents. {:?}", path));
    let file = syn::parse_file(&content).expect(&format!("Cannot parse file. {:?}", path));
    file
}

fn is_item_struct_root(item_struct: &ItemStruct) -> bool {
    for attr in &item_struct.attrs {
        if attr.path().is_ident("sculpt") {
            return true
        }
    }
    false
}

fn item_to_ident(item: &Item) -> Option<Ident> {
    match item {
        Item::Struct(item_struct) => Some(item_struct.ident.clone()),
        Item::Enum(item_enum) => Some(item_enum.ident.clone()),
        _ => None
    }
}