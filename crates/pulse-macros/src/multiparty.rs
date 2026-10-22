//! Lowering for `global_protocol!` and `session_types!`.
//!
//! Both take a list of `From -> To : Message;` steps and check the steps at
//! compile time by building the protocol and projecting it. `global_protocol!`
//! expands to a [`pulse_multiparty::GlobalProtocol`] value. `session_types!`
//! expands to a module with one `pulse-core` protocol type per role, so each
//! role's endpoint is a `Chan<Module::Role, Role>`.

use proc_macro2::{Span, TokenStream};
use pulse_multiparty::GlobalProtocol;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    Ident, LitStr, Token, Type,
};

struct Step {
    from: String,
    to: String,
    message: Type,
    label: String,
}

struct Steps(Vec<Step>);

impl Parse for Steps {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut steps = Vec::new();
        while !input.is_empty() {
            let from: Ident = input.parse()?;
            input.parse::<Token![->]>()?;
            let to: Ident = input.parse()?;
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
                message,
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

/// Check the steps by building and projecting the protocol.
fn check(steps: &[Step]) -> syn::Result<GlobalProtocol> {
    let triples: Vec<(&str, &str, &str)> = steps
        .iter()
        .map(|step| (step.from.as_str(), step.to.as_str(), step.label.as_str()))
        .collect();
    let protocol = GlobalProtocol::from_steps(&triples);
    protocol
        .project()
        .map_err(|error| syn::Error::new(Span::call_site(), error))?;
    Ok(protocol)
}

/// Expand `global_protocol! { A -> B : Msg; ... }`.
pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let Steps(steps) = syn::parse2(input)?;
    check(&steps)?;

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

/// `Name { A -> B : Msg; ... }` for `session_types!`.
struct SessionTypes {
    name: Ident,
    steps: Steps,
}

impl Parse for SessionTypes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let content;
        syn::braced!(content in input);
        let steps: Steps = content.parse()?;
        Ok(Self { name, steps })
    }
}

/// Expand `session_types! { Name { A -> B : Msg; ... } }`.
///
/// The module `Name` contains one type alias per role. Each alias is the
/// `pulse-core` protocol that role follows: a `Send` for each message it sends
/// and a `Recv` for each message it receives, in order, ending in `Close`.
/// Message types are resolved through `use super::*;`.
pub fn expand_session_types(input: TokenStream) -> syn::Result<TokenStream> {
    let SessionTypes { name, steps } = syn::parse2(input)?;
    let steps = steps.0;
    check(&steps)?;

    let mut roles: Vec<&str> = Vec::new();
    for step in &steps {
        for role in [step.from.as_str(), step.to.as_str()] {
            if !roles.contains(&role) {
                roles.push(role);
            }
        }
    }
    let aliases = roles.iter().map(|role| {
        let alias = format_ident!("{}", role);
        let protocol = local_protocol(&steps, role);
        let doc = format!("The protocol followed by the `{role}` role.");
        quote!(#[doc = #doc] pub type #alias = #protocol;)
    });
    Ok(quote! {
        #[doc = concat!("Session types for the `", stringify!(#name), "` protocol.")]
        #[allow(non_snake_case, non_camel_case_types)]
        pub mod #name {
            #[allow(unused_imports)]
            use super::*;
            #(#aliases)*
        }
    })
}

/// The `pulse-core` protocol for one role, built from its actions in order.
fn local_protocol(steps: &[Step], role: &str) -> TokenStream {
    let actions = steps.iter().filter_map(|step| {
        if step.from == role {
            Some((true, &step.message))
        } else if step.to == role {
            Some((false, &step.message))
        } else {
            None
        }
    });
    let actions: Vec<(bool, &Type)> = actions.collect();
    actions
        .into_iter()
        .rev()
        .fold(quote!(::pulse_core::Close), |rest, (send, message)| {
            if send {
                quote!(::pulse_core::Send<#message, #rest>)
            } else {
                quote!(::pulse_core::Recv<#message, #rest>)
            }
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

    #[test]
    fn role_types_are_dual_sequences() {
        let steps = syn::parse2::<Steps>(quote!(Client -> Server : Req; Server -> Client : Resp;))
            .expect("steps")
            .0;
        let client = local_protocol(&steps, "Client").to_string();
        let server = local_protocol(&steps, "Server").to_string();
        assert!(client.contains("Send < Req"), "{client}");
        assert!(client.contains("Recv < Resp"), "{client}");
        assert!(server.contains("Recv < Req"), "{server}");
        assert!(server.contains("Send < Resp"), "{server}");
    }
}
