//! @path: karst/crates/karst_expander/expand.rs
//! @author: redskaber
//! @datetime: 2026-10-04
//! @discription: karst::crates::karst_expander::expand

use karst_core::{Binding, CoreError, CoreExpr, HandlerClause, LiteralValue, ModuleItem};
use karst_span::Span;
use karst_syntax::{Phase, StxDatum, SyntaxObject};

use crate::ctx::{ExpandCtx, Resolution};
use crate::error::ExpandError;

fn symbol_text(stx: &SyntaxObject) -> Option<&str> {
    match &stx.datum {
        StxDatum::Symbol(sym) => Some(sym.as_str()),
        _ => None,
    }
}

fn symbol_texts(node: &SyntaxObject) -> Option<Vec<&str>> {
    let items = match &node.datum {
        StxDatum::List(items) => items,
        _ => return None,
    };
    let mut texts = Vec::with_capacity(items.len());
    for item in items {
        texts.push(symbol_text(item)?);
    }
    Some(texts)
}

fn malformed(form: &'static str, span: Span) -> ExpandError {
    ExpandError::MalformedForm { form, span }
}

fn core_err(source: CoreError, span: Span) -> ExpandError {
    ExpandError::Core { source, span }
}

/// expand entry core key
pub fn expand(ctx: &mut ExpandCtx, stx: &SyntaxObject) -> Result<CoreExpr, ExpandError> {
    ctx.enter(stx.span)?;
    let result = expand_node(ctx, stx);
    ctx.exit();
    result
}

/// Expand a whole program: top-level forms -> module items
pub fn expand_program(forms: &[SyntaxObject]) -> Result<Vec<ModuleItem>, ExpandError> {
    let mut ctx = ExpandCtx::new();
    expand_declarations(&mut ctx, forms)
}

fn require_runtime(stx: &SyntaxObject) -> Result<(), ExpandError> {
    if stx.phase != Phase::Runtime {
        return Err(ExpandError::PhaseViolation {
            phase: stx.phase,
            span: stx.span,
        });
    }
    Ok(())
}

fn expand_node(ctx: &mut ExpandCtx, stx: &SyntaxObject) -> Result<CoreExpr, ExpandError> {
    require_runtime(stx)?;
    match &stx.datum {
        StxDatum::Int(v) => Ok(CoreExpr::Literal {
            span: stx.span,
            value: LiteralValue::Int(*v),
        }),
        StxDatum::Float(v) => Ok(CoreExpr::Literal {
            span: stx.span,
            value: LiteralValue::Float(*v),
        }),
        StxDatum::Bool(v) => Ok(CoreExpr::Literal {
            span: stx.span,
            value: LiteralValue::Bool(*v),
        }),
        StxDatum::String(s) => Ok(CoreExpr::Literal {
            span: stx.span,
            value: LiteralValue::String(s.clone()),
        }),
        StxDatum::Symbol(_) => {
            let text = symbol_text(stx).expect("datum is a symbol");
            expand_symbol(ctx, text, stx)
        }
        StxDatum::List(items) => expand_list(ctx, items, stx.span),
    }
}

fn expand_symbol(
    ctx: &mut ExpandCtx,
    text: &str,
    stx: &SyntaxObject,
) -> Result<CoreExpr, ExpandError> {
    // handler 'nil'
    if text == "nil" {
        return Ok(CoreExpr::Literal {
            span: stx.span,
            value: LiteralValue::Nil,
        });
    }
    //var
    match ctx.resolve(text, &stx.scopes) {
        Resolution::Local | Resolution::Global => {
            CoreExpr::var(stx.span, text).map_err(|s| core_err(s, stx.span))
        }
        Resolution::Unbound => Err(ExpandError::UnboundIdentifier {
            name: text.to_owned(),
            span: stx.span,
        }),
    }
}

fn expand_list(
    ctx: &mut ExpandCtx,
    items: &[SyntaxObject],
    span: Span,
) -> Result<CoreExpr, ExpandError> {
    if items.is_empty() {
        return Err(ExpandError::EmptyForm { span });
    }
    require_runtime(&items[0])?;
    if let Some(head) = symbol_text(&items[0]) {
        match head {
            "fn" => return expand_fn(ctx, items, span),
            "if" => return expand_if(ctx, items, span),
            "let" => return expand_let(ctx, items, span),
            "do" => return expand_do(ctx, items, span),
            "set!" => return expand_set(ctx, items, span), // assgin == set!
            "perform" => return expand_perform(ctx, items, span),
            "handle" => return expand_handle(ctx, items, span),
            _ => {}
        }
    }
    expand_application(ctx, items, span)
}

