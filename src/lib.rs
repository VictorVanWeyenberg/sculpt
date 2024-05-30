use std::collections::HashMap;
use std::fmt::Debug;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use itertools::Itertools;
use proc_macro2::{Ident, TokenStream, TokenTree};
use quote::{format_ident, quote};
use rust_format::{Formatter, RustFmt};
use syn::{Attribute, Field, Fields, Item, ItemEnum, ItemStruct, Type, Variant};

const OPTIONS: &str = "Options";

// =================================================================================================
// Dependency Tree Domain
// =================================================================================================

#[derive(Debug)]
struct DependencyTree {
    nodes: Vec<DependencyNode>,
}

impl DependencyTree {
    fn new(nodes: Vec<DependencyNode>) -> DependencyTree {
        DependencyTree { nodes }
    }

    fn find_root(&self) -> Option<&DependencyNode> {
        let mut possible_roots = self.nodes.iter()
            .filter(|node| node.d_type.is_root());
        let number_of_roots = possible_roots.clone().count();
        if number_of_roots == 0 {
            None
        } else if number_of_roots > 1 {
            panic!("Only one root Sculptor allowed per file.");
        } else {
            possible_roots.next()
        }
    }
}

#[derive(Debug)]
struct DependencyNode {
    d_type: DependencyType,
    formatter: IdentFormatter,
}

impl DependencyNode {
    fn new(name: Ident, d_type: DependencyType) -> DependencyNode {
        let formatter = IdentFormatter(name.clone());
        DependencyNode {
            d_type,
            formatter,
        }
    }

    fn generate_picker_method(&self) -> Option<TokenStream> {
        if !self.d_type.is_pickable() {
            return None;
        }
        let pick_method = self.formatter.pick_method();
        let picker_trait = self.formatter.picker_trait();
        Some(quote! {
            fn #pick_method(&self, picker: &mut impl #picker_trait) {
                let choice = picker.options()[0];
                picker.fulfill(&choice);
            }
        })
    }
}

#[derive(Debug)]
struct IdentFormatter(Ident);

impl IdentFormatter {
    fn ident_builder_callbacks(&self) -> Ident {
        format_ident!("{}BuilderCallbacks", self.0)
    }

    fn pick_method(&self) -> Ident {
        format_ident!("pick_{}", self.0.to_string().to_lowercase())
    }

    fn picker_trait(&self) -> Ident {
        format_ident!("{}Picker", self.0)
    }
}

#[derive(Debug)]
enum DependencyType {
    Struct {
        is_root: bool,
    },
    Tuple,
    Enum {
        is_pickable: bool,
    },
}

impl DependencyType {
    fn new_struct(is_root: bool) -> DependencyType {
        DependencyType::Struct { is_root }
    }

    fn new_tuple() -> DependencyType {
        DependencyType::Tuple
    }

    fn new_enum(is_pickable: bool) -> DependencyType {
        DependencyType::Enum {
            is_pickable,
        }
    }

    fn is_root(&self) -> bool {
        match self {
            DependencyType::Struct { is_root, .. } => *is_root,
            DependencyType::Enum { .. } => false,
            DependencyType::Tuple { .. } => false
        }
    }

    fn is_pickable(&self) -> bool {
        match self {
            DependencyType::Struct { .. } => false,
            DependencyType::Enum { is_pickable, .. } => *is_pickable,
            DependencyType::Tuple { .. } => false
        }
    }
}

// =================================================================================================
// Find files
// =================================================================================================

