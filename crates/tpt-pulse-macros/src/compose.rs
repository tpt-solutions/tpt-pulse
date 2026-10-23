//! Sub-graphs: a `#[signal_graph]` function whose parameters are `Signal` handles.
//!
//! A sub-graph is not lowered on its own. It also gets a `macro_rules!` named
//! `__pulse_graph_<name>`. A caller that declares a
//! signal from a sub-graph call defers its lowering to that macro, which passes
//! both functions to `::tpt_pulse_signals::__compose!`. `compose` inlines the
//! sub-graph's signals into the caller, renamed, and the caller is lowered as one
//! graph. So `set` on an input reaches the inlined nodes, and the cycle check
//! covers the whole composed graph.
//!
//! Each sub-graph becomes a `macro_rules!` in the module where it is declared, so
//! callers must come after it in that module (or in a submodule declared after it).
//! Give sub-graphs distinct names, since the name identifies the macro.

use std::collections::BTreeMap;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    braced,
    parse::{Parse, ParseStream},
    parse_quote,
    visit_mut::{self, VisitMut},
    Expr, ExprClosure, FnArg, Ident, ItemFn, Pat, PatIdent, PathArguments, Stmt, Type,
};

use crate::signal::{decl_names, lower, signal_decl};

/// A `let` in a caller that calls a sub-graph with signal arguments.
pub(crate) struct SubgraphCall {
    /// The position of the `let` in the caller's body.
    index: usize,
    /// The sub-graph's name.
    callee: Ident,
    /// The signals passed, from `&name` arguments.
    args: Vec<Ident>,
}

/// The first `let x = sub_graph(&a, ..);` in the caller's body, if there is one.
pub(crate) fn subgraph_call(item: &ItemFn) -> syn::Result<Option<SubgraphCall>> {
    let names = decl_names(&item.block.stmts)?;
    for (index, stmt) in item.block.stmts.iter().enumerate() {
        let Stmt::Local(local) = stmt else {
            continue;
        };
        let Some(init) = &local.init else {
            continue;
        };
        let Expr::Call(call) = &*init.expr else {
            continue;
        };
        let Expr::Path(func) = &*call.func else {
            continue;
        };
        let Some(callee) = func.path.get_ident() else {
            continue;
        };
        let args: Option<Vec<Ident>> = call
            .args
            .iter()
            .map(|arg| signal_argument(arg, &names))
            .collect();
        if let Some(args) = args {
            if !args.is_empty() {
                return Ok(Some(SubgraphCall {
                    index,
                    callee: callee.clone(),
                    args,
                }));
            }
        }
    }
    Ok(None)
}

/// `&name` where `name` is a signal declared in the caller.
fn signal_argument(expr: &Expr, names: &[Ident]) -> Option<Ident> {
    let Expr::Reference(reference) = expr else {
        return None;
    };
    let Expr::Path(path) = &*reference.expr else {
        return None;
    };
    let ident = path.path.get_ident()?;
    names
        .iter()
        .any(|name| name == ident)
        .then(|| ident.clone())
}

/// Whether a function takes at least one `Signal` handle, which makes it a sub-graph.
pub(crate) fn is_subgraph(item: &ItemFn) -> bool {
    item.sig.inputs.iter().any(|arg| match arg {
        FnArg::Typed(typed) => is_signal_type(&typed.ty),
        FnArg::Receiver(_) => false,
    })
}

fn is_signal_type(ty: &Type) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    path.path.segments.last().is_some_and(|segment| {
        segment.ident == "Signal" && matches!(segment.arguments, PathArguments::AngleBracketed(_))
    })
}

fn macro_name(callee: &Ident) -> Ident {
    format_ident!("__pulse_graph_{}", callee)
}