fn expand_fn(
    ctx: &mut ExpandCtx,
    items: &[SyntaxObject],
    span: Span,
) -> Result<CoreExpr, ExpandError> {
    // (fn (params ...) body)
    if items.len() != 3 {
        return Err(malformed("fn", span));
    }
    // params
    require_runtime(&items[1])?;
    let params = symbol_texts(&items[1]).ok_or_else(|| malformed("fn", span))?;
    // body
    require_runtime(&items[2])?;
    expand_fn_binding(ctx, &params, &items[2], span)
}

fn expand_fn_binding(
    ctx: &mut ExpandCtx,
    params: &[&str],
    body_node: &SyntaxObject,
    fn_span: Span,
) -> Result<CoreExpr, ExpandError> {
    let scope_id = ctx.fresh_scope();
    ctx.push_frame(scope_id);
    let result = (|| {
        for param in params {
            ctx.bind(param);
        }
        let colored_body = body_node.with_scope_added(scope_id);
        let body = expand(ctx, &colored_body)?;
        CoreExpr::fn_(fn_span, params, body).map_err(|s| core_err(s, fn_span))
    })();
    ctx.pop_frame();
    result
}

fn expand_if(
    ctx: &mut ExpandCtx,
    items: &[SyntaxObject],
    span: Span,
) -> Result<CoreExpr, ExpandError> {
    // (if cond then else)
    if items.len() != 4 {
        return Err(malformed("if", span));
    }
    for operand in &items[1..] {
        require_runtime(operand)?;
    }
    let cond = expand(ctx, &items[1])?;
    let then_branch = expand(ctx, &items[2])?;
    let else_branch = expand(ctx, &items[3])?;
    Ok(CoreExpr::If {
        span,
        cond: Box::new(cond),
        then_branch: Box::new(then_branch),
        else_branch: Box::new(else_branch),
    })
}
fn expand_let(
    ctx: &mut ExpandCtx,
    items: &[SyntaxObject],
    span: Span,
) -> Result<CoreExpr, ExpandError> {
    // (let spec body) with spec `()` | `(name value)` | `((name value) ...)`
    if items.len() != 3 {
        return Err(malformed("let", span));
    }
    require_runtime(&items[1])?;
    require_runtime(&items[2])?;
    // spec
    let spec = match &items[1].datum {
        StxDatum::List(spec) => spec,
        _ => return Err(malformed("let", span)),
    };
    let mut pairs: Vec<(&str, &SyntaxObject)> = Vec::with_capacity(spec.len());
    if spec.is_empty() {
    } else if matches!(spec[0].datum, StxDatum::List(_)) {
        for pair in spec {
            require_runtime(pair)?;
            let pair_items = match &pair.datum {
                StxDatum::List(pair_items) if pair_items.len() == 2 => pair_items,
                _ => return Err(malformed("let", span)),
            };
            require_runtime(&pair_items[0])?;
            let name = symbol_text(&pair_items[0]).ok_or_else(|| malformed("let", span))?;
            pairs.push((name, &pair_items[1]));
        }
    } else {
        if spec.len() != 2 {
            return Err(malformed("let", span));
        }
        require_runtime(&spec[0])?;
        let name = symbol_text(&spec[0]).ok_or_else(|| malformed("let", span))?;
        pairs.push((name, &spec[1]));
    }
    // colored
    let scope_id = ctx.fresh_scope();
    ctx.push_frame(scope_id);
    let result = (|| {
        let mut bindings = Vec::with_capacity(pairs.len());
        for (name, value_node) in pairs {
            require_runtime(value_node)?;
            let colored_value = value_node.with_scope_added(scope_id);
            let value = expand(ctx, &colored_value)?;
            bindings.push(
                Binding::new(value_node.span, name, value)
                    .map_err(|s| core_err(s, value_node.span))?,
            );
            ctx.bind(name);
        }
        let colored_body = items[2].with_scope_added(scope_id);
        let body = expand(ctx, &colored_body)?;
        Ok(CoreExpr::Let {
            span,
            bindings,
            body: Box::new(body),
        })
    })();
    ctx.pop_frame();
    result
}

