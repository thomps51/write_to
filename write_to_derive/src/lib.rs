use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{self, Data, DataStruct, Fields};

#[proc_macro_derive(WriteTo)]
pub fn write_to_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_write_to(&ast)
}

fn impl_write_to(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => fields.named.clone(),
        _ => panic!("this derive macro only works on structs with named fields"),
    };
    let field_writes = fields.into_iter().map(|f| {
        let field_name = f.ident;
        quote! {
            self.#field_name.write_to(writer)?;
        }
    });

    let gen = quote! {
        #[automatically_derived]
        impl WriteTo for #name {
            fn write_to<T: std::io::Write>(&self, writer: &mut T) -> std::io::Result<()> {
                #(#field_writes)*
                Ok(())
            }
        }
    };
    gen.into()
}

#[proc_macro_derive(ReadFrom)]
pub fn read_from_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_read_from(&ast)
}

fn impl_read_from(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => fields.named.clone(),
        _ => panic!("this derive macro only works on structs with named fields"),
    };
    let field_names = fields.clone().into_iter().map(|f| {
        let field_name = f.ident;
        quote! {
            #field_name,
        }
    });
    let field_reads = fields.into_iter().map(|f| {
        let field_name = f.ident;
        let field_type = f.ty;
        quote! {
            let (#field_name, len) = <#field_type>::read_from(reader, len)?;
        }
    });

    let gen = quote! {
        #[automatically_derived]
        impl ReadFrom for #name {
            fn read_from<T: std::io::Read>(reader: &mut T, len: usize) -> std::io::Result<(Self, usize)> {
                #(#field_reads)*
                // If length != 0, we have unread data
                Ok((Self{#(#field_names)*}, len))
            }
        }
    };
    gen.into()
}

#[proc_macro_derive(Length)]
pub fn length_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_length(&ast)
}

fn impl_length(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => fields.named.clone(),
        _ => panic!("this derive macro only works on structs with named fields"),
    };
    let field_lengths = fields.into_iter().map(|f| {
        let field_name = f.ident;
        quote! {
            let length = length + self.#field_name.length();
        }
    });

    let gen = quote! {
        #[automatically_derived]
        impl Length for #name {
            fn length(&self) -> usize {
                let length = 0;
                #(#field_lengths)*
                length
            }
        }
    };
    gen.into()
}

// Adds accessors to sized integer types, standardizing them to usize or isize (under 128 bits).
// Assumes 64 bit system.  Because of macro hygiene, the generated accessors will be inaccessible
// if you use it in a macro_rules to declare a class.
#[proc_macro_derive(NormalizedIntegerAccessors)]
pub fn normalized_members_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_normalized_members(&ast)
}

fn impl_normalized_members(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => fields.named.clone(),
        _ => panic!("this derive macro only works on structs with named fields"),
    };
    let unsigned_types = ["u32", "u64"];
    let signed_types = ["i32", "i64"];
    let field_members = fields.clone().into_iter().map(|field| match &field.ty {
        syn::Type::Path(type_path) => {
            let field_name = field.ident;
            let type_str = type_path.clone().into_token_stream().to_string();
            let result_type_str = if unsigned_types.contains(&type_str.as_str()) {
                "usize"
            } else if signed_types.contains(&type_str.as_str()) {
                "isize"
            } else {
                return quote!();
            };
            let result_type = syn::Ident::new(result_type_str, Span::call_site());
            let field_name_str = field_name.clone().unwrap().into_token_stream().to_string();
            let get_func_name =
                syn::Ident::new(&format!("get_{}", field_name_str), Span::call_site());
            ::quote::quote_spanned! {Span::mixed_site() =>
                pub fn #get_func_name(&self) -> #result_type {
                    self.#field_name as _
                }
            }
        }
        _ => quote!(),
    });
    let field_params = fields.clone().into_iter().map(|field| {
        let field_name = field.ident;
        let field_type = field.ty;
        match &field_type {
            syn::Type::Path(type_path) => {
                let type_str = type_path.clone().into_token_stream().to_string();
                let result_type_str = if unsigned_types.contains(&type_str.as_str()) {
                    "usize"
                } else if signed_types.contains(&type_str.as_str()) {
                    "isize"
                } else {
                    return quote! {
                        #field_name: #field_type
                    };
                };
                let result_type = syn::Ident::new(result_type_str, Span::call_site());
                quote! {
                    #field_name: #result_type
                }
            }
            _ => quote!(
                #field_name: #field_type
            ),
        }
    });
    let field_args = fields.into_iter().map(|field| {
        let field_name = field.ident;
        let field_type = field.ty;
        match &field_type {
            syn::Type::Path(_type_path) => {
                quote! {
                    #field_name: #field_name as _
                }
            }
            _ => quote!(
                #field_name
            ),
        }
    });
    // let gen = quote! {
    let gen = ::quote::quote_spanned! {Span::mixed_site() =>
        #[automatically_derived]
        impl #name {
            fn normalized_new(#(#field_params),*) -> Self {
                Self {
                    #(#field_args),*
                }
            }

            #(#field_members)*
        }
    };
    gen.into()
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
