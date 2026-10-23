//! Lowering for `#[signal_graph]`.
//!
//! Signal declarations become ordinary locals. A derived signal is a call to its
//! closure on the current values of its inputs, and `set` recomputes the derived
//! signals downstream of the target, in declaration order. The generated code has
//! no runtime graph, no dynamic dispatch, and no heap allocation.
//!
//! Dependencies are checked with `tpt-pulse-ir` before code is generated, so a
//! cyclic graph is rejected at compile time with a `tpt-telos` diagnostic.

use std::collections::{BTreeMap, BTreeSet};

use crate::compose;

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse_quote,
    punctuated::Punctuated,
    visit::{self, Visit},
    visit_mut::{self, VisitMut},
    Expr, ExprPath, GenericArgument, Ident, ItemFn, Pat, PathArguments, Stmt, Token, Type,
};
use tpt_pulse_ir::{OrderGraph, OrderViolation};

/// One signal declared with `let`.
pub(crate) struct Decl {
    name: Ident,
    ty: Option<Type>,
    stmt: usize,
    kind: Kind,
    /// The path that names `Signal` at the call site, such as `Signal` or
    /// `tpt_pulse_signals::Signal`. Kept so the expansion still uses the user's import.
    signal_path: syn::Path,
}

enum Kind {
    /// `Signal::new(init)`.
    Source { init: Expr },
    /// `Signal::map(&a, f)` or `Signal::map2(&a, &b, f)`.
    Derived { closure: Expr, args: Vec<Ident> },
}

/// A `name.set(value);` statement on a source signal.
struct Set {
    stmt: usize,
    target: Ident,
    value: Expr,
}

/// Expand one `#[signal_graph]` function.
/// Expand one `#[signal_graph]` function: a sub-graph, a caller that uses one, or a plain graph.
pub fn expand(item: ItemFn) -> syn::Result<TokenStream> {
    if let Some(call) = compose::subgraph_call(&item)? {
        return Ok(compose::defer(&call, &item));
    }
    if compose::is_subgraph(&item) {
        return compose::expand_subgraph(item);
    }
    lower(item)
}

/// Lower a plain graph: its signals become locals, and `set` recomputes the dependents.
pub(crate) fn lower(item: ItemFn) -> syn::Result<TokenStream> {
    let ItemFn {
        attrs,
        vis,
        sig,
        mut block,
    } = item;
    let mut stmts = std::mem::take(&mut block.stmts);

    let mut decls = Vec::new();
    for (index, stmt) in stmts.iter().enumerate() {
        if let Some(decl) = signal_decl(stmt, index)? {
            decls.push(decl);
        }
    }
    let names: BTreeSet<String> = decls.iter().map(|d| d.name.to_string()).collect();
    if names.len() != decls.len() {
        let duplicate = decls
            .iter()
            .enumerate()
            .find(|(i, d)| decls[..*i].iter().any(|e| e.name == d.name))
            .map(|(_, d)| d.name.clone())
            .expect("a duplicate exists when the name counts differ");
        return Err(syn::Error::new_spanned(
            duplicate,
            "signal declared more than once",
        ));
    }

    let mut sets = Vec::new();
    for (index, stmt) in stmts.iter().enumerate() {
        if let Some((target, value)) = set_call(stmt, &names) {
            sets.push(Set {
                stmt: index,
                target,
                value,
            });
        }
    }

    for decl in &decls {
        match &decl.kind {
            Kind::Source { init } => reject_captures(init, &names, "Signal::new initial value")?,
            Kind::Derived { closure, .. } => reject_captures(closure, &names, "map closure")?,
        }
    }

    let index_of: BTreeMap<String, usize> = decls
        .iter()
        .enumerate()
        .map(|(i, d)| (d.name.to_string(), i))
        .collect();
    let dependents = check_graph(&decls, &index_of)?;

    let set_stmts: BTreeSet<usize> = sets.iter().map(|s| s.stmt).collect();
    let decl_stmts: BTreeMap<usize, usize> =
        decls.iter().enumerate().map(|(i, d)| (d.stmt, i)).collect();

    // Rewrite `x.get()` everywhere and the values of `set` statements.
    let mut rewriter = Rewrite {
        names: &names,
        error: None,
    };
    for (index, stmt) in stmts.iter_mut().enumerate() {
        if !decl_stmts.contains_key(&index) && !set_stmts.contains(&index) {
            rewriter.visit_stmt_mut(stmt);
        }
    }
    let mut sets: Vec<Set> = sets;
    for set in &mut sets {
        rewriter.visit_expr_mut(&mut set.value);
    }
    if let Some(error) = rewriter.error {
        return Err(error);
    }

    let mut out = Vec::new();
    if let Some(first) = decls.first() {
        // Keeps the user's `Signal` import in use, since the calls below are lowered away.
        let signal_path = &first.signal_path;
        out.push(quote! {
            let _ = ::core::marker::PhantomData::<#signal_path<()>>;
        });
    }
    for (index, stmt) in stmts.iter().enumerate() {
        if let Some(&decl_index) = decl_stmts.get(&index) {
            out.push(lower_decl(&decls[decl_index]));
        } else if let Some(set) = sets.iter().find(|s| s.stmt == index) {
            out.push(lower_set(set, &decls, &index_of, &dependents)?);
        } else {
            out.push(stmt.to_token_stream());
        }
    }

    Ok(quote! {
        #(#attrs)*
        #vis #sig {
            #(#out)*
        }
    })
}

