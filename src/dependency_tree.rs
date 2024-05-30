use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{Fields, Item, ItemEnum, ItemStruct};

use crate::{is_item_enum_picker, is_item_struct_root};

#[derive(Debug)]
pub struct DependencyTree {
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

    pub(crate) fn generate(self) -> Result<TokenStream, String> {
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

pub fn to_dependency_tree(ast: syn::File) -> DependencyTree {
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