fn expand_do(
    ctx: &mut ExpandCtx,
    items: &[SyntaxObject],
    span: Span,
) -> Result<CoreExpr, ExpandError> {
    // (do e1 ... en) seq
    if items.len() < 2 {
        return Err(malformed("do", span));
    }
    if items.len() == 2 {
        require_runtime(&items[1])?;
        return expand(ctx, &items[1]);
    }
    for node in &items[1..] {
        require_runtime(node)?;
    }
    let scope_id = ctx.fresh_scope();
    ctx.push_frame(scope_id);
    let result = (|| {
        let mut bindings = Vec::with_capacity(items.len() - 2);
        for step in &items[1..items.len() - 1] {
            let colored_step = step.with_scope_added(scope_id);
            let value = expand(ctx, &colored_step)?;
            bindings.push(Binding::new(step.span, "_", value).map_err(|s| core_err(s, step.span))?);
            ctx.bind("_");
        }
        let body_node = items.last().expect("do form has at least 3 items");
        let colored_body = body_node.with_scope_added(scope_id);
        let body = expand(ctx, &colored_body)?;
        Ok(CoreExpr::Let {
            span,
            bindings,
            body: Box::new(body),
        })
    })();
    ctx.pop_frame();
    result
}

fn expand_set(
    ctx: &mut ExpandCtx,
    items: &[SyntaxObject],
    span: Span,
) -> Result<CoreExpr, ExpandError> {
    // (set! name value) => assgin
    if items.len() != 3 {
        return Err(malformed("set!", span));
    }
    require_runtime(&items[1])?;
    require_runtime(&items[2])?;
    let name = symbol_text(&items[1]).ok_or_else(|| malformed("set!", span))?;
    match ctx.resolve(name, &items[1].scopes) {
        Resolution::Local | Resolution::Global => {
            let value = expand(ctx, &items[2])?;
            CoreExpr::assign(span, name, value).map_err(|s| core_err(s, span))
        }
        Resolution::Unbound => Err(ExpandError::UnboundIdentifier {
            name: name.to_owned(),
            span: items[1].span,
        }),
    }
}

fn expand_perform(
    ctx: &mut ExpandCtx,
    items: &[SyntaxObject],
    span: Span,
) -> Result<CoreExpr, ExpandError> {
    // (perform effect args ...)
    if items.len() < 2 {
        return Err(malformed("perform", span));
    }
    require_runtime(&items[1])?;
    let effect = symbol_text(&items[1]).ok_or_else(|| malformed("perform", span))?;
    let mut args = Vec::with_capacity(items.len() - 2);
    for arg in &items[2..] {
        require_runtime(arg)?;
        args.push(expand(ctx, arg)?);
    }
    CoreExpr::perform(span, effect, args).map_err(|s| core_err(s, span))
}

fn expand_handle(
    ctx: &mut ExpandCtx,
    items: &[SyntaxObject],
    span: Span,
) -> Result<CoreExpr, ExpandError> {
    // (handle body (effect handler) ...)
    if items.len() < 2 {
        return Err(malformed("handle", span));
    }
    require_runtime(&items[1])?;
    let body = expand(ctx, &items[1])?;
    let mut handlers = Vec::with_capacity(items.len() - 2);
    for handle in &items[2..] {
        require_runtime(handle)?;
        let pair = match &handle.datum {
            StxDatum::List(pair) if pair.len() == 2 => pair,
            _ => return Err(malformed("handle", span)),
        };
        require_runtime(&pair[0])?;
        require_runtime(&pair[1])?;
        let effect = symbol_text(&pair[0]).ok_or_else(|| malformed("handle", span))?;
        let handler = expand(ctx, &pair[1])?;
        handlers
            .push(HandlerClause::new(handle.span, effect, handler).map_err(|s| core_err(s, span))?);
    }
    CoreExpr::handle(span, body, handlers).map_err(|s| core_err(s, span))
}
fn expand_application(
    ctx: &mut ExpandCtx,
    items: &[SyntaxObject],
    span: Span,
) -> Result<CoreExpr, ExpandError> {
    // (func args ...)
    let func = expand(ctx, &items[0])?;
    let mut args = Vec::with_capacity(items.len() - 1);
    for arg in &items[1..] {
        args.push(expand(ctx, arg)?);
    }
    Ok(CoreExpr::Apply {
        span,
        func: Box::new(func),
        args,
    })
}