/// The caller's tokens, handed to the sub-graph's macro, which hands them back to `__compose`.
pub(crate) fn defer(call: &SubgraphCall, caller: &ItemFn) -> TokenStream {
    let name = macro_name(&call.callee);
    quote!(#name! { #caller })
}

/// Expand a sub-graph: keep the function, and generate the macro that inlines it.
pub(crate) fn expand_subgraph(item: ItemFn) -> syn::Result<TokenStream> {
    if subgraph_call(&item)?.is_some() {
        return Err(syn::Error::new_spanned(
            &item.sig.ident,
            "a sub-graph cannot call another sub-graph yet",
        ));
    }
    for arg in &item.sig.inputs {
        match arg {
            FnArg::Typed(typed) if matches!(*typed.pat, Pat::Ident(_)) && is_signal_type(&typed.ty) => {}
            _ => {
                return Err(syn::Error::new_spanned(
                    arg,
                    "every parameter of a sub-graph must be a `Signal<T>` handle, such as `a: Signal<f64>`",
                ))
            }
        }
    }
    let names = decl_names(&item.block.stmts)?;
    let (tail, body) = item
        .block
        .stmts
        .split_last()
        .ok_or_else(|| syn::Error::new_spanned(&item.sig.ident, "a sub-graph is empty"))?;
    for stmt in body {
        if signal_decl_of(stmt)?.is_none() {
            return Err(syn::Error::new_spanned(
                stmt,
                "a sub-graph may only declare signals; `set` and other statements are not allowed",
            ));
        }
    }
    let ends_with_signal = matches!(
        tail,
        Stmt::Expr(Expr::Path(path), None)
            if path.path.get_ident().is_some_and(|ident| names.contains(ident))
    );
    if !ends_with_signal {
        return Err(syn::Error::new_spanned(
            tail,
            "a sub-graph must end with one of its signals, such as `scaled`",
        ));
    }

    let name = macro_name(&item.sig.ident);
    let direct = direct_stub(&item);
    Ok(quote! {
        #direct

        #[doc(hidden)]
        #[allow(unused_macros)]
        macro_rules! #name {
            ($($caller:tt)*) => {
                ::tpt_pulse_signals::__compose! { callee { #item } caller { $($caller)* } }
            };
        }
    })
}

fn signal_decl_of(stmt: &Stmt) -> syn::Result<Option<()>> {
    Ok(signal_decl(stmt, 0)?.map(|_| ()))
}

/// Inline the sub-graph at the caller's call site, then lower the caller, or defer
/// again if the caller calls another sub-graph.
pub(crate) fn compose(callee: ItemFn, mut caller: ItemFn) -> syn::Result<TokenStream> {
    let call = subgraph_call(&caller)?.ok_or_else(|| {
        syn::Error::new_spanned(&caller.sig.ident, "the caller does not call a sub-graph")
    })?;
    if call.callee != callee.sig.ident {
        return Err(syn::Error::new_spanned(
            &call.callee,
            format!("expected a call to `{}`", callee.sig.ident),
        ));
    }

    let params: Vec<Ident> = callee
        .sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            FnArg::Typed(typed) => match &*typed.pat {
                Pat::Ident(pat) => Some(pat.ident.clone()),
                _ => None,
            },
            FnArg::Receiver(_) => None,
        })
        .collect();
    if params.len() != call.args.len() {
        return Err(syn::Error::new_spanned(
            &call.callee,
            format!(
                "`{}` takes {} signal(s), but {} were passed",
                callee.sig.ident,
                params.len(),
                call.args.len()
            ),
        ));
    }

    let Stmt::Local(local) = caller.block.stmts[call.index].clone() else {
        unreachable!("subgraph_call returns the index of a `let`");
    };
    let bound = bound_name(&local.pat)?;

    // Every name the sub-graph uses is renamed: parameters to the caller's
    // signals, and its own signals to `<bound>__<name>`, so they cannot collide.
    let mut renames: BTreeMap<String, Ident> = BTreeMap::new();
    for (param, arg) in params.iter().zip(&call.args) {
        renames.insert(param.to_string(), arg.clone());
    }
    let inner = decl_names(&callee.block.stmts)?;
    let caller_names = decl_names(&caller.block.stmts)?;
    for name in &inner {
        let renamed = format_ident!("{}_{}", bound, name);
        if caller_names.contains(&renamed) {
            return Err(syn::Error::new_spanned(
                &renamed,
                "this name is already a signal in the caller; rename one of them",
            ));
        }
        renames.insert(name.to_string(), renamed);
    }
    let mut renamer = Renamer { renames: &renames };

    let (tail, body) = callee
        .block
        .stmts
        .split_last()
        .expect("sub-graphs end with a signal, checked when they were expanded");
    let mut inlined: Vec<Stmt> = Vec::new();
    for stmt in body {
        let mut stmt = stmt.clone();
        syn::visit_mut::visit_stmt_mut(&mut renamer, &mut stmt);
        inlined.push(stmt);
    }
    let Stmt::Expr(Expr::Path(tail_path), None) = tail.clone() else {
        unreachable!("sub-graphs end with a signal, checked when they were expanded");
    };
    let tail_name = tail_path
        .path
        .get_ident()
        .and_then(|ident| renames.get(&ident.to_string()))
        .expect("the tail is a declared signal")
        .clone();

    // `let out = sub_graph(..)` becomes an identity map over the sub-graph's
    // output, so `out` is an ordinary derived signal in the caller.
    let pat = &local.pat;
    let forward: Stmt = parse_quote! {
        let #pat = ::tpt_pulse_signals::Signal::map(&#tail_name, |value| value);
    };

    let mut stmts = caller.block.stmts[..call.index].to_vec();
    stmts.extend(inlined);
    stmts.push(forward);
    stmts.extend_from_slice(&caller.block.stmts[call.index + 1..]);
    caller.block.stmts = stmts;

    match subgraph_call(&caller)? {
        Some(next) => {
            let name = macro_name(&next.callee);
            Ok(quote!(#name! { #caller }))
        }
        None => lower(caller),
    }
}

/// The name a `let` binds, with or without a type.
fn bound_name(pat: &Pat) -> syn::Result<Ident> {
    match pat {
        Pat::Ident(ident) => Ok(ident.ident.clone()),
        Pat::Type(typed) => bound_name(&typed.pat),
        other => Err(syn::Error::new_spanned(
            other,
            "a sub-graph call must bind a single name",
        )),
    }
}

/// Renames the sub-graph's signal names. Closures are left alone: they cannot read
/// signals, so the names inside them are their own.
struct Renamer<'a> {
    renames: &'a BTreeMap<String, Ident>,
}

impl VisitMut for Renamer<'_> {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        if let Expr::Path(path) = expr {
            if let Some(ident) = path.path.get_ident() {
                if let Some(renamed) = self.renames.get(&ident.to_string()) {
                    path.path = renamed.clone().into();
                }
            }
        }
        visit_mut::visit_expr_mut(self, expr);
    }

    fn visit_pat_ident_mut(&mut self, pat: &mut PatIdent) {
        if let Some(renamed) = self.renames.get(&pat.ident.to_string()) {
            pat.ident = renamed.clone();
        }
        visit_mut::visit_pat_ident_mut(self, pat);
    }

    fn visit_expr_closure_mut(&mut self, _closure: &mut ExprClosure) {}
}

