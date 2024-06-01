use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use proc_macro2::TokenStream;
use quote::quote;
use rust_format::{Formatter, RustFmt};

use crate::generate::generate;

mod generate;

pub fn build(path: PathBuf, root_dir: &Path, out_dir: &Path) {
    let source = root_dir.join(&path);
    let destination = out_dir.join(&path);
    let ast = to_ast(source);
    let tokens = generate(ast.items.clone()).unwrap_or(quote! {});
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