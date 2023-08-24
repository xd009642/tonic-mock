use proc_macro::TokenStream;
use quote::quote;
use syn::{fold::Fold, parse_macro_input, Ident, Item, ItemMod, ItemTrait, TraitItem};


#[proc_macro_attribute]
pub fn mock(args: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemMod);

    if let Some((_, items)) = input.content {
        if let Some(Item::Trait(trayt)) = items.iter().find(|x| matches!(x, Item::Trait(_))) {
            for item in trayt.items.iter().filter(|x| matches!(x, TraitItem::Fn(_))) {
                let item = if let TraitItem::Fn(item) = item {
                    item
                } else {
                    unreachable!();
                };
                eprintln!("Have item: {:?}", item);
            }
        }
    }

    TokenStream::from(quote!{})
}