fn expand_declarations(
    ctx: &mut ExpandCtx,
    forms: &[SyntaxObject],
) -> Result<Vec<ModuleItem>, ExpandError> {
    let mod_scope_id = ctx.fresh_scope();
    ctx.push_frame(mod_scope_id);
    let result = (|| {
        let mut items: Vec<ModuleItem> = Vec::with_capacity(forms.len());
        for form in forms {
            require_runtime(form)?;
            let colored_form = form.with_scope_added(mod_scope_id);
            items.push(expand_declaration(ctx, &colored_form)?);
        }
        Ok(items)
    })();
    ctx.pop_frame();
    result
}

fn expand_declaration(ctx: &mut ExpandCtx, form: &SyntaxObject) -> Result<ModuleItem, ExpandError> {
    let items = match &form.datum {
        StxDatum::List(items) => items,
        _ => return Err(ExpandError::TopLevelExpression { span: form.span }),
    };
    require_runtime(&items[0])?;
    match symbol_text(&items[0]) {
        Some("define") => expand_define(ctx, items, form.span),
        Some("require") => expand_require(items, form.span),
        Some("module") => expand_module(ctx, items, form.span),
        _ => Err(ExpandError::TopLevelExpression { span: form.span }),
    }
}

fn expand_define(
    ctx: &mut ExpandCtx,
    items: &[SyntaxObject],
    span: Span,
) -> Result<ModuleItem, ExpandError> {
    // (define name value) | (define (func params ...) body)
    if items.len() != 3 {
        return Err(malformed("define", span));
    }
    require_runtime(&items[1])?;
    match &items[1].datum {
        StxDatum::List(name_params) if !name_params.is_empty() => {
            require_runtime(&name_params[0])?;
            let name = symbol_text(&name_params[0]).ok_or_else(|| malformed("define", span))?;
            let mut params = Vec::with_capacity(name_params.len() - 1);
            for param in &name_params[1..] {
                require_runtime(param)?;
                params.push(symbol_text(param).ok_or_else(|| malformed("define", span))?);
            }
            require_runtime(&items[2])?;
            ctx.bind(name);
            let value = expand_fn_binding(ctx, &params, &items[2], name_params[0].span)?;
            ModuleItem::define(span, name, value).map_err(|s| core_err(s, span))
        }
        StxDatum::Symbol(_) => {
            let name = symbol_text(&items[1]).expect("datum is a symbol");
            require_runtime(&items[2])?;
            ctx.bind(name);
            let value = expand(ctx, &items[2])?;
            ModuleItem::define(span, name, value).map_err(|s| core_err(s, span))
        }
        _ => Err(malformed("define", span)),
    }
}

fn expand_require(items: &[SyntaxObject], span: Span) -> Result<ModuleItem, ExpandError> {
    // (require "module")
    if items.len() != 2 {
        return Err(malformed("require", span));
    }
    require_runtime(&items[1])?;
    match &items[1].datum {
        StxDatum::String(module) => {
            ModuleItem::require(span, module).map_err(|s| core_err(s, span))
        }
        _ => Err(malformed("require", span)),
    }
}

fn expand_module(
    ctx: &mut ExpandCtx,
    items: &[SyntaxObject],
    span: Span,
) -> Result<ModuleItem, ExpandError> {
    // (module name items ...)
    if items.len() < 3 {
        return Err(malformed("module", span));
    }
    require_runtime(&items[1])?;
    let name = symbol_text(&items[1]).ok_or_else(|| malformed("module", span))?;
    let saved = ctx.isolate_frames();
    let result = (|| {
        let items = expand_declarations(ctx, &items[2..])?;
        ModuleItem::module(span, name, items).map_err(|s| core_err(s, span))
    })();
    ctx.restore_frames(saved);
    result
}

#[cfg(test)]
mod tests {
    //! Unit tests for the expansion loop (sub-stage test doc FP2-FP4).

    use super::*;
    use karst_span::{ByteOffset, ExpansionId, FileId};
    use karst_syntax::ScopeSet;

    fn sp(start: u32, end: u32) -> Span {
        Span::new(
            FileId(0),
            ByteOffset(start),
            ByteOffset(end),
            ExpansionId::ROOT,
        )
        .expect("test fixture span must be well-formed")
    }

    fn sym(start: u32, end: u32, text: &str) -> SyntaxObject {
        SyntaxObject::symbol(text, sp(start, end), ScopeSet::new(), Phase::Runtime)
            .expect("test fixture symbol must be valid")
    }

