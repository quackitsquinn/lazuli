use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

use quote::quote;

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
    // TODO: Throughout the code make it account for duplicates of the same type.
    // prevent u32::size_const() && u32::size_const() and similar things
    let fields = match &ast.data {
        syn::Data::Struct(data) => &data.fields,
        _ => panic!("Sendable can only be derived for structs"),
    };
    // Check that all fields implement Sendable.
    // TODO: Switch to a implementation that is not a dependency on static_assertions
    let field_impl_check: TokenStream2 = fields
        .iter()
        .map(|field| {
            let ty = &field.ty;
            quote! {
                rsocks::__sa::assert_impl_all!(#ty: rsocks::Sendable);
            }
        })
        .collect();
    // Generate the size function. (Take the size of each field and sum them up)
    let field_size: TokenStream2 = fields
        .iter()
        .map(|field| {
            let ty = &field.ty;
            let ident = field.ident.as_ref().unwrap();
            quote! {
                size += <#ty as rsocks::Sendable>::size(&self.#ident);
            }
        })
        .collect();

    // Generate the send fn. (Serialize each field and append them to a Vec<u8>)
    let send_gen: TokenStream2 = fields
        .iter()
        .map(|field| {
            let ty = &field.ty;
            let ident = field.ident.as_ref().unwrap();
            quote! {
                data.extend(self.#ident.send());
            }
        })
        .collect();
    // Generate the size_const fn. (Check if all fields have a const size)
    // TODO: Cache the result of size_const
    let dyn_size = fields.iter().map(|field| {
        let ty = &field.ty;
        quote! {
            <#ty as rsocks::Sendable>::size_const()
        }
    });
    // Generate the recv fn. (Deserialize each field from a dyn Read)
    let recv_gen: TokenStream2 = fields
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

        #field_impl_check // Check that all fields implement Sendable

        impl rsocks::Sendable for #name {
            type Error = std::io::Error;

            fn size(&self) -> u32 {
                let mut size = 0;
                #field_size
                size
            }

            fn size_const() -> bool {
                true
                #(&& #dyn_size)*
            }

            fn send(&self) -> Vec<u8> {
                let mut data = Vec::new();
                #send_gen
                data
            }

            fn recv(data: &mut dyn std::io::Read) -> Result<Self, Self::Error> {
                Ok(Self {
                    #recv_gen
                })
            }
        }
    }
    .into()
}
