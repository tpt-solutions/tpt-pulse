//! Lowering for `global_protocol!` and `session_types!`.
//!
//! `global_protocol!` takes the same statements as the text format: messages,
//! `choice by D { ... } or { ... }`, `loop { ... }`, and `continue;`. It checks
//! them at compile time by projecting the protocol, then expands to a
//! [`pulse_multiparty::GlobalProtocol`] value.
//!
//! `session_types!` takes plain messages only. It expands to a module with one
//! `pulse-core` protocol type per role, so each role's endpoint is a
//! `Chan<Module::Role, Role>`.

use proc_macro2::{Span, TokenStream};
use pulse_multiparty::{GlobalProtocol, Statement as GlobalStatement};
use quote::{format_ident, quote};
use syn::{
    braced,
    parse::{Parse, ParseStream},
    Ident, LitStr, Token, Type,
};

/// A message `from -> to : Message;`.
struct Step {
    from: String,
    to: String,
    message: Type,
    label: String,
}

/// One statement as written in the macro input.
enum Stmt {
    Message(Box<Step>),
    Choice {
        decider: Ident,
        branches: Vec<Vec<Stmt>>,
    },
    Loop(Vec<Stmt>),
    Continue,
}

struct Statements(Vec<Stmt>);

impl Parse for Statements {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        parse_statements(input).map(Self)
    }
}

/// Words that start a statement and cannot be role names.
const KEYWORDS: [&str; 3] = ["choice", "by", "or"];

fn peek_word(input: ParseStream, word: &str) -> bool {
    input.peek(Ident)
        && input
            .fork()
            .parse::<Ident>()
            .is_ok_and(|ident| ident == word)
}

fn parse_role(input: ParseStream) -> syn::Result<Ident> {
    let role: Ident = input.parse()?;
    if KEYWORDS.iter().any(|keyword| role == keyword) {
        return Err(syn::Error::new(
            role.span(),
            format!("`{role}` is a keyword, not a role name"),
        ));
    }
    Ok(role)
}