/// The names of the signals declared by `let` statements, in order.
pub(crate) fn decl_names(stmts: &[Stmt]) -> syn::Result<Vec<Ident>> {
    let mut names = Vec::new();
    for (index, stmt) in stmts.iter().enumerate() {
        if let Some(decl) = signal_decl(stmt, index)? {
            names.push(decl.name);
        }
    }
    Ok(names)
}

/// Parse `let name[: Signal<T>] = Signal::new(..) | map(..) | map2(..);`.
pub(crate) fn signal_decl(stmt: &Stmt, stmt_index: usize) -> syn::Result<Option<Decl>> {
    let Stmt::Local(local) = stmt else {
        return Ok(None);
    };
    let Some(init) = &local.init else {
        return Ok(None);
    };
    let (name, ty) = match &local.pat {
        Pat::Ident(pat) if pat.subpat.is_none() => (pat.ident.clone(), None),
        Pat::Type(pat) => match &*pat.pat {
            Pat::Ident(ident) => (ident.ident.clone(), Some(signal_value_type(&pat.ty)?)),
            _ => return Ok(None),
        },
        _ => return Ok(None),
    };
    let Some((method, args, signal_path)) = signal_call(&init.expr) else {
        return Ok(None);
    };
    let kind = match (method.as_str(), args.len()) {
        ("new", 1) => Kind::Source {
            init: args.into_iter().next().expect("one argument"),
        },
        ("map", 2) => {
            let mut args = args.into_iter();
            let source = signal_ref(&args.next().expect("source"))?;
            Kind::Derived {
                args: vec![source],
                closure: args.next().expect("closure"),
            }
        }
        ("map2", 3) => {
            let mut args = args.into_iter();
            let first = signal_ref(&args.next().expect("first source"))?;
            let second = signal_ref(&args.next().expect("second source"))?;
            Kind::Derived {
                args: vec![first, second],
                closure: args.next().expect("closure"),
            }
        }
        _ => {
            return Err(syn::Error::new_spanned(
                &init.expr,
                "unsupported Signal call; expected Signal::new, Signal::map or Signal::map2",
            ))
        }
    };
    Ok(Some(Decl {
        name,
        ty,
        stmt: stmt_index,
        kind,
        signal_path,
    }))
}

/// `Signal::method(args..)` as its method name, its arguments, and the `Signal` path.
fn signal_call(expr: &Expr) -> Option<(String, Vec<Expr>, syn::Path)> {
    let Expr::Call(call) = expr else {
        return None;
    };
    let Expr::Path(path) = &*call.func else {
        return None;
    };
    let segments = &path.path.segments;
    if segments.len() < 2 || segments[segments.len() - 2].ident != "Signal" {
        return None;
    }
    let method = segments.last()?.ident.to_string();
    let mut signal_path = path.path.clone();
    signal_path.segments.pop();
    signal_path.segments.pop_punct();
    Some((method, call.args.iter().cloned().collect(), signal_path))
}

/// `&name` as the name of a signal argument.
fn signal_ref(expr: &Expr) -> syn::Result<Ident> {
    if let Expr::Reference(reference) = expr {
        if let Expr::Path(path) = &*reference.expr {
            if let Some(ident) = path.path.get_ident() {
                return Ok(ident.clone());
            }
        }
    }
    Err(syn::Error::new_spanned(
        expr,
        "signal arguments must be references to signals, such as `&sensor`",
    ))
}

/// `T` from a type written as `Signal<T>`.
fn signal_value_type(ty: &Type) -> syn::Result<Type> {
    if let Type::Path(path) = ty {
        if let Some(segment) = path.path.segments.last() {
            if segment.ident == "Signal" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(GenericArgument::Type(value)) = args.args.first() {
                        return Ok(value.clone());
                    }
                }
            }
        }
    }
    Err(syn::Error::new_spanned(
        ty,
        "expected a type written as `Signal<T>`",
    ))
}