    fn int(start: u32, end: u32, value: i64) -> SyntaxObject {
        SyntaxObject::int(value, sp(start, end), ScopeSet::new(), Phase::Runtime)
    }

    fn lst(start: u32, end: u32, items: Vec<SyntaxObject>) -> SyntaxObject {
        SyntaxObject::list(items, sp(start, end), ScopeSet::new(), Phase::Runtime)
    }

    // ---------- positive ----------

    #[test]
    fn atoms_expand_to_their_literal_kinds() {
        let mut ctx = ExpandCtx::new();
        let cases = [
            (int(0, 2, 42), "literal"),
            (
                SyntaxObject::float(2.5, sp(0, 3), ScopeSet::new(), Phase::Runtime),
                "literal",
            ),
            (
                SyntaxObject::bool(true, sp(0, 2), ScopeSet::new(), Phase::Runtime),
                "literal",
            ),
            (
                SyntaxObject::string("s".to_owned(), sp(0, 3), ScopeSet::new(), Phase::Runtime),
                "literal",
            ),
            // `nil` is the reserved literal keyword.
            (sym(0, 3, "nil"), "literal"),
        ];
        for (stx, kind) in &cases {
            let expr = expand(&mut ctx, stx).expect("atom must expand");
            assert_eq!(expr.kind_name(), *kind);
        }
        let nil_expr = expand(&mut ctx, &sym(0, 3, "nil")).expect("nil expands");
        assert_eq!(
            nil_expr,
            CoreExpr::Literal {
                span: sp(0, 3),
                value: LiteralValue::Nil
            }
        );
    }

    #[test]
    fn param_reference_resolves_through_coloring() {
        let mut ctx = ExpandCtx::new();
        // (fn (x) x)
        let form = lst(
            0,
            9,
            vec![
                sym(0, 2, "fn"),
                lst(4, 7, vec![sym(5, 6, "x")]),
                sym(8, 9, "x"),
            ],
        );
        let expr = expand(&mut ctx, &form).expect("fn form must expand");
        match expr {
            CoreExpr::Fn { params, body, .. } => {
                assert_eq!(params.len(), 1);
                assert_eq!(params[0].as_str(), "x");
                assert_eq!(*body, CoreExpr::var(sp(8, 9), "x").unwrap());
            }
            other => panic!("expected fn, got {other}"),
        }
    }

    #[test]
    fn let_binds_progressively_like_let_star() {
        let mut ctx = ExpandCtx::new();
        // (let ((a 1) (b a)) b) — b's value sees a (let* order).
        let form = lst(
            0,
            22,
            vec![
                sym(0, 3, "let"),
                lst(
                    5,
                    15,
                    vec![
                        lst(6, 10, vec![sym(7, 8, "a"), int(9, 10, 1)]),
                        lst(12, 15, vec![sym(13, 14, "b"), sym(14, 15, "a")]),
                    ],
                ),
                sym(18, 19, "b"),
            ],
        );
        let expr = expand(&mut ctx, &form).expect("let form must expand");
        match expr {
            CoreExpr::Let { bindings, body, .. } => {
                assert_eq!(bindings.len(), 2);
                assert_eq!(bindings[0].name.as_str(), "a");
                assert_eq!(bindings[1].name.as_str(), "b");
                // b's value is a Var reference to a — visible in let* order.
                assert_eq!(bindings[1].value, CoreExpr::var(sp(14, 15), "a").unwrap());
                assert_eq!(*body, CoreExpr::var(sp(18, 19), "b").unwrap());
            }
            other => panic!("expected let, got {other}"),
        }
    }

    #[test]
    fn do_desugars_to_discard_let_chain() {
        let mut ctx = ExpandCtx::new();
        // (do 1 2) → Let{[_ ← 1], 2}
        let form = lst(0, 8, vec![sym(0, 2, "do"), int(3, 4, 1), int(6, 7, 2)]);
        let expr = expand(&mut ctx, &form).expect("do form must expand");
        match expr {
            CoreExpr::Let { bindings, body, .. } => {
                assert_eq!(bindings.len(), 1);
                assert_eq!(bindings[0].name.as_str(), "_");
                assert_eq!(
                    bindings[0].value,
                    CoreExpr::Literal {
                        span: sp(3, 4),
                        value: LiteralValue::Int(1)
                    }
                );
                assert_eq!(
                    *body,
                    CoreExpr::Literal {
                        span: sp(6, 7),
                        value: LiteralValue::Int(2)
                    }
                );
            }
            other => panic!("expected let, got {other}"),
        }
        // (do e) is the identity.
        let single = lst(0, 6, vec![sym(0, 2, "do"), int(3, 4, 9)]);
        let expr = expand(&mut ctx, &single).expect("single-form do expands");
        assert_eq!(
            expr,
            CoreExpr::Literal {
                span: sp(3, 4),
                value: LiteralValue::Int(9)
            }
        );
    }

