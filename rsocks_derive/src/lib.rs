use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

use quote::{quote, ToTokens};
use syn::{Data, Field, Ident, Index, Type};

#[proc_macro_derive(Sendable, attributes(error_type))]
pub fn derive_sendable(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = syn::parse(input);

    if let Err(e) = input {
        return e.to_compile_error().into();
    }
    // Build the impl
    let expanded = impl_sendable(&input.unwrap());
    // Return the generated impl
    TokenStream::from(expanded)
}

fn impl_sendable(ast: &syn::DeriveInput) -> proc_macro::TokenStream {
    let name = &ast.ident;
    // Get the fields of the struct
    // TODO: Handle tuple structs
    let fields = match &ast.data {
        syn::Data::Struct(data) => &data.fields,
        _ => panic!("Sendable can only be derived for structs"),
    };

    let data = {
        if let Data::Struct(data) = &ast.data {
            data
        } else {
            unreachable!()
        }
    };

    let mut type_count: Vec<(Type, u32)> = Vec::new();
    for field in fields {
        let ty = &field.ty;
        let type_name = format!("{}", quote! {#ty});
        let mut found = false;
        for (t, c) in &mut type_count {
            let fmt_typename = format!("{}", quote! {#t});
            if type_name == fmt_typename {
                *c += 1;
                found = true;
                break;
            }
        }
        if !found {
            type_count.push((ty.clone(), 1));
        }
    }

    // Check that all fields implement Sendable.
    // TODO: Switch to a implementation that is not a dependency on static_assertions
    let field_impl_check: TokenStream2 = type_count
        .iter()
        .map(|(ty, _)| {
            quote! {
                const _: fn() = || {
                    fn _assert_sendable<T: rsocks::Sendable>() {}
                    _assert_sendable::<#ty>();
                };
            }
        })
        .collect();
    // Generate the size function. (Take the size of each field and sum them up)
    let field_size: TokenStream2 = generate_size(&data);

    // Generate the send fn. (Serialize each field and append them to a Vec<u8>)
    let send_gen: TokenStream2 = generate_send(&data);
    // Generate the size_const fn. (Check if all fields have a const size)
    let dyn_size = type_count.iter().map(|field| {
        let ty = &field.0;
        quote! {
            <#ty as rsocks::Sendable>::size_const()
        }
    });
    // Generate the recv fn. (Deserialize each field from a dyn Read)
    let recv_gen: TokenStream2 = generate_recv(&data, &name);
    quote! {

        #field_impl_check // Check that all fields implement Sendable

        impl rsocks::Sendable for #name {
            type Error = std::io::Error; // TODO: In the future, determine if impl types should just use anyhow::Error

            fn size(&self) -> u32 {
                let mut size = 0;
                #field_size
                size
            }

            fn size_const() -> bool {
                static size_l: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
                *size_l.get_or_init(|| {
                    let mut size = true;
                    #(
                        size &= #dyn_size;
                    )*
                    size
                })
            }

            fn send(&self) -> Vec<u8> {
                let mut data = Vec::new();
                #send_gen
                data
            }

            fn recv(data: &mut dyn std::io::Read) -> Result<Self, Self::Error> {
                Ok(
                    #recv_gen
                )
            }
        }
    }
    .into()
}
/// Gets the identifier for each field and executes transform on it.
fn field_struct_gen(
    transform: fn(&TokenStream2, &Field) -> TokenStream2,
    input: &syn::DataStruct,
) -> TokenStream2 {
    match &input.fields {
        syn::Fields::Named(ref fields) => fields
            .named
            .iter()
            .map(|field| {
                let ident = field.ident.as_ref().unwrap();
                transform(&ident.to_token_stream(), field)
            })
            .collect(),
        syn::Fields::Unnamed(ref fields) => fields
            .unnamed
            .iter()
            .enumerate()
            .map(|(i, field)| {
                let ident = Index::from(i);
                transform(&ident.to_token_stream(), field)
            })
            .collect(),
        syn::Fields::Unit => {
            quote! {}
        }
    }
}

fn generate_size(input: &syn::DataStruct) -> TokenStream2 {
    field_struct_gen(
        |ident, field| {
            let ty = &field.ty;
            quote! {
                size += <#ty as rsocks::Sendable>::size(&self.#ident);
            }
        },
        input,
    )
}

fn generate_send(input: &syn::DataStruct) -> TokenStream2 {
    field_struct_gen(
        |ident, _| {
            quote! {
                data.extend(self.#ident.send());
            }
        },
        input,
    )
}

fn generate_recv(input: &syn::DataStruct, name: &Ident) -> TokenStream2 {
    // we cant use field_struct_gen here because named and unnamed fields are handled differently
    match &input.fields {
        syn::Fields::Named(ref named) => {
            let fields: TokenStream2 = named
                .named
                .iter()
                .map(|field| {
                    let ty = &field.ty;
                    let ident = field.ident.as_ref().unwrap();
                    quote! {
                        #ident: <#ty as rsocks::Sendable>::recv(data).unwrap(),
                    }
                })
                .collect();
            quote! {
                #name {
                    #fields
                }
            }
        }
        syn::Fields::Unnamed(ref unnamed) => {
            let fields: TokenStream2 = unnamed
                .unnamed
                .iter()
                .enumerate()
                .map(|(_, field)| {
                    let ty = &field.ty;
                    quote! {
                        <#ty as rsocks::Sendable>::recv(data).unwrap(),
                    }
                })
                .collect();
            quote! {
                #name (
                    #fields
                )
            }
        }
        syn::Fields::Unit => {
            quote! {
                #name
            }
        }
    }
}