/// `name.set(value);` on a signal named in this function.
fn set_call(stmt: &Stmt, names: &BTreeSet<String>) -> Option<(Ident, Expr)> {
    let Stmt::Expr(Expr::MethodCall(call), Some(_)) = stmt else {
        return None;
    };
    if call.method != "set" || call.args.len() != 1 {
        return None;
    }
    let target = signal_name(&call.receiver, names)?;
    Some((target, call.args[0].clone()))
}

fn signal_name(expr: &Expr, names: &BTreeSet<String>) -> Option<Ident> {
    let Expr::Path(path) = expr else {
        return None;
    };
    let ident = path.path.get_ident()?;
    names.contains(&ident.to_string()).then(|| ident.clone())
}

/// Reject closures and initial values that read signals directly, because
/// that read would not be a tracked dependency.
fn reject_captures(expr: &Expr, names: &BTreeSet<String>, what: &str) -> syn::Result<()> {
    let mut captures = Captures { names, found: None };
    captures.visit_expr(expr);
    match captures.found {
        Some(ident) => Err(syn::Error::new_spanned(
            ident,
            format!(
                "{what} cannot read a signal directly; pass signals to Signal::map or Signal::map2"
            ),
        )),
        None => Ok(()),
    }
}

/// Validate the dependency graph and return each signal's dependents.
///
/// Acyclicity is checked first, through `tpt-pulse-ir` and `tpt-telos`, so a cycle is
/// reported as a cycle. Only then are forward references rejected.
fn check_graph(decls: &[Decl], index_of: &BTreeMap<String, usize>) -> syn::Result<Vec<Vec<usize>>> {
    let mut graph = OrderGraph::new();
    let events: Vec<usize> = decls
        .iter()
        .map(|decl| graph.event(decl.name.to_string()))
        .collect();
    let mut dependents = vec![Vec::new(); decls.len()];
    for (index, decl) in decls.iter().enumerate() {
        let Kind::Derived { args, .. } = &decl.kind else {
            continue;
        };
        for arg in args {
            let Some(&dependency) = index_of.get(&arg.to_string()) else {
                return Err(syn::Error::new_spanned(
                    arg,
                    format!("unknown signal `{arg}`"),
                ));
            };
            graph.before(events[dependency], events[index]);
            dependents[dependency].push(index);
        }
    }
    match graph.verify() {
        Ok(()) => {}
        Err(OrderViolation::Cycle(cycle)) => {
            let first = &cycle[0];
            let decl = &decls[index_of[first]];
            return Err(syn::Error::new_spanned(
                &decl.name,
                format!(
                    "signal graph is cyclic: {}; rejected by tpt-telos",
                    OrderViolation::Cycle(cycle.clone())
                ),
            ));
        }
        Err(OrderViolation::Undecided) => {
            return Err(syn::Error::new_spanned(
                &decls[0].name,
                "tpt-telos could not decide whether the signal graph is acyclic",
            ));
        }
    }

    for (index, decl) in decls.iter().enumerate() {
        let Kind::Derived { args, .. } = &decl.kind else {
            continue;
        };
        for arg in args {
            if index_of[&arg.to_string()] > index {
                return Err(syn::Error::new_spanned(
                    arg,
                    format!(
                        "`{arg}` is used before it is declared; declare it before `{}`",
                        decl.name
                    ),
                ));
            }
        }
    }
    Ok(dependents)
}