    #[test]
    fn fn_if_apply_compose_the_fib_shape() {
        let mut ctx = ExpandCtx::new();
        // (fn (n) (if (< n 2) n (+ n 1))) — globals `<` and `+`.
        let form = lst(
            0,
            30,
            vec![
                sym(0, 2, "fn"),
                lst(4, 7, vec![sym(5, 6, "n")]),
                lst(
                    8,
                    29,
                    vec![
                        sym(9, 11, "if"),
                        lst(
                            13,
                            19,
                            vec![sym(14, 15, "<"), sym(16, 17, "n"), int(18, 19, 2)],
                        ),
                        sym(21, 22, "n"),
                        lst(
                            24,
                            29,
                            vec![sym(25, 26, "+"), sym(27, 28, "n"), int(28, 29, 1)],
                        ),
                    ],
                ),
            ],
        );
        let expr = expand(&mut ctx, &form).expect("fib shape must expand");
        match expr {
            CoreExpr::Fn { body, .. } => match *body {
                CoreExpr::If {
                    cond,
                    then_branch,
                    else_branch,
                    ..
                } => {
                    assert!(matches!(*cond, CoreExpr::Apply { .. }));
                    assert_eq!(*then_branch, CoreExpr::var(sp(21, 22), "n").unwrap());
                    assert!(matches!(*else_branch, CoreExpr::Apply { .. }));
                }
                other => panic!("expected if body, got {other}"),
            },
            other => panic!("expected fn, got {other}"),
        }
    }

    // ---------- negative ----------

    #[test]
    fn unbound_symbol_fails_with_name_and_span() {
        let mut ctx = ExpandCtx::new();
        let err = expand(&mut ctx, &sym(7, 10, "fib")).expect_err("fib must be unbound");
        assert_eq!(
            err,
            ExpandError::UnboundIdentifier {
                name: "fib".to_owned(),
                span: sp(7, 10)
            }
        );
    }

    #[test]
    fn unbound_in_operand_position_fails() {
        let mut ctx = ExpandCtx::new();
        // (+ 1 undefined-thing)
        let form = lst(
            0,
            21,
            vec![sym(1, 2, "+"), int(4, 5, 1), sym(7, 21, "undefined-thing")],
        );
        let err = expand(&mut ctx, &form).expect_err("operand must be unbound");
        match err {
            ExpandError::UnboundIdentifier { name, span } => {
                assert_eq!(name, "undefined-thing");
                assert_eq!(span, sp(7, 21));
            }
            other => panic!("expected unbound identifier, got {other}"),
        }
    }

    #[test]
    fn empty_synthesized_form_is_rejected() {
        let mut ctx = ExpandCtx::new();
        let form = lst(0, 2, vec![]);
        let err = expand(&mut ctx, &form).expect_err("empty form must fail");
        assert_eq!(err, ExpandError::EmptyForm { span: sp(0, 2) });
    }

    #[test]
    fn macro_phase_syntax_is_rejected() {
        let mut ctx = ExpandCtx::new();
        let form = SyntaxObject::symbol("x", sp(0, 3), ScopeSet::new(), Phase::ExpandTime)
            .expect("fixture symbol must be valid");
        let err = expand(&mut ctx, &form).expect_err("macro-phase must fail");
        assert_eq!(
            err,
            ExpandError::PhaseViolation {
                phase: Phase::ExpandTime,
                span: sp(0, 3)
            }
        );
    }

    #[test]
    fn fn_without_body_is_malformed() {
        let mut ctx = ExpandCtx::new();
        // (fn (x))
        let form = lst(0, 8, vec![sym(0, 2, "fn"), lst(4, 7, vec![sym(5, 6, "x")])]);
        let err = expand(&mut ctx, &form).expect_err("fn without body must fail");
        assert_eq!(err, malformed("fn", sp(0, 8)));
    }

