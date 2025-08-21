mod derive_edge;
mod derive_node;

use proc_macro::TokenStream;

#[proc_macro_derive(Node, attributes(junction))]
pub fn derive_node(input: TokenStream) -> TokenStream {
    derive_node::derive_node(input)
}

#[proc_macro_derive(Edge, attributes(junction))]
pub fn derive_edge(input: TokenStream) -> TokenStream {
    derive_edge::derive_edge(input)
}