/// The statement that declares a signal as an ordinary local.
fn lower_decl(decl: &Decl) -> TokenStream {
    let name = &decl.name;
    let ty = decl.ty.as_ref().map(|ty| quote!(: #ty));
    let value = match &decl.kind {
        Kind::Source { init } => quote!(#init),
        Kind::Derived { closure, args } => apply(closure, args),
    };
    quote! {
        #[allow(unused_mut)]
        let mut #name #ty = #value;
    }
}

/// `target = value;`, followed by recomputing every derived signal downstream of
/// `target` that is declared before this statement.
fn lower_set(
    set: &Set,
    decls: &[Decl],
    index_of: &BTreeMap<String, usize>,
    dependents: &[Vec<usize>],
) -> syn::Result<TokenStream> {
    let target_index = index_of[&set.target.to_string()];
    if !matches!(decls[target_index].kind, Kind::Source { .. }) {
        return Err(syn::Error::new_spanned(
            &set.target,
            "only signals created with Signal::new can be set",
        ));
    }

    let mut downstream = BTreeSet::new();
    let mut frontier = vec![target_index];
    while let Some(node) = frontier.pop() {
        for &next in &dependents[node] {
            if downstream.insert(next) {
                frontier.push(next);
            }
        }
    }

    let target = &set.target;
    let value = &set.value;
    let recompute = downstream
        .into_iter()
        .filter(|&index| decls[index].stmt < set.stmt)
        .map(|index| {
            let decl = &decls[index];
            let name = &decl.name;
            let Kind::Derived { closure, args } = &decl.kind else {
                unreachable!("only derived signals have dependents");
            };
            let value = apply(closure, args);
            quote!(#name = #value;)
        });
    Ok(quote! {
        {
            #target = #value;
            #(#recompute)*
        }
    })
}

/// Call `closure` on clones of the current values of `args`.
///
/// The `__apply` helpers are generic, so the closure's parameter types are
/// inferred from the arguments. They are inlined like a direct call.
fn apply(closure: &Expr, args: &[Ident]) -> TokenStream {
    let values = args
        .iter()
        .map(|arg| quote!(::core::clone::Clone::clone(&#arg)));
    match args.len() {
        1 => {
            let first = values.collect::<Vec<_>>();
            quote!(::tpt_pulse_signals::__apply1(#closure, #(#first),*))
        }
        _ => {
            let both = values.collect::<Vec<_>>();
            quote!(::tpt_pulse_signals::__apply2(#closure, #(#both),*))
        }
    }
}

/// Replaces `x.get()` with a clone of `x`, and rejects `set` anywhere except a
/// top-level statement on a source signal.
struct Rewrite<'a> {
    names: &'a BTreeSet<String>,
    error: Option<syn::Error>,
}

impl VisitMut for Rewrite<'_> {
    fn visit_macro_mut(&mut self, mac: &mut syn::Macro) {
        // Macro arguments are unparsed tokens to the visitor. Comma-separated
        // expressions, such as `println!("{}", x.get())`, are rewritten too.
        if let Ok(mut args) = mac.parse_body_with(Punctuated::<Expr, Token![,]>::parse_terminated) {
            for arg in args.iter_mut() {
                self.visit_expr_mut(arg);
            }
            mac.tokens = args.to_token_stream();
        }
    }

    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        visit_mut::visit_expr_mut(self, expr);
        let replacement = match &*expr {
            Expr::MethodCall(call) => match signal_name(&call.receiver, self.names) {
                Some(name) if call.method == "get" && call.args.is_empty() => {
                    Some(parse_quote!(::core::clone::Clone::clone(&#name)))
                }
                Some(_) if call.method == "set" => {
                    self.error.get_or_insert_with(|| {
                        syn::Error::new_spanned(
                            call,
                            "`set` must be a top-level statement on a signal created with Signal::new",
                        )
                    });
                    None
                }
                _ => None,
            },
            _ => None,
        };
        if let Some(replacement) = replacement {
            *expr = replacement;
        }
    }
}

/// Finds a signal name used inside a closure or initial value.
struct Captures<'a> {
    names: &'a BTreeSet<String>,
    found: Option<Ident>,
}

impl<'ast> Visit<'ast> for Captures<'_> {
    fn visit_expr_path(&mut self, path: &'ast ExprPath) {
        if let Some(ident) = path.path.get_ident() {
            if self.found.is_none() && self.names.contains(&ident.to_string()) {
                self.found = Some(ident.clone());
            }
        }
        visit::visit_expr_path(self, path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expand_source(source: &str) -> syn::Result<String> {
        let item: ItemFn = syn::parse_str(source)?;
        expand(item).map(|tokens| tokens.to_string())
    }

    #[test]
    fn set_recomputes_downstream_signals() {
        let output = expand_source(
            "fn f() -> f64 {
                let a = Signal::new(0.0);
                let b = Signal::new(0.0);
                let c = Signal::map2(&a, &b, |x, y| x + y);
                let m = Signal::map(&c, |v| v * 0.5);
                a.set(10.0);
                m.get()
            }",
        )
        .expect("valid signal graph");
        assert!(output.contains("__apply2"));
        assert!(
            output.contains("m = :: tpt_pulse_signals :: __apply1"),
            "{output}"
        );
    }

    #[test]
    fn cyclic_graph_is_rejected_by_tpt_telos() {
        let error = expand_source(
            "fn f() {
                let a = Signal::map(&b, |v| v);
                let b = Signal::map(&a, |v| v);
            }",
        )
        .expect_err("cycle must be rejected");
        assert!(error.to_string().contains("cyclic"), "{error}");
        assert!(error.to_string().contains("tpt-telos"), "{error}");
    }

    #[test]
    fn closure_may_not_read_a_signal() {
        let error = expand_source(
            "fn f() {
                let a = Signal::new(1.0);
                let b = Signal::map(&a, |v| v + a.get());
            }",
        )
        .expect_err("capture must be rejected");
        assert!(
            error.to_string().contains("cannot read a signal"),
            "{error}"
        );
    }

    #[test]
    fn set_on_derived_signal_is_rejected() {
        let error = expand_source(
            "fn f() {
                let a = Signal::new(1.0);
                let b = Signal::map(&a, |v| v);
                b.set(2.0);
            }",
        )
        .expect_err("derived set must be rejected");
        assert!(
            error.to_string().contains("only signals created"),
            "{error}"
        );
    }
}