pub fn build(path: PathBuf, root_dir: &Path, out_dir: &Path) {
    let source = root_dir.join(&path);
    let destination = out_dir.join(&path);
    let ast = to_ast(&source);
    let dt = to_dependency_tree(ast.clone());
    let dt_tokens = match dt.generate() {
        Ok(tokens) => tokens,
        Err(err) => panic!("Error while building sculptor traits \"{}\" for file {:?}.", err, &source)
    };
    let tl_tokens = to_type_linker(ast).extrapolate();
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

// =================================================================================================
// Conversion to Dependency Tree
// =================================================================================================

fn to_ast(path: &PathBuf) -> syn::File {
    let mut file = File::open(&path).expect(&format!("Cannot open file. {:?}", path));
    let mut content = String::new();
    file.read_to_string(&mut content)
        .expect(&format!("Cannot read contents. {:?}", path));
    let file = syn::parse_file(&content).expect(&format!("Cannot parse file. {:?}", path));
    file
}

fn to_dependency_tree(ast: syn::File) -> DependencyTree {
    let nodes: Vec<DependencyNode> = ast
        .items
        .into_iter()
        .map(to_dependency_node)
        .filter(|dn| dn.is_some())
        .map(|dn| dn.unwrap())
        .collect();
    DependencyTree::new(nodes)
}

fn to_dependency_node(item: Item) -> Option<DependencyNode> {
    match item {
        Item::Enum(item_enum) => Some(to_enum_dependency_node(item_enum)),
        Item::Struct(item_struct) => Some(to_struct_dependency_node(item_struct)),
        _ => None,
    }
}

fn is_item_enum_picker(item_enum: &ItemEnum) -> bool {
    for attr in &item_enum.attrs {
        if attribute_is_derive(attr, "Picker") {
            return true;
        }
    }
    false
}

fn is_item_struct_root(item_struct: &ItemStruct) -> bool {
    for attr in &item_struct.attrs {
        if attribute_is_derive(attr, "Sculptor") {
            return true;
        }
    }
    false
}

fn attribute_is_derive(attr: &Attribute, derived: &str) -> bool {
    if attr.path().is_ident("derive") {
        match attr.meta.require_list() {
            Ok(meta_list) => {
                for tree in meta_list.clone().tokens {
                    match tree {
                        TokenTree::Ident(ident) => {
                            if ident == derived {
                                return true;
                            }
                        }
                        _ => continue
                    }
                }
                false
            }
            Err(_) => false
        }
    } else {
        false
    }
}

fn to_enum_dependency_node(item_enum: ItemEnum) -> DependencyNode {
    let is_pickable = is_item_enum_picker(&item_enum);
    let d_type = DependencyType::new_enum(is_pickable);
    DependencyNode::new(item_enum.ident, d_type)
}

fn to_struct_dependency_node(item_struct: ItemStruct) -> DependencyNode {
    let is_root = is_item_struct_root(&item_struct);
    let d_type = match item_struct.fields {
        Fields::Named(_) => {
            DependencyType::new_struct(is_root)
        }
        Fields::Unnamed(_) => {
            DependencyType::new_tuple()
        }
        Fields::Unit => panic!("Struct field turns out to be unit!"),
    };
    DependencyNode::new(item_struct.ident, d_type)
}

// =================================================================================================
// Generation
// =================================================================================================

impl DependencyTree {
    fn generate(self) -> Result<TokenStream, String> {
        match self.find_root() {
            None => Err("No root sculptor found.".to_string()),
            Some(_) => {
                let callbacks_trait = self.generate_callbacks_trait();
                Ok(quote! {
                    #callbacks_trait
                })
            }
        }
    }

    fn generate_callbacks_trait(&self) -> TokenStream {
        let builder_callbacks_trait_name = self
            .find_root()
            .unwrap()
            .formatter
            .ident_builder_callbacks();
        let pick_methods = self.generate_pick_methods();
        quote! {
            pub trait #builder_callbacks_trait_name {
                #(#pick_methods)*
            }
        }
    }

    fn generate_pick_methods(&self) -> Vec<TokenStream> {
        self.nodes
            .iter()
            .map(|node| node.generate_picker_method())
            .filter_map(|node| node)
            .collect()
    }
}

//           Baz     Quux1
// Foo -> Bar -> Grault
//           `-> Quux -> Corge -^
//            Qux    `-> Garply -^
//                   Quux2

fn to_type_linker(ast: syn::File) -> TypeLinker {
    let items = ast.items;
    let root = items.iter()
        .find(|item| match item {
            Item::Struct(item_struct) => is_item_struct_root(item_struct),
            _ => false
        }).expect("").clone();
    TypeLinker::new(items, root)
}

struct TypeLinker {
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

    fn extrapolate(mut self) -> Vec<TokenStream> {
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

fn item_to_ident(item: &Item) -> Option<Ident> {
    match item {
        Item::Struct(item_struct) => Some(item_struct.ident.clone()),
        Item::Enum(item_enum) => Some(item_enum.ident.clone()),
        _ => None
    }
}

struct LinkCompiler {
    root: Ident,
    links: HashMap<Vec<FieldItemOrVariantIdent>, HashMap<Variant, Option<Item>>>,
}

impl LinkCompiler {
    fn new(root: Ident, links: HashMap<Vec<FieldItemOrVariantIdent>, HashMap<Variant, Option<Item>>>) -> LinkCompiler {
        LinkCompiler { root, links }
    }

    fn compile(self) -> Vec<TokenStream> {
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