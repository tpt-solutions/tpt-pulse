//! Lowering for `global_protocol!`.
//!
//! The input is a list of `From -> To : Message;` steps. The steps are checked at
//! compile time by building the protocol and projecting it, and the expansion
//! constructs the same [`pulse_multiparty::GlobalProtocol`] at run time.

use proc_macro2::{Span, TokenStream};
use pulse_multiparty::GlobalProtocol;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    LitStr, Token, Type,
};

struct Step {
    from: String,
    to: String,
    label: String,
}

struct Steps(Vec<Step>);

impl Parse for Steps {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut steps = Vec::new();
        while !input.is_empty() {
            let from: syn::Ident = input.parse()?;
            input.parse::<Token![->]>()?;
            let to: syn::Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            let message: Type = input.parse()?;
            if input.peek(Token![;]) {
                input.parse::<Token![;]>()?;
            } else if !input.is_empty() {
                return Err(input.error("expected `;` after a protocol step"));
            }
            steps.push(Step {
                from: from.to_string(),
                to: to.to_string(),
                label: message_label(&message),
            });
        }
        Ok(Self(steps))
    }
}

/// The message's type name, without its module path.
fn message_label(ty: &Type) -> String {
    match ty {
        Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_default(),
        other => quote!(#other).to_string(),
    }
}

/// Expand `global_protocol! { A -> B : Msg; ... }`.
pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let Steps(steps) = syn::parse2(input)?;
    let triples: Vec<(&str, &str, &str)> = steps
        .iter()
        .map(|step| (step.from.as_str(), step.to.as_str(), step.label.as_str()))
        .collect();
    GlobalProtocol::from_steps(&triples)
        .project()
        .map_err(|error| syn::Error::new(Span::call_site(), error))?;

    let entries = steps.iter().map(|step| {
        let from = LitStr::new(&step.from, Span::call_site());
        let to = LitStr::new(&step.to, Span::call_site());
        let label = LitStr::new(&step.label, Span::call_site());
        quote!((#from, #to, #label))
    });
    Ok(quote! {
        ::pulse_multiparty::GlobalProtocol::from_steps(&[#(#entries),*])
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_to_a_protocol_constructor() {
        let tokens = expand(quote!(Client -> Server : AuthReq; Server -> Client : AuthOk;))
            .expect("valid protocol");
        let text = tokens.to_string();
        assert!(text.contains("from_steps"));
        assert!(text.contains("\"AuthReq\""));
    }

    #[test]
    fn rejects_self_messages_at_compile_time() {
        let error = expand(quote!(A -> A : Ping;)).expect_err("self message");
        assert!(
            error.to_string().contains("sends `Ping` to itself"),
            "{error}"
        );
    }
}
