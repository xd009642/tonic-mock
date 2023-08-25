use proc_macro::TokenStream;
use quote::quote;
use syn::{token::Token, fold::Fold, parse_macro_input, Ident, Item, ItemMod, ItemTrait, TraitItem, TraitItemFn, ImplItemFn};
use std::collections::BTreeMap;


struct MethodMockGen {
    /// The trait item function to implement for the mock server
    trait_fn: TokenStream,
    builder_method: TokenStream,
    mock_type: (String, String),
    
}

#[proc_macro_attribute]
pub fn mock(args: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemMod);

    if let Some((_, items)) = input.content.as_ref() {
        if let Some(Item::Trait(trayt)) = items.iter().find(|x| matches!(x, Item::Trait(_))) {

            let mut trait_methods = vec![];
            let mut trait_types = BTreeMap::<String, String>::new();
            for item in trayt.items.iter() {
                match item {
                    TraitItem::Fn(item) => {
                        // Needs to be an async lock
                        let mock_name = format!("{}_mock", item.sig.ident);
                        let mock_method = format!("mock_{}", item.sig.ident);
                        let builder_method = quote! {
                            pub fn #mock_method(&mut self, method_mock: _ ) {
                                self.#mock_name = Some(method_mock);
                            }
                        }.into();

                        let trait_fn = quote! {
                            #(item.sig) {
                                if let Some(mock) = self.#mock_name.lock().await.as_mut() {
                                    mock.process_request(request)
                                } else {
                                    Err(tonic::Status::unimplemented(format!("{} is not implemented", #(item.sig.ident))))
                                }
                            }
                        }.into();

                        trait_methods.push(MethodMockGen {
                            builder_method, 
                            trait_fn, 
                            mock_type: (mock_name, format!("Option<()>"))
                        });
                    },
                    TraitItem::Type(ty) => {

                    },
                    _ => {}
                }
            }

            let trait_name = trayt.ident.to_string();
            let mock_name = format!("Mock{}", trait_name);


            let expanded = quote! {
                pub struct #mock_name {
                    
                }

                impl #mock_name {
                    pub fn new() -> Self {
                        Self {
                            // Default mocks (unavailable or not implemented whatever the default
                            // is)
                        }
                    }

                    // Functions to set the method mocks for each method
                }

                impl #trait_name for #mock_name {
                    // type definitions - box stream time! 
                    // we want to wrap the type in the traits with `core::pin::Pin<Box<dyn $OG_TYPE>>`

                    // method definitions. Graph the method mock from self and call it
                }
            };
        }
    }

    //input.content.as_mut().unwrap().1.push(syn::Item::Verbatim());

    TokenStream::from(quote!{
        #input
    })
}