/// The input to `__compose!`: `callee { fn .. } caller { fn .. }`.
pub(crate) struct ComposeInput {
    callee: ItemFn,
    caller: ItemFn,
}

impl Parse for ComposeInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        expect_keyword(input, "callee")?;
        let callee;
        braced!(callee in input);
        let callee: ItemFn = callee.parse()?;
        expect_keyword(input, "caller")?;
        let caller;
        braced!(caller in input);
        let caller: ItemFn = caller.parse()?;
        Ok(Self { callee, caller })
    }
}

fn expect_keyword(input: ParseStream, keyword: &str) -> syn::Result<()> {
    let ident: Ident = input.parse()?;
    if ident == keyword {
        Ok(())
    } else {
        Err(syn::Error::new_spanned(
            ident,
            format!("expected `{keyword}`"),
        ))
    }
}

/// Entry point of `__compose!`.
pub(crate) fn compose_tokens(input: TokenStream) -> syn::Result<TokenStream> {
    let ComposeInput { callee, caller } = syn::parse2(input)?;
    compose(callee, caller)
}

/// The function as a direct call sees it. Its body cannot be lowered on its own,
/// so a direct call panics. Callers use the inlined form instead.
fn direct_stub(item: &ItemFn) -> ItemFn {
    let mut stub = item.clone();
    stub.attrs.push(parse_quote!(#[allow(unused_variables)]));
    stub.block = Box::new(parse_quote!({
        ::core::panic!("a sub-graph can only be called from a #[signal_graph] function")
    }));
    stub
}