fn parse_statements(input: ParseStream) -> syn::Result<Vec<Stmt>> {
    let mut statements = Vec::new();
    while !input.is_empty() {
        if input.peek(Token![loop]) {
            input.parse::<Token![loop]>()?;
            let body;
            braced!(body in input);
            statements.push(Stmt::Loop(parse_statements(&body)?));
        } else if input.peek(Token![continue]) {
            input.parse::<Token![continue]>()?;
            input.parse::<Token![;]>()?;
            statements.push(Stmt::Continue);
        } else if peek_word(input, "choice") {
            input.parse::<Ident>()?;
            if !peek_word(input, "by") {
                return Err(input.error("expected `by` after `choice`"));
            }
            input.parse::<Ident>()?;
            let decider = parse_role(input)?;
            let body;
            braced!(body in input);
            let mut branches = vec![parse_statements(&body)?];
            while peek_word(input, "or") {
                input.parse::<Ident>()?;
                let body;
                braced!(body in input);
                branches.push(parse_statements(&body)?);
            }
            statements.push(Stmt::Choice { decider, branches });
        } else {
            let from = parse_role(input)?;
            input.parse::<Token![->]>()?;
            let to = parse_role(input)?;
            input.parse::<Token![:]>()?;
            let message: Type = input.parse()?;
            if input.peek(Token![;]) {
                input.parse::<Token![;]>()?;
            } else if !input.is_empty() {
                return Err(input.error("expected `;` after a protocol step"));
            }
            statements.push(Stmt::Message(Box::new(Step {
                from: from.to_string(),
                to: to.to_string(),
                label: message_label(&message),
                message,
            })));
        }
    }
    Ok(statements)
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

/// The statements as the runtime representation, for checking and emission.
fn to_global(statements: &[Stmt]) -> Vec<GlobalStatement> {
    statements
        .iter()
        .map(|statement| match statement {
            Stmt::Message(step) => GlobalStatement::Message(pulse_multiparty::Step {
                from: step.from.clone(),
                to: step.to.clone(),
                label: step.label.clone(),
            }),
            Stmt::Choice { decider, branches } => GlobalStatement::Choice {
                decider: decider.to_string(),
                branches: branches.iter().map(|branch| to_global(branch)).collect(),
            },
            Stmt::Loop(body) => GlobalStatement::Loop(to_global(body)),
            Stmt::Continue => GlobalStatement::Continue,
        })
        .collect()
}

/// Check the statements by projecting the protocol they describe.
fn check(statements: &[Stmt]) -> syn::Result<GlobalProtocol> {
    let protocol = GlobalProtocol::new(to_global(statements));
    protocol
        .project()
        .map_err(|error| syn::Error::new(Span::call_site(), error))?;
    Ok(protocol)
}

fn lit(text: &str) -> LitStr {
    LitStr::new(text, Span::call_site())
}

/// The tokens that rebuild one checked statement.
fn emit_statement(statement: &GlobalStatement) -> TokenStream {
    match statement {
        GlobalStatement::Message(step) => {
            let from = lit(&step.from);
            let to = lit(&step.to);
            let label = lit(&step.label);
            quote!(::pulse_multiparty::Statement::Message(
                ::pulse_multiparty::Step {
                    from: ::std::string::String::from(#from),
                    to: ::std::string::String::from(#to),
                    label: ::std::string::String::from(#label),
                }
            ))
        }
        GlobalStatement::Choice { decider, branches } => {
            let decider = lit(decider);
            let branches = branches.iter().map(|branch| {
                let items = branch.iter().map(emit_statement);
                quote!(::std::vec![#(#items),*])
            });
            quote!(::pulse_multiparty::Statement::Choice {
                decider: ::std::string::String::from(#decider),
                branches: ::std::vec![#(#branches),*],
            })
        }
        GlobalStatement::Loop(body) => {
            let items = body.iter().map(emit_statement);
            quote!(::pulse_multiparty::Statement::Loop(
                ::std::vec![#(#items),*]
            ))
        }
        GlobalStatement::Continue => quote!(::pulse_multiparty::Statement::Continue),
    }
}

/// Expand `global_protocol! { A -> B : Msg; loop { ... } ... }`.
pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let Statements(statements) = syn::parse2(input)?;
    let protocol = check(&statements)?;
    let items = protocol.statements().iter().map(emit_statement);
    Ok(quote! {
        ::pulse_multiparty::GlobalProtocol::new(::std::vec![#(#items),*])
    })
}

/// `Name { A -> B : Msg; ... }` for `session_types!`.
struct SessionTypes {
    name: Ident,
    statements: Statements,
}

impl Parse for SessionTypes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let content;
        braced!(content in input);
        let statements: Statements = content.parse()?;
        Ok(Self { name, statements })
    }
}

/// The messages of a `session_types!` body. Choices and loops are rejected.
fn plain_steps(statements: Vec<Stmt>) -> syn::Result<Vec<Step>> {
    statements
        .into_iter()
        .map(|statement| match statement {
            Stmt::Message(step) => Ok(*step),
            _ => Err(syn::Error::new(
                Span::call_site(),
                "session_types! takes plain messages; use global_protocol! for choice and loop",
            )),
        })
        .collect()
}

/// Expand `session_types! { Name { A -> B : Msg; ... } }`.
///
/// The module `Name` contains one type alias per role. Each alias is the
/// `pulse-core` protocol that role follows: a `Send` for each message it sends
/// and a `Recv` for each message it receives, in order, ending in `Close`.
/// Message types are resolved through `use super::*;`.
pub fn expand_session_types(input: TokenStream) -> syn::Result<TokenStream> {
    let SessionTypes { name, statements } = syn::parse2(input)?;
    check(&statements.0)?;
    let steps = plain_steps(statements.0)?;

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
        assert!(text.contains("GlobalProtocol"));
        assert!(text.contains("\"AuthReq\""));
    }

    #[test]
    fn expands_choice_and_loop() {
        let tokens = expand(quote! {
            loop {
                A -> B : Ask;
                choice by B {
                    B -> A : Answer;
                    continue;
                } or {
                    B -> A : Done;
                }
            }
        })
        .expect("valid protocol");
        let text = tokens.to_string();
        assert!(text.contains("Statement :: Loop"), "{text}");
        assert!(text.contains("Statement :: Choice"), "{text}");
        assert!(text.contains("Statement :: Continue"), "{text}");
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
    fn rejects_a_continue_outside_a_choice() {
        let error = expand(quote!(loop { A -> B : X; continue; })).expect_err("bad continue");
        assert!(error.to_string().contains("`continue`"), "{error}");
    }

    #[test]
    fn session_types_reject_choice() {
        let error = expand_session_types(quote! {
            Bad {
                choice by A { A -> B : X; } or { A -> B : Y; }
            }
        })
        .expect_err("choice is not allowed");
        assert!(error.to_string().contains("plain messages"), "{error}");
    }

    #[test]
    fn role_types_are_dual_sequences() {
        let steps = plain_steps(
            syn::parse2::<Statements>(quote!(Client -> Server : Req; Server -> Client : Resp;))
                .expect("steps")
                .0,
        )
        .expect("plain messages");
        let client = local_protocol(&steps, "Client").to_string();
        let server = local_protocol(&steps, "Server").to_string();
        assert!(client.contains("Send < Req"), "{client}");
        assert!(client.contains("Recv < Resp"), "{client}");
        assert!(server.contains("Recv < Req"), "{server}");
        assert!(server.contains("Send < Resp"), "{server}");
    }
}