    #[test]
    fn if_with_two_operands_is_malformed() {
        let mut ctx = ExpandCtx::new();
        // (if 1 2)
        let form = lst(0, 8, vec![sym(0, 2, "if"), int(4, 5, 1), int(6, 7, 2)]);
        let err = expand(&mut ctx, &form).expect_err("two-armed if must fail");
        assert_eq!(err, malformed("if", sp(0, 8)));
    }

    #[test]
    fn let_with_short_binding_pair_is_malformed() {
        let mut ctx = ExpandCtx::new();
        // (let ((x)) x)
        let form = lst(
            0,
            13,
            vec![
                sym(0, 3, "let"),
                lst(5, 9, vec![lst(6, 9, vec![sym(7, 8, "x")])]),
                sym(11, 12, "x"),
            ],
        );
        let err = expand(&mut ctx, &form).expect_err("short pair must fail");
        assert_eq!(err, malformed("let", sp(0, 13)));
    }

    #[test]
    fn duplicate_params_reassert_via_core() {
        let mut ctx = ExpandCtx::new();
        // (fn (x x) x)
        let form = lst(
            0,
            12,
            vec![
                sym(0, 2, "fn"),
                lst(4, 9, vec![sym(5, 6, "x"), sym(7, 8, "x")]),
                sym(11, 12, "x"),
            ],
        );
        let err = expand(&mut ctx, &form).expect_err("duplicate params must fail");
        match err {
            ExpandError::Core { source, .. } => {
                assert_eq!(
                    source,
                    karst_core::CoreError::DuplicateParam {
                        name: "x".to_owned()
                    }
                );
            }
            other => panic!("expected core error, got {other}"),
        }
    }

    #[test]
    fn duplicate_handlers_reassert_via_core() {
        let mut ctx = ExpandCtx::new();
        // (handle 1 (E 2) (E 3)) — literal handlers keep the
        // duplicate check the first failure.
        let form = lst(
            0,
            20,
            vec![
                sym(0, 6, "handle"),
                int(7, 8, 1),
                lst(9, 14, vec![sym(10, 11, "E"), int(13, 14, 2)]),
                lst(15, 20, vec![sym(16, 17, "E"), int(18, 19, 3)]),
            ],
        );
        let err = expand(&mut ctx, &form).expect_err("duplicate handlers must fail");
        match err {
            ExpandError::Core { source, .. } => {
                assert_eq!(
                    source,
                    karst_core::CoreError::DuplicateHandler {
                        effect: "E".to_owned()
                    }
                );
            }
            other => panic!("expected core error, got {other}"),
        }
    }

    #[test]
    fn bare_top_level_expression_is_rejected() {
        // (fib 10) at module top level.
        let form = lst(0, 8, vec![sym(1, 4, "fib"), int(5, 7, 10)]);
        let err = expand_program(&[form]).expect_err("bare expression must fail");
        assert_eq!(err, ExpandError::TopLevelExpression { span: sp(0, 8) });
    }

    #[test]
    fn set_to_unbound_target_fails() {
        let mut ctx = ExpandCtx::new();
        // (set! nope 1)
        let form = lst(
            0,
            12,
            vec![sym(0, 4, "set!"), sym(5, 9, "nope"), int(10, 11, 1)],
        );
        let err = expand(&mut ctx, &form).expect_err("unbound set! target must fail");
        assert_eq!(
            err,
            ExpandError::UnboundIdentifier {
                name: "nope".to_owned(),
                span: sp(5, 9)
            }
        );
    }

    #[test]
    fn deep_tree_hits_the_expansion_budget_exactly() {
        let build = |depth: usize| {
            let mut node = int(0, 1, 42);
            for _ in 0..depth {
                node = lst(0, 2, vec![sym(0, 1, "+"), node]);
            }
            node
        };
        let mut ctx = ExpandCtx::new();
        let within = build(crate::EXPANSION_LIMIT - 1);
        expand(&mut ctx, &within).expect("within-budget tree must expand");
        let beyond = build(crate::EXPANSION_LIMIT);
        let err = expand(&mut ctx, &beyond).expect_err("over-budget tree must fail");
        assert_eq!(
            err,
            ExpandError::ExpansionDepthExceeded {
                limit: crate::EXPANSION_LIMIT,
                span: sp(0, 1)
            }
        );
    }
}
