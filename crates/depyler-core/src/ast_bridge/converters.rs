use crate::hir::*;
use anyhow::{Result, bail};
use rustpython_ast::{self as ast, Ranged};

/// Context for tracking source spans during conversion
#[derive(Clone, Default)]
pub struct SpanContext {
    source: Option<String>,
}

impl SpanContext {
    pub fn new() -> Self {
        Self { source: None }
    }

    pub fn with_source(source: impl Into<String>) -> Self {
        Self {
            source: Some(source.into()),
        }
    }

    /// Extract a span from an AST node that implements Ranged
    pub fn span_from<T: Ranged>(&self, node: &T) -> Option<Span> {
        self.source
            .as_ref()
            .map(|src| Span::from_text_range(node.range(), src))
    }
}

/// Statement converter to reduce complexity
pub struct StmtConverter;

impl StmtConverter {
    pub fn convert(stmt: ast::Stmt) -> Result<HirStmt> {
        match stmt {
            ast::Stmt::AnnAssign(a) => Self::convert_ann_assign(a),
            ast::Stmt::Assert(a) => Self::convert_assert(a),
            ast::Stmt::Assign(a) => Self::convert_assign(a),
            ast::Stmt::AsyncFor(af) => Self::convert_async_for(af),
            ast::Stmt::AsyncFunctionDef(f) => Self::convert_async_function_def(f),
            ast::Stmt::AsyncWith(aw) => Self::convert_async_with(aw),
            ast::Stmt::AugAssign(a) => Self::convert_aug_assign(a),
            ast::Stmt::Break(b) => Self::convert_break(b),
            ast::Stmt::ClassDef(_) => bail!("Statement type not yet supported: ClassDef (classes)"),
            ast::Stmt::Continue(c) => Self::convert_continue(c),
            ast::Stmt::Delete(d) => Self::convert_delete(d),
            ast::Stmt::Expr(e) => Self::convert_expr_stmt(e),
            ast::Stmt::For(f) => Self::convert_for(f),
            ast::Stmt::FunctionDef(f) => Self::convert_nested_function_def(f),
            ast::Stmt::Global(g) => Self::convert_global(g),
            ast::Stmt::If(i) => Self::convert_if(i),
            ast::Stmt::Import(i) => Self::convert_import(i),
            ast::Stmt::ImportFrom(i) => Self::convert_import_from(i),
            ast::Stmt::Match(m) => Self::convert_match(m),
            ast::Stmt::Nonlocal(n) => Self::convert_nonlocal(n),
            ast::Stmt::Pass(_) => Self::convert_pass(),
            ast::Stmt::Raise(r) => Self::convert_raise(r),
            ast::Stmt::Return(r) => Self::convert_return(r),
            ast::Stmt::Try(t) => Self::convert_try(t),
            ast::Stmt::While(w) => Self::convert_while(w),
            ast::Stmt::With(w) => Self::convert_with(w),
            _ => bail!("Statement type not yet supported: unknown"),
        }
    }

    /// Convert a statement with source span tracking
    pub fn convert_with_span(stmt: ast::Stmt, ctx: &SpanContext) -> Result<SpannedStmt> {
        let span = ctx.span_from(&stmt);
        let inner = Self::convert(stmt)?;
        Ok(Spanned { inner, span })
    }

    fn convert_assign(a: ast::StmtAssign) -> Result<HirStmt> {
        if a.targets.len() == 1 {
            let target = extract_assign_target(&a.targets[0])?;
            let value = ExprConverter::convert(*a.value)?;
            Ok(HirStmt::Assign {
                target,
                value,
                type_annotation: None,
            })
        } else {
            // Chained assignment like a = b = c = value is not supported
            anyhow::bail!(
                "Chained assignment (a = b = c = value) is not supported. Use separate assignments instead."
            )
        }
    }

    fn convert_ann_assign(a: ast::StmtAnnAssign) -> Result<HirStmt> {
        let target = extract_assign_target(&a.target)?;

        // Extract type annotation
        let type_annotation = Some(super::type_extraction::TypeExtractor::extract_type(
            &a.annotation,
        )?);

        // Handle annotated assignments without values (e.g., `x: int` or `field: CustomType`)
        // Python allows type annotations without initialization. Represent such
        // declarations with a special Uninitialized expression so the Rust
        // generator can emit a declaration without an initializer.
        let value = if let Some(v) = a.value {
            ExprConverter::convert(*v)?
        } else {
            HirExpr::Uninitialized
        };

        Ok(HirStmt::Assign {
            target,
            value,
            type_annotation,
        })
    }

    fn convert_return(r: ast::StmtReturn) -> Result<HirStmt> {
        let value = r.value.map(|v| ExprConverter::convert(*v)).transpose()?;
        Ok(HirStmt::Return(value))
    }

    fn convert_if(i: ast::StmtIf) -> Result<HirStmt> {
        let condition = ExprConverter::convert(*i.test)?;
        let then_body = convert_body(i.body)?;
        let else_body = if i.orelse.is_empty() {
            None
        } else {
            Some(convert_body(i.orelse)?)
        };
        Ok(HirStmt::If {
            condition,
            then_body,
            else_body,
        })
    }

    fn convert_while(w: ast::StmtWhile) -> Result<HirStmt> {
        // Python: while chunk := f.read(8192):
        // Rust: loop { let chunk = f.read(8192); if chunk.is_empty() { break; } ... }
        if let ast::Expr::NamedExpr(named) = *w.test {
            // Extract variable name from target
            let var_name = if let ast::Expr::Name(n) = *named.target {
                n.id.to_string()
            } else {
                bail!("Walrus operator target must be a simple variable name");
            };

            // Convert the value expression
            let value_expr = ExprConverter::convert(*named.value)?;

            // Convert the body
            let body = convert_body(w.body)?;

            // Prepend the assignment and truthiness check to the body
            // loop { let var = expr; if !truthiness { break; } ...body... }
            let assign = HirStmt::Assign {
                target: AssignTarget::Symbol(var_name.clone()),
                value: value_expr.clone(),
                type_annotation: None,
            };

            // Create truthiness check: if chunk.is_empty() { break; }
            // For now, use simple .is_empty() check (works for Vec, String)
            let truthiness_check = HirStmt::If {
                condition: HirExpr::MethodCall {
                    object: Box::new(HirExpr::Var(var_name.clone())),
                    method: "is_empty".to_string(),
                    args: vec![],
                    kwargs: vec![],
                    type_params: vec![],
                },
                then_body: vec![HirStmt::Break { label: None }],
                else_body: None,
            };

            // Prepend assignment and check to body
            let mut loop_body = vec![assign, truthiness_check];
            loop_body.extend(body);

            // Convert to While(true) { ... } which is equivalent to loop { ... }
            return Ok(HirStmt::While {
                condition: HirExpr::Literal(crate::hir::Literal::Bool(true)),
                body: loop_body,
            });
        }

        let condition = ExprConverter::convert(*w.test)?;
        let body = convert_body(w.body)?;
        Ok(HirStmt::While { condition, body })
    }

    fn convert_for(f: ast::StmtFor) -> Result<HirStmt> {
        let target = extract_assign_target(&f.target)?;
        let iter = ExprConverter::convert(*f.iter)?;
        let body = convert_body(f.body)?;
        Ok(HirStmt::For { target, iter, body })
    }

    fn convert_expr_stmt(e: ast::StmtExpr) -> Result<HirStmt> {
        let expr = ExprConverter::convert(*e.value)?;
        Ok(HirStmt::Expr(expr))
    }

    fn convert_aug_assign(a: ast::StmtAugAssign) -> Result<HirStmt> {
        let target = extract_assign_target(&a.target)?;
        let op = convert_aug_op(&a.op)?;

        // Convert the target to an expression for the left side of the binary op
        let left = match &target {
            AssignTarget::Symbol(s) => Box::new(HirExpr::Var(s.clone())),
            AssignTarget::Attribute { value, attr } => Box::new(HirExpr::Attribute {
                value: value.clone(),
                attr: attr.clone(),
            }),
            AssignTarget::Index { base, index } => Box::new(HirExpr::Index {
                base: base.clone(),
                index: index.clone(),
            }),
            _ => bail!("Augmented assignment not supported for this target type"),
        };

        let right = Box::new(ExprConverter::convert(*a.value)?);
        let value = HirExpr::Binary { op, left, right };
        Ok(HirStmt::Assign {
            target,
            value,
            type_annotation: None,
        })
    }

    fn convert_raise(r: ast::StmtRaise) -> Result<HirStmt> {
        let exception = r.exc.map(|e| ExprConverter::convert(*e)).transpose()?;
        let cause = r.cause.map(|c| ExprConverter::convert(*c)).transpose()?;
        Ok(HirStmt::Raise { exception, cause })
    }

    fn convert_break(_b: ast::StmtBreak) -> Result<HirStmt> {
        // Python's AST doesn't support labeled break directly
        // Labels are handled at a higher level with loop naming
        Ok(HirStmt::Break { label: None })
    }

    fn convert_continue(_c: ast::StmtContinue) -> Result<HirStmt> {
        // Python's AST doesn't support labeled continue directly
        // Labels are handled at a higher level with loop naming
        Ok(HirStmt::Continue { label: None })
    }

    fn convert_with(w: ast::StmtWith) -> Result<HirStmt> {
        // Convert body first (shared by all context managers)
        let body = w
            .body
            .into_iter()
            .map(StmtConverter::convert)
            .collect::<Result<Vec<_>>>()?;

        // Build nested With statements from innermost to outermost
        // For `with a, b, c:` we create `with a { with b { with c { body } } }`
        let mut items = w.items;
        items.reverse(); // Process from last to first

        let mut result_body = body;
        for item in items {
            let context = ExprConverter::convert(item.context_expr)?;
            let target = item.optional_vars.and_then(|vars| match vars.as_ref() {
                ast::Expr::Name(n) => Some(n.id.to_string()),
                _ => None,
            });

            result_body = vec![HirStmt::With {
                context,
                target,
                body: result_body,
            }];
        }

        // Return the outermost With statement
        result_body
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Empty with statement"))
    }

    fn convert_try(t: ast::StmtTry) -> Result<HirStmt> {
        let body = convert_body(t.body)?;

        let mut handlers = Vec::new();
        for handler in t.handlers {
            // Extract the ExceptHandlerExceptHandler from the enum
            let ast::ExceptHandler::ExceptHandler(h) = handler;

            let exception_type = h.type_.as_ref().map(|t| {
                match t.as_ref() {
                    ast::Expr::Name(n) => n.id.to_string(),
                    _ => "Exception".to_string(), // Default to generic exception
                }
            });

            let name = h.name.as_ref().map(|id| id.to_string());
            let handler_body = convert_body(h.body)?;

            handlers.push(crate::hir::ExceptHandler {
                exception_type,
                name,
                body: handler_body,
            });
        }

        let orelse = if t.orelse.is_empty() {
            None
        } else {
            Some(convert_body(t.orelse)?)
        };

        let finalbody = if t.finalbody.is_empty() {
            None
        } else {
            Some(convert_body(t.finalbody)?)
        };

        Ok(HirStmt::Try {
            body,
            handlers,
            orelse,
            finalbody,
        })
    }

    fn convert_assert(a: ast::StmtAssert) -> Result<HirStmt> {
        let test = ExprConverter::convert(*a.test)?;
        let msg = a.msg.map(|m| ExprConverter::convert(*m)).transpose()?;
        Ok(HirStmt::Assert { test, msg })
    }

    fn convert_pass() -> Result<HirStmt> {
        Ok(HirStmt::Pass)
    }

    /// Convert nested function definition (inner functions)
    ///
    /// Converts nested function definitions to Rust inner functions
    fn convert_nested_function_def(func: ast::StmtFunctionDef) -> Result<HirStmt> {
        let name = func.name.to_string();
        let params = convert_nested_function_params(&func.args)?;
        let ret_type = super::type_extraction::TypeExtractor::extract_return_type(&func.returns)?;

        // Extract docstring and filter it from the body
        let (docstring, body) = extract_nested_function_body(func.body)?;

        Ok(HirStmt::FunctionDef {
            name,
            params: Box::new(params.into()),
            ret_type,
            body,
            docstring,
        })
    }

    /// Convert async nested function definition
    fn convert_async_function_def(func: ast::StmtAsyncFunctionDef) -> Result<HirStmt> {
        let name = func.name.to_string();
        let params = convert_nested_function_params(&func.args)?;
        let ret_type = super::type_extraction::TypeExtractor::extract_return_type(&func.returns)?;

        // Extract docstring and filter it from the body
        let (docstring, body) = extract_nested_function_body(func.body)?;

        Ok(HirStmt::AsyncFunctionDef {
            name,
            params: Box::new(params.into()),
            ret_type,
            body,
            docstring,
        })
    }

    fn convert_global(g: ast::StmtGlobal) -> Result<HirStmt> {
        let names = g.names.iter().map(|id| id.to_string()).collect();
        Ok(HirStmt::Global { names })
    }

    fn convert_nonlocal(n: ast::StmtNonlocal) -> Result<HirStmt> {
        let names = n.names.iter().map(|id| id.to_string()).collect();
        Ok(HirStmt::Nonlocal { names })
    }

    fn convert_import(i: ast::StmtImport) -> Result<HirStmt> {
        let modules = i
            .names
            .iter()
            .map(|alias| {
                let name = alias.name.to_string();
                let asname = alias.asname.as_ref().map(|n| n.to_string());
                (name, asname)
            })
            .collect();
        Ok(HirStmt::Import { modules })
    }

    fn convert_import_from(i: ast::StmtImportFrom) -> Result<HirStmt> {
        let module = i.module.as_ref().map(|m| m.to_string());
        let names = i
            .names
            .iter()
            .map(|alias| {
                let name = alias.name.to_string();
                let asname = alias.asname.as_ref().map(|n| n.to_string());
                (name, asname)
            })
            .collect();
        Ok(HirStmt::ImportFrom { module, names })
    }

    fn convert_async_for(af: ast::StmtAsyncFor) -> Result<HirStmt> {
        let target = extract_assign_target(&af.target)?;
        let iter = ExprConverter::convert(*af.iter)?;
        let body = convert_body(af.body)?;
        Ok(HirStmt::AsyncFor { target, iter, body })
    }

    fn convert_async_with(aw: ast::StmtAsyncWith) -> Result<HirStmt> {
        if aw.items.len() != 1 {
            bail!("Multiple async context managers not yet supported");
        }
        let item = &aw.items[0];
        let context = ExprConverter::convert(item.context_expr.clone())?;
        let target = item.optional_vars.as_ref().and_then(|v| {
            if let ast::Expr::Name(n) = v.as_ref() {
                Some(n.id.to_string())
            } else {
                None
            }
        });
        let body = convert_body(aw.body)?;
        Ok(HirStmt::AsyncWith {
            context,
            target,
            body,
        })
    }

    fn convert_delete(d: ast::StmtDelete) -> Result<HirStmt> {
        let targets = d
            .targets
            .iter()
            .map(extract_assign_target)
            .collect::<Result<Vec<_>>>()?;
        Ok(HirStmt::Delete { targets })
    }

    fn convert_match(m: ast::StmtMatch) -> Result<HirStmt> {
        let subject = ExprConverter::convert(*m.subject)?;
        let cases = m
            .cases
            .into_iter()
            .map(Self::convert_match_case)
            .collect::<Result<Vec<_>>>()?;
        Ok(HirStmt::Match { subject, cases })
    }

    fn convert_match_case(case: ast::MatchCase) -> Result<MatchCase> {
        let pattern = Self::convert_pattern(case.pattern)?;
        let guard = case.guard.map(|g| ExprConverter::convert(*g)).transpose()?;
        let body = convert_body(case.body)?;
        Ok(MatchCase {
            pattern,
            guard,
            body,
        })
    }

    fn convert_pattern(pattern: ast::Pattern) -> Result<HirPattern> {
        match pattern {
            ast::Pattern::MatchValue(v) => {
                let value = ExprConverter::convert(*v.value)?;
                Ok(HirPattern::Value(value))
            }
            ast::Pattern::MatchSingleton(s) => {
                let lit = match &s.value {
                    ast::Constant::None => Literal::None,
                    ast::Constant::Bool(b) => Literal::Bool(*b),
                    ast::Constant::Ellipsis => Literal::Ellipsis,
                    _ => bail!("Unsupported singleton constant in match pattern"),
                };
                Ok(HirPattern::Singleton(lit))
            }
            ast::Pattern::MatchSequence(seq) => {
                let patterns = seq
                    .patterns
                    .into_iter()
                    .map(Self::convert_pattern)
                    .collect::<Result<Vec<_>>>()?;
                Ok(HirPattern::Sequence(patterns))
            }
            ast::Pattern::MatchMapping(mapping) => {
                let keys = mapping
                    .keys
                    .into_iter()
                    .map(super::convert_expr)
                    .collect::<Result<Vec<_>>>()?;
                let patterns = mapping
                    .patterns
                    .into_iter()
                    .map(Self::convert_pattern)
                    .collect::<Result<Vec<_>>>()?;
                let rest = mapping.rest.map(|id| id.to_string());
                Ok(HirPattern::Mapping {
                    keys,
                    patterns,
                    rest,
                })
            }
            ast::Pattern::MatchClass(cls) => {
                let cls_name = match *cls.cls {
                    ast::Expr::Name(n) => n.id.to_string(),
                    ast::Expr::Attribute(attr) => {
                        let value = ExprConverter::convert(*attr.value)?;
                        format!("{:?}.{}", value, attr.attr)
                    }
                    _ => bail!("Unsupported class expression in match pattern"),
                };
                let patterns = cls
                    .patterns
                    .into_iter()
                    .map(Self::convert_pattern)
                    .collect::<Result<Vec<_>>>()?;
                let kwd_attrs = cls.kwd_attrs.iter().map(|id| id.to_string()).collect();
                let kwd_patterns = cls
                    .kwd_patterns
                    .into_iter()
                    .map(Self::convert_pattern)
                    .collect::<Result<Vec<_>>>()?;
                Ok(HirPattern::Class {
                    cls: cls_name,
                    patterns,
                    kwd_attrs,
                    kwd_patterns,
                })
            }
            ast::Pattern::MatchStar(star) => {
                let name = star.name.map(|id| id.to_string());
                Ok(HirPattern::Star(name))
            }
            ast::Pattern::MatchAs(as_pat) => {
                // Wildcard is represented as MatchAs with no pattern and no name
                if as_pat.pattern.is_none() && as_pat.name.is_none() {
                    return Ok(HirPattern::Wildcard);
                }
                let pattern = as_pat
                    .pattern
                    .map(|p| Self::convert_pattern(*p))
                    .transpose()?
                    .map(Box::new);
                let name = as_pat.name.map(|id| id.to_string());
                Ok(HirPattern::As { pattern, name })
            }
            ast::Pattern::MatchOr(or_pat) => {
                let patterns = or_pat
                    .patterns
                    .into_iter()
                    .map(Self::convert_pattern)
                    .collect::<Result<Vec<_>>>()?;
                Ok(HirPattern::Or(patterns))
            }
        }
    }
}

/// Expression converter to reduce complexity
pub struct ExprConverter;

impl ExprConverter {
    pub fn convert(expr: ast::Expr) -> Result<HirExpr> {
        match expr {
            ast::Expr::Constant(c) => Self::convert_constant(c),
            ast::Expr::Name(n) => Self::convert_name(n),
            ast::Expr::BinOp(b) => Self::convert_binop_expr(b),
            ast::Expr::UnaryOp(u) => Self::convert_unaryop_expr(u),
            ast::Expr::BoolOp(b) => Self::convert_boolop(b),
            ast::Expr::Call(c) => Self::convert_call(c),
            ast::Expr::Subscript(s) => Self::convert_subscript(s),
            ast::Expr::List(l) => Self::convert_list(l),
            ast::Expr::Dict(d) => Self::convert_dict(d),
            ast::Expr::Tuple(t) => Self::convert_tuple(t),
            ast::Expr::Compare(c) => Self::convert_compare(c),
            ast::Expr::ListComp(lc) => Self::convert_list_comp(lc),
            ast::Expr::SetComp(sc) => Self::convert_set_comp(sc),
            ast::Expr::DictComp(dc) => Self::convert_dict_comp(dc),
            ast::Expr::GeneratorExp(ge) => Self::convert_generator_exp(ge),
            ast::Expr::Lambda(l) => Self::convert_lambda(l),
            ast::Expr::Set(s) => Self::convert_set(s),
            ast::Expr::Attribute(a) => Self::convert_attribute(a),
            ast::Expr::Await(a) => Self::convert_await(a),
            ast::Expr::Yield(y) => Self::convert_yield(y),
            ast::Expr::JoinedStr(js) => Self::convert_fstring(js),
            ast::Expr::IfExp(i) => Self::convert_ifexp(i),
            ast::Expr::NamedExpr(ne) => Self::convert_named_expr(ne),
            // When used as a regular argument, just unwrap and pass the inner expression
            ast::Expr::Starred(s) => Self::convert(*s.value),
            _ => bail!("Expression type not yet supported"),
        }
    }

    /// Convert an expression with source span tracking
    pub fn convert_with_span(expr: ast::Expr, ctx: &SpanContext) -> Result<SpannedExpr> {
        let span = ctx.span_from(&expr);
        let inner = Self::convert(expr)?;
        Ok(Spanned { inner, span })
    }

    fn convert_constant(c: ast::ExprConstant) -> Result<HirExpr> {
        let lit = match &c.value {
            ast::Constant::Int(i) => {
                // Convert BigInt to i64, with overflow handling
                let int_val = i.try_into().unwrap_or(0i64);
                Literal::Int(int_val)
            }
            ast::Constant::Float(f) => Literal::Float(*f),
            ast::Constant::Str(s) => Literal::String(s.to_string()),
            ast::Constant::Bytes(b) => Literal::Bytes(b.clone()),
            ast::Constant::Bool(b) => Literal::Bool(*b),
            ast::Constant::None => Literal::None,
            ast::Constant::Ellipsis => Literal::Ellipsis,
            ast::Constant::Complex { real, imag } => Literal::Complex(*real, *imag),
            _ => bail!("Unsupported constant type"),
        };
        Ok(HirExpr::Literal(lit))
    }

    fn convert_name(n: ast::ExprName) -> Result<HirExpr> {
        Ok(HirExpr::Var(n.id.to_string()))
    }

    fn convert_binop_expr(b: ast::ExprBinOp) -> Result<HirExpr> {
        let op = convert_binop(&b.op)?;
        let left = Box::new(Self::convert(*b.left)?);
        let right = Box::new(Self::convert(*b.right)?);
        Ok(HirExpr::Binary { op, left, right })
    }

    fn convert_unaryop_expr(u: ast::ExprUnaryOp) -> Result<HirExpr> {
        let op = convert_unaryop(&u.op)?;
        let operand = Box::new(Self::convert(*u.operand)?);
        Ok(HirExpr::Unary { op, operand })
    }

    fn convert_call(c: ast::ExprCall) -> Result<HirExpr> {
        // Special handling for sorted() with key parameter
        if let ast::Expr::Name(n) = &*c.func {
            if n.id.as_str() == "sorted" && !c.keywords.is_empty() {
                // Extract key and reverse parameters
                let mut key_lambda = None;
                let mut reverse = false;

                for keyword in &c.keywords {
                    if let Some(arg_name) = &keyword.arg {
                        match arg_name.as_str() {
                            "key" => {
                                if let ast::Expr::Lambda(lambda) = &keyword.value {
                                    key_lambda = Some(lambda.clone());
                                } else {
                                    bail!("sorted() key parameter must be a lambda");
                                }
                            }
                            "reverse" => {
                                // Extract boolean value from reverse parameter
                                if let ast::Expr::Constant(c) = &keyword.value {
                                    if let ast::Constant::Bool(b) = &c.value {
                                        reverse = *b;
                                    } else {
                                        bail!("sorted() reverse parameter must be a boolean");
                                    }
                                } else {
                                    bail!("sorted() reverse parameter must be a constant boolean");
                                }
                            }
                            _ => {} // Ignore other parameters
                        }
                    }
                }

                // If we found a key lambda, create SortByKey
                if let Some(lambda) = key_lambda {
                    // Convert the iterable (first positional arg)
                    if c.args.is_empty() {
                        bail!("sorted() requires at least one argument");
                    }
                    let iterable = Box::new(Self::convert(c.args[0].clone())?);

                    // Extract lambda parameters and body
                    let key_params: Vec<String> = lambda
                        .args
                        .args
                        .iter()
                        .map(|arg| arg.def.arg.to_string())
                        .collect();

                    let key_body = Box::new(Self::convert(*lambda.body.clone())?);

                    return Ok(HirExpr::SortByKey {
                        iterable,
                        key_params,
                        key_body,
                        reverse,
                    });
                }

                // This ensures the reverse parameter is preserved in the HIR
                if reverse {
                    if c.args.is_empty() {
                        bail!("sorted() requires at least one argument");
                    }
                    let iterable = Box::new(Self::convert(c.args[0].clone())?);

                    // Use identity function: lambda x: x
                    let key_params = vec!["x".to_string()];
                    let key_body = Box::new(HirExpr::Var("x".to_string()));

                    return Ok(HirExpr::SortByKey {
                        iterable,
                        key_params,
                        key_body,
                        reverse,
                    });
                }
            }
        }

        // Check if any args use the Starred expression (unpacking operator)
        let has_starred = c
            .args
            .iter()
            .any(|arg| matches!(arg, ast::Expr::Starred(_)));

        if has_starred {
            // Special handling for os.path.join(*parts)
            if let ast::Expr::Attribute(attr) = &*c.func {
                // Check if this is os.path.join
                if let ast::Expr::Attribute(inner_attr) = &*attr.value {
                    if let ast::Expr::Name(module_name) = &*inner_attr.value {
                        if module_name.id.as_str() == "os"
                            && inner_attr.attr.as_str() == "path"
                            && attr.attr.as_str() == "join"
                        {
                            // Extract the starred argument
                            if let Some(ast::Expr::Starred(starred)) = c
                                .args
                                .iter()
                                .find(|arg| matches!(arg, ast::Expr::Starred(_)))
                            {
                                let parts_expr = Self::convert(*starred.value.clone())?;

                                // Create a method call: parts.join(MAIN_SEPARATOR_STR)
                                // We'll represent this as a special Call that the Rust generator knows how to handle
                                return Ok(HirExpr::Call {
                                    func: "__os_path_join_starred".to_string(),
                                    args: vec![parts_expr],
                                    kwargs: vec![],
                                    type_params: vec![],
                                });
                            }
                        }
                    }
                }
            }

            // Special handling for print(*items)
            if let ast::Expr::Name(name) = &*c.func {
                if name.id.as_str() == "print" {
                    // Extract the starred argument
                    if let Some(ast::Expr::Starred(starred)) = c
                        .args
                        .iter()
                        .find(|arg| matches!(arg, ast::Expr::Starred(_)))
                    {
                        let items_expr = Self::convert(*starred.value.clone())?;

                        // Create a special Call that the Rust generator knows how to handle
                        return Ok(HirExpr::Call {
                            func: "__print_starred".to_string(),
                            args: vec![items_expr],
                            kwargs: vec![],
                            type_params: vec![],
                        });
                    }
                }
            }

            // General case: For user-defined functions with *args, just pass the argument directly
            // Python: func(*items) where func is user-defined
            // Rust: func(items) where func accepts &[T] or Vec<T>
            // This allows forwarding variadic arguments without special handling
            // We don't need to bail - just convert the starred args to regular args by unwrapping them
        }

        let args = c
            .args
            .into_iter()
            .map(Self::convert)
            .collect::<Result<Vec<_>>>()?;

        let kwargs: Vec<(String, HirExpr)> = c
            .keywords
            .into_iter()
            .filter_map(|kw| {
                // Only process keywords with explicit names (not **kwargs unpacking)
                if let Some(arg_name) = kw.arg {
                    let value = Self::convert(kw.value).ok()?;
                    Some((arg_name.to_string(), value))
                } else {
                    None // Skip **kwargs unpacking for now
                }
            })
            .collect();

        match &*c.func {
            ast::Expr::Name(n) => {
                // Simple function call
                let func = n.id.to_string();
                Ok(HirExpr::Call {
                    func,
                    args,
                    kwargs,
                    type_params: vec![],
                })
            }
            ast::Expr::Attribute(attr) => {
                // Method call
                let object = Box::new(Self::convert(*attr.value.clone())?);
                let method = attr.attr.to_string();
                Ok(HirExpr::MethodCall {
                    object,
                    method,
                    args,
                    kwargs,
                    type_params: vec![],
                })
            }
            ast::Expr::Subscript(subscript) => {
                // Check if this is a dictionary/list access being called (not a generic call)
                // Dictionary access with string key: data_processors["key"](args)
                // List access with integer: funcs[0](args)
                if matches!(
                    &*subscript.slice,
                    ast::Expr::Constant(c) if matches!(c.value, ast::Constant::Str(_) | ast::Constant::Int(_))
                ) {
                    // This is dict/list indexing followed by call: obj[key](args)
                    // Convert the subscript expression first, then call it
                    let subscript_expr = Self::convert_subscript(subscript.clone())?;
                    Ok(HirExpr::MethodCall {
                        object: Box::new(subscript_expr),
                        method: "__call__".to_string(),
                        args,
                        kwargs,
                        type_params: vec![],
                    })
                } else {
                    // Generic function/method call: func[Type](args) or obj.method[Type](args)
                    let type_params = Self::extract_type_params_from_slice(&subscript.slice)?;

                    match &*subscript.value {
                        ast::Expr::Name(n) => {
                            // Generic function call: func[Type](args)
                            let func = n.id.to_string();
                            Ok(HirExpr::Call {
                                func,
                                args,
                                kwargs,
                                type_params,
                            })
                        }
                        ast::Expr::Attribute(attr) => {
                            // Generic method call: obj.method[Type](args)
                            let object = Box::new(Self::convert(*attr.value.clone())?);
                            let method = attr.attr.to_string();
                            Ok(HirExpr::MethodCall {
                                object,
                                method,
                                args,
                                kwargs,
                                type_params,
                            })
                        }
                        _ => bail!("Unsupported generic call base: {:?}", subscript.value),
                    }
                }
            }
            ast::Expr::Call(inner_call) => {
                // Chained function call: outer()() or func(a)(b)
                // Convert the inner call first, then wrap it as the object being called
                let inner_expr = Self::convert_call(inner_call.clone())?;
                // Create a method call to simulate calling the result
                // We'll use a special "call" method that codegen can handle
                Ok(HirExpr::MethodCall {
                    object: Box::new(inner_expr),
                    method: "__call__".to_string(),
                    args,
                    kwargs,
                    type_params: vec![],
                })
            }
            _ => bail!("Unsupported function call type: {:?}", c.func),
        }
    }

    fn convert_subscript(s: ast::ExprSubscript) -> Result<HirExpr> {
        let base = Box::new(Self::convert(*s.value)?);

        // Check if the slice is actually a slice expression or a simple index
        match s.slice.as_ref() {
            ast::Expr::Slice(slice_expr) => {
                // Convert slice expression
                let start = slice_expr
                    .lower
                    .as_ref()
                    .map(|e| Self::convert(e.as_ref().clone()))
                    .transpose()?
                    .map(Box::new);
                let stop = slice_expr
                    .upper
                    .as_ref()
                    .map(|e| Self::convert(e.as_ref().clone()))
                    .transpose()?
                    .map(Box::new);
                let step = slice_expr
                    .step
                    .as_ref()
                    .map(|e| Self::convert(e.as_ref().clone()))
                    .transpose()?
                    .map(Box::new);

                Ok(HirExpr::Slice {
                    base,
                    start,
                    stop,
                    step,
                })
            }
            _ => {
                // Regular indexing
                let index = Box::new(Self::convert(*s.slice)?);
                Ok(HirExpr::Index { base, index })
            }
        }
    }

    fn convert_list(l: ast::ExprList) -> Result<HirExpr> {
        let elts = l
            .elts
            .into_iter()
            .map(Self::convert)
            .collect::<Result<Vec<_>>>()?;
        Ok(HirExpr::List(elts))
    }

    fn convert_dict(d: ast::ExprDict) -> Result<HirExpr> {
        let mut items = Vec::new();
        for (k, v) in d.keys.into_iter().zip(d.values.into_iter()) {
            if let Some(key) = k {
                let key_expr = Self::convert(key)?;
                let val_expr = Self::convert(v)?;
                items.push((key_expr, val_expr));
            } else {
                bail!("Dict unpacking not supported");
            }
        }
        Ok(HirExpr::Dict(items))
    }

    fn convert_tuple(t: ast::ExprTuple) -> Result<HirExpr> {
        let elts = t
            .elts
            .into_iter()
            .map(Self::convert)
            .collect::<Result<Vec<_>>>()?;
        Ok(HirExpr::Tuple(elts))
    }

    fn convert_boolop(b: ast::ExprBoolOp) -> Result<HirExpr> {
        // Convert boolean operations (and, or) to binary operations
        if b.values.len() < 2 {
            bail!("BoolOp must have at least 2 values");
        }

        // Convert the operator
        let op = match b.op {
            ast::BoolOp::And => BinOp::And,
            ast::BoolOp::Or => BinOp::Or,
        };

        // Convert values and chain them left-to-right
        let mut result = Self::convert(b.values[0].clone())?;
        for value in b.values.iter().skip(1) {
            let right = Self::convert(value.clone())?;
            result = HirExpr::Binary {
                op,
                left: Box::new(result),
                right: Box::new(right),
            };
        }

        Ok(result)
    }

    fn convert_compare(c: ast::ExprCompare) -> Result<HirExpr> {
        // Handle chained comparisons by desugaring them
        // Example: 0 <= x <= 100 becomes (0 <= x) and (x <= 100)

        if c.ops.is_empty() || c.comparators.is_empty() {
            bail!("Compare expression must have at least one operator and comparator");
        }

        // Special handling for 'is None', 'is True', 'is False' patterns (single comparison only)
        if c.ops.len() == 1
            && c.comparators.len() == 1
            && matches!(c.ops[0], ast::CmpOp::Is | ast::CmpOp::IsNot)
        {
            let comparator = &c.comparators[0];
            // Check if comparing with None
            let is_none_comparison = matches!(comparator, ast::Expr::Constant(cons)
                    if matches!(cons.value, ast::Constant::None));

            if is_none_comparison {
                // Convert 'x is None' to x.is_none(), 'x is not None' to x.is_some()
                let object = Box::new(Self::convert(*c.left)?);
                let method = if matches!(c.ops[0], ast::CmpOp::Is) {
                    "is_none".to_string()
                } else {
                    "is_some".to_string()
                };
                return Ok(HirExpr::MethodCall {
                    object,
                    method,
                    args: vec![],
                    kwargs: vec![],
                    type_params: vec![],
                });
            }

            // Check if comparing with True or False
            let is_bool_comparison = matches!(comparator, ast::Expr::Constant(cons)
                    if matches!(cons.value, ast::Constant::Bool(_)));

            if is_bool_comparison {
                // Convert 'x is True' to x == true, 'x is False' to x == false
                // Convert 'x is not True' to x != true, 'x is not False' to x != false
                let left_hir = Box::new(Self::convert(*c.left)?);
                let right_hir = Box::new(Self::convert(comparator.clone())?);
                let op = if matches!(c.ops[0], ast::CmpOp::Is) {
                    BinOp::Eq
                } else {
                    BinOp::NotEq
                };
                return Ok(HirExpr::Binary {
                    op,
                    left: left_hir,
                    right: right_hir,
                });
            }
        }

        // Build chain: a op1 b op2 c becomes (a op1 b) and (b op2 c)
        let mut left_expr = *c.left;
        let mut comparisons = Vec::new();

        for (op, comparator) in c.ops.iter().zip(c.comparators.iter()) {
            let op_hir = convert_cmpop(op)?;
            let left_hir = Box::new(Self::convert(left_expr.clone())?);
            let right_hir = Box::new(Self::convert(comparator.clone())?);

            comparisons.push(HirExpr::Binary {
                op: op_hir,
                left: left_hir,
                right: right_hir,
            });

            // For next iteration, the right side becomes the left side
            left_expr = comparator.clone();
        }

        // If only one comparison, return it directly
        if comparisons.len() == 1 {
            return Ok(comparisons.into_iter().next().unwrap());
        }

        // Chain multiple comparisons with AND
        let mut result = comparisons[0].clone();
        for comparison in comparisons.iter().skip(1) {
            result = HirExpr::Binary {
                op: BinOp::And,
                left: Box::new(result),
                right: Box::new(comparison.clone()),
            };
        }

        Ok(result)
    }

    fn convert_list_comp(lc: ast::ExprListComp) -> Result<HirExpr> {
        // Handle nested comprehensions with multiple generators (flat_map pattern)
        if lc.generators.len() > 1 {
            let element = Box::new(Self::convert(*lc.elt)?);
            let generators = lc
                .generators
                .into_iter()
                .map(|generator| {
                    let target = Self::extract_comprehension_target(&generator.target)?;
                    let iter = Box::new(Self::convert(generator.iter)?);
                    let conditions = generator
                        .ifs
                        .into_iter()
                        .map(|cond| Self::convert(cond))
                        .collect::<Result<Vec<_>>>()?;
                    Ok(HirComprehension {
                        target,
                        iter,
                        conditions,
                    })
                })
                .collect::<Result<Vec<_>>>()?;

            return Ok(HirExpr::FlattenedListComp {
                element,
                generators,
            });
        }

        let generator = &lc.generators[0];

        // Extract the target variable or tuple pattern
        let target = Self::extract_comprehension_target(&generator.target)?;

        // Convert the iterator expression
        let iter = Box::new(Self::convert(generator.iter.clone())?);

        // Convert the element expression
        let element = Box::new(Self::convert(*lc.elt)?);

        // Convert the condition if present
        // Multiple if conditions are ANDed together: [x for x in items if x > 0 if x < 10]
        // becomes: [x for x in items if (x > 0) && (x < 10)]
        let condition = if generator.ifs.is_empty() {
            None
        } else if generator.ifs.len() == 1 {
            Some(Box::new(Self::convert(generator.ifs[0].clone())?))
        } else {
            // Combine multiple conditions with And
            let mut combined = Self::convert(generator.ifs[0].clone())?;
            for cond in &generator.ifs[1..] {
                let right = Self::convert(cond.clone())?;
                combined = HirExpr::Binary {
                    op: crate::hir::BinOp::And,
                    left: Box::new(combined),
                    right: Box::new(right),
                };
            }
            Some(Box::new(combined))
        };

        Ok(HirExpr::ListComp {
            element,
            target,
            iter,
            condition,
        })
    }

    fn convert_set_comp(sc: ast::ExprSetComp) -> Result<HirExpr> {
        // Convert only simple set comprehensions for now
        if sc.generators.len() != 1 {
            bail!("Nested set comprehensions not yet supported");
        }

        let generator = &sc.generators[0];

        // Extract the target variable or tuple pattern
        let target = Self::extract_comprehension_target(&generator.target)?;

        // Convert the iterator expression
        let iter = Box::new(Self::convert(generator.iter.clone())?);

        // Convert the element expression
        let element = Box::new(Self::convert(*sc.elt)?);

        // Convert the condition if present
        // Multiple if conditions are ANDed together
        let condition = if generator.ifs.is_empty() {
            None
        } else if generator.ifs.len() == 1 {
            Some(Box::new(Self::convert(generator.ifs[0].clone())?))
        } else {
            // Combine multiple conditions with And
            let mut combined = Self::convert(generator.ifs[0].clone())?;
            for cond in &generator.ifs[1..] {
                let right = Self::convert(cond.clone())?;
                combined = HirExpr::Binary {
                    op: crate::hir::BinOp::And,
                    left: Box::new(combined),
                    right: Box::new(right),
                };
            }
            Some(Box::new(combined))
        };

        Ok(HirExpr::SetComp {
            element,
            target,
            iter,
            condition,
        })
    }

    fn convert_dict_comp(dc: ast::ExprDictComp) -> Result<HirExpr> {
        // Convert only simple dict comprehensions for now
        if dc.generators.len() != 1 {
            bail!("Nested dict comprehensions not yet supported");
        }

        let generator = &dc.generators[0];

        // Extract the target variable or tuple pattern
        let target = Self::extract_comprehension_target(&generator.target)?;

        // Convert the iterator expression
        let iter = Box::new(Self::convert(generator.iter.clone())?);

        // Convert the key and value expressions
        let key = Box::new(Self::convert(*dc.key)?);
        let value = Box::new(Self::convert(*dc.value)?);

        // Convert the condition if present
        // Multiple if conditions are ANDed together
        let condition = if generator.ifs.is_empty() {
            None
        } else if generator.ifs.len() == 1 {
            Some(Box::new(Self::convert(generator.ifs[0].clone())?))
        } else {
            // Combine multiple conditions with And
            let mut combined = Self::convert(generator.ifs[0].clone())?;
            for cond in &generator.ifs[1..] {
                let right = Self::convert(cond.clone())?;
                combined = HirExpr::Binary {
                    op: crate::hir::BinOp::And,
                    left: Box::new(combined),
                    right: Box::new(right),
                };
            }
            Some(Box::new(combined))
        };

        Ok(HirExpr::DictComp {
            key,
            value,
            target,
            iter,
            condition,
        })
    }

    fn convert_generator_exp(ge: ast::ExprGeneratorExp) -> Result<HirExpr> {
        // Convert element expression
        let element = Box::new(Self::convert(*ge.elt)?);

        // Convert all generators (support nested)
        let mut generators = Vec::new();
        for generator in ge.generators {
            // Extract target variable(s)
            let target = match &generator.target {
                ast::Expr::Name(n) => n.id.to_string(),
                ast::Expr::Tuple(t) => {
                    // For tuple unpacking like: (x, y) in zip(a, b)
                    // Extract all names and join with commas (simplified for now)
                    let names: Vec<String> = t
                        .elts
                        .iter()
                        .filter_map(|e| {
                            if let ast::Expr::Name(n) = e {
                                Some(n.id.to_string())
                            } else {
                                None
                            }
                        })
                        .collect();
                    if names.is_empty() {
                        bail!("Complex tuple unpacking in generator expression not yet supported");
                    }
                    // Join with comma for tuple targets
                    format!("({})", names.join(", "))
                }
                _ => bail!("Complex generator targets not yet supported"),
            };

            // Convert iterator expression
            let iter = Box::new(Self::convert(generator.iter.clone())?);

            // Convert all conditions
            let conditions: Vec<crate::hir::HirExpr> = generator
                .ifs
                .iter()
                .map(|if_expr| Self::convert(if_expr.clone()))
                .collect::<Result<Vec<_>>>()?;

            generators.push(crate::hir::HirComprehension {
                target,
                iter,
                conditions,
            });
        }

        Ok(HirExpr::GeneratorExp {
            element,
            generators,
        })
    }

    fn convert_lambda(l: ast::ExprLambda) -> Result<HirExpr> {
        // Extract parameter names
        let params: Vec<String> = l
            .args
            .args
            .iter()
            .map(|arg| arg.def.arg.to_string())
            .collect();

        // Convert body expression
        let body = Box::new(ExprConverter::convert(*l.body)?);

        Ok(HirExpr::Lambda { params, body })
    }

    fn convert_set(s: ast::ExprSet) -> Result<HirExpr> {
        let elems = s
            .elts
            .into_iter()
            .map(super::convert_expr)
            .collect::<Result<Vec<_>>>()?;
        Ok(HirExpr::Set(elems))
    }

    fn convert_attribute(a: ast::ExprAttribute) -> Result<HirExpr> {
        let value = Box::new(Self::convert(*a.value)?);
        let attr = a.attr.to_string();
        Ok(HirExpr::Attribute { value, attr })
    }

    fn convert_await(a: ast::ExprAwait) -> Result<HirExpr> {
        let value = Box::new(Self::convert(*a.value)?);
        Ok(HirExpr::Await { value })
    }

    fn convert_yield(y: ast::ExprYield) -> Result<HirExpr> {
        let value = y
            .value
            .map(|v| Self::convert(*v))
            .transpose()?
            .map(Box::new);
        Ok(HirExpr::Yield { value })
    }

    fn convert_fstring(js: ast::ExprJoinedStr) -> Result<HirExpr> {
        let mut parts = Vec::new();

        for value in js.values {
            match value {
                // Literal string parts
                ast::Expr::Constant(c) => {
                    if let ast::Constant::Str(s) = c.value {
                        parts.push(FStringPart::Literal(s.to_string()));
                    }
                }
                // Formatted values (expressions to interpolate)
                ast::Expr::FormattedValue(fv) => {
                    let expr = Self::convert(*fv.value)?;
                    parts.push(FStringPart::Expr(Box::new(expr)));
                }
                _ => {
                    // Other expression types in f-strings (rare)
                    let expr = Self::convert(value)?;
                    parts.push(FStringPart::Expr(Box::new(expr)));
                }
            }
        }

        Ok(HirExpr::FString { parts })
    }

    fn convert_ifexp(i: ast::ExprIfExp) -> Result<HirExpr> {
        let test = Box::new(Self::convert(*i.test)?);
        let body = Box::new(Self::convert(*i.body)?);
        let orelse = Box::new(Self::convert(*i.orelse)?);
        Ok(HirExpr::IfExpr { test, body, orelse })
    }

    fn convert_named_expr(ne: ast::ExprNamedExpr) -> Result<HirExpr> {
        let target = if let ast::Expr::Name(n) = *ne.target {
            n.id.to_string()
        } else {
            bail!("Walrus operator target must be a simple variable name");
        };
        let value = Box::new(Self::convert(*ne.value)?);
        Ok(HirExpr::NamedExpr { target, value })
    }

    /// Extract type parameters from a subscript slice for generic calls.
    /// Handles both single type params `func[int]` and multiple `func[int, str]`.
    fn extract_type_params_from_slice(slice: &ast::Expr) -> Result<Vec<Type>> {
        match slice {
            ast::Expr::Name(n) => {
                // Single type parameter: func[int]
                let ty = super::type_extraction::TypeExtractor::extract_simple_type(&n.id)?;
                Ok(vec![ty])
            }
            ast::Expr::Tuple(t) => {
                // Multiple type parameters: func[int, str]
                t.elts
                    .iter()
                    .map(|e| match e {
                        ast::Expr::Name(n) => {
                            super::type_extraction::TypeExtractor::extract_simple_type(&n.id)
                        }
                        ast::Expr::Subscript(_) => {
                            // Nested generic type: func[List[int]]
                            super::type_extraction::TypeExtractor::extract_type(e)
                        }
                        _ => bail!("Unsupported type parameter expression: {:?}", e),
                    })
                    .collect()
            }
            ast::Expr::Subscript(_) => {
                // Nested generic: func[List[int]]
                let ty = super::type_extraction::TypeExtractor::extract_type(slice)?;
                Ok(vec![ty])
            }
            _ => bail!("Unsupported type parameter slice: {:?}", slice),
        }
    }

    /// Extract a comprehension target pattern (simple name or tuple unpacking)
    fn extract_comprehension_target(target: &ast::Expr) -> Result<String> {
        match target {
            ast::Expr::Name(n) => Ok(n.id.to_string()),
            ast::Expr::Tuple(t) => {
                // For tuple unpacking like: (k, v) in dict.items()
                let names: Vec<String> = t
                    .elts
                    .iter()
                    .map(|e| Self::extract_comprehension_target(e))
                    .collect::<Result<Vec<_>>>()?;
                if names.is_empty() {
                    bail!("Empty tuple pattern in comprehension not supported");
                }
                Ok(format!("({})", names.join(", ")))
            }
            _ => bail!("Complex comprehension targets not yet supported"),
        }
    }
}

// ============================================================================
// ============================================================================

/// Convert parameters for nested functions
fn convert_nested_function_params(args: &ast::Arguments) -> Result<Vec<HirParam>> {
    let mut params = Vec::new();

    // Calculate number of args without defaults
    let num_args = args.args.len();
    let defaults_vec: Vec<_> = args.defaults().collect();
    let num_defaults = defaults_vec.len();
    let first_default_idx = num_args.saturating_sub(num_defaults);

    for (i, arg) in args.args.iter().enumerate() {
        let name = arg.def.arg.to_string();
        let ty = if let Some(annotation) = &arg.def.annotation {
            super::type_extraction::TypeExtractor::extract_type(annotation)?
        } else {
            Type::Unknown
        };

        // Check if this parameter has a default value
        let default = if i >= first_default_idx {
            let default_idx = i - first_default_idx;
            if let Some(default_expr) = defaults_vec.get(default_idx) {
                Some(ExprConverter::convert((*default_expr).clone())?)
            } else {
                None
            }
        } else {
            None
        };

        params.push(HirParam { name, ty, default });
    }

    Ok(params)
}

/// Extract docstring and convert body for nested functions
fn extract_nested_function_body(body: Vec<ast::Stmt>) -> Result<(Option<String>, Vec<HirStmt>)> {
    if body.is_empty() {
        return Ok((None, vec![]));
    }

    // Check if first statement is a docstring
    let (docstring, remaining_body) = if let ast::Stmt::Expr(expr_stmt) = &body[0] {
        if let ast::Expr::Constant(c) = &*expr_stmt.value {
            if let ast::Constant::Str(s) = &c.value {
                (Some(s.to_string()), &body[1..])
            } else {
                (None, &body[..])
            }
        } else {
            (None, &body[..])
        }
    } else {
        (None, &body[..])
    };

    // Convert remaining statements
    let hir_body = convert_body(remaining_body.to_vec())?;

    Ok((docstring, hir_body))
}

// ---------------------------------------------------------------------------
// AstBridge conversion methods — moved here from ast_bridge.rs to separate
// high-level bridge setup from the bulk of the AST-to-HIR conversion logic.
// ---------------------------------------------------------------------------

use super::{AstBridge, FunctionAnalyzer, TypeExtractor};
use crate::annotations::TranspilationAnnotations;
use crate::types::type_hints::TypeHintProvider;

impl AstBridge {
    pub(super) fn convert_function(
        &self,
        func: ast::StmtFunctionDef,
        is_async: bool,
    ) -> Result<HirFunction> {
        let name = func.name.to_string();
        let params = convert_parameters(&func.args)?;
        let ret_type = TypeExtractor::extract_return_type(&func.returns)?;

        // Extract annotations from source code if available
        let annotations = self.extract_function_annotations(&func);

        // Extract docstring and filter it from the body
        let (docstring, filtered_body) = extract_docstring_and_body(func.body)?;
        let mut properties = FunctionAnalyzer::analyze(&filtered_body);
        properties.is_async = is_async;

        let mut hir_function = HirFunction {
            name,
            params: params.into(),
            ret_type,
            body: filtered_body,
            properties,
            annotations,
            docstring,
        };

        // Infer parameter types if any parameters have Type::Unknown
        self.infer_function_parameter_types(&mut hir_function);

        Ok(hir_function)
    }

    pub(super) fn convert_async_function(
        &self,
        func: ast::StmtAsyncFunctionDef,
    ) -> Result<HirFunction> {
        let name = func.name.to_string();
        let params = convert_parameters(&func.args)?;
        let ret_type = TypeExtractor::extract_return_type(&func.returns)?;

        // Extract annotations from source code if available
        let annotations = self.extract_async_function_annotations(&func);

        // Extract docstring and filter it from the body
        let (docstring, filtered_body) = extract_docstring_and_body(func.body)?;
        let mut properties = FunctionAnalyzer::analyze(&filtered_body);
        properties.is_async = true;

        let mut hir_function = HirFunction {
            name,
            params: params.into(),
            ret_type,
            body: filtered_body,
            properties,
            annotations,
            docstring,
        };

        // Infer parameter types if any parameters have Type::Unknown
        self.infer_function_parameter_types(&mut hir_function);

        Ok(hir_function)
    }

    pub(super) fn extract_function_annotations(
        &self,
        func: &ast::StmtFunctionDef,
    ) -> TranspilationAnnotations {
        // Try to extract from source code comments first
        if let Some(source) = &self.source_code {
            if let Some(annotation_text) = self
                .annotation_extractor
                .extract_function_annotations(source, &func.name)
            {
                if let Ok(annotations) = self.annotation_parser.parse_annotations(&annotation_text)
                {
                    return annotations;
                }
            }
        }

        // Fallback: Try to extract from docstring if present
        if let Some(ast::Stmt::Expr(expr)) = func.body.first() {
            if let ast::Expr::Constant(constant) = expr.value.as_ref() {
                if let ast::Constant::Str(docstring) = &constant.value {
                    if let Ok(annotations) = self.annotation_parser.parse_annotations(docstring) {
                        return annotations;
                    }
                }
            }
        }

        TranspilationAnnotations::default()
    }

    pub(super) fn extract_async_function_annotations(
        &self,
        func: &ast::StmtAsyncFunctionDef,
    ) -> TranspilationAnnotations {
        // Try to extract from source code comments first
        if let Some(source) = &self.source_code {
            if let Some(annotation_text) = self
                .annotation_extractor
                .extract_function_annotations(source, &func.name)
            {
                if let Ok(annotations) = self.annotation_parser.parse_annotations(&annotation_text)
                {
                    return annotations;
                }
            }
        }

        // Fallback: Try to extract from docstring if present
        if let Some(ast::Stmt::Expr(expr)) = func.body.first() {
            if let ast::Expr::Constant(constant) = expr.value.as_ref() {
                if let ast::Constant::Str(docstring) = &constant.value {
                    if let Ok(annotations) = self.annotation_parser.parse_annotations(docstring) {
                        return annotations;
                    }
                }
            }
        }

        TranspilationAnnotations::default()
    }

    pub(super) fn extract_class_annotations(
        &self,
        class: &ast::StmtClassDef,
    ) -> TranspilationAnnotations {
        // Try to extract from source code comments first
        if let Some(source) = &self.source_code {
            if let Some(annotation_text) = self
                .annotation_extractor
                .extract_class_annotations(source, &class.name)
            {
                if let Ok(annotations) = self.annotation_parser.parse_annotations(&annotation_text)
                {
                    return annotations;
                }
            }
        }

        // Fallback: Try to extract from docstring if present
        if let Some(ast::Stmt::Expr(expr)) = class.body.first() {
            if let ast::Expr::Constant(constant) = expr.value.as_ref() {
                if let ast::Constant::Str(docstring) = &constant.value {
                    if let Ok(annotations) = self.annotation_parser.parse_annotations(docstring) {
                        return annotations;
                    }
                }
            }
        }

        TranspilationAnnotations::default()
    }

    /// Check if an assignment looks like a type alias (static method for early detection)
    pub(super) fn looks_like_type_alias(assign: &ast::StmtAssign) -> bool {
        // Single target assignment
        if assign.targets.len() != 1 {
            return false;
        }

        match assign.value.as_ref() {
            // Simple alias: UserId = int
            ast::Expr::Name(_) => true,
            // Generic alias: UserId = Optional[int]
            ast::Expr::Subscript(_) => true,
            // NewType/TypeVar call: UserId = NewType('UserId', int)
            // Only match calls where the function is a simple name (not a method call)
            ast::Expr::Call(call) => matches!(call.func.as_ref(), ast::Expr::Name(_)),
            _ => false,
        }
    }

    pub(super) fn try_convert_type_alias(
        &self,
        assign: &ast::StmtAssign,
    ) -> Result<Option<TypeAlias>> {
        // Look for patterns like: UserId = int or UserId = NewType('UserId', int)
        if assign.targets.len() != 1 {
            return Ok(None); // Skip multiple assignment targets
        }

        let target = match &assign.targets[0] {
            ast::Expr::Name(name) => name.id.as_str(),
            _ => return Ok(None), // Skip complex assignment targets
        };

        // Check if this looks like a type alias (simple assignment of a type)
        let (target_type, is_newtype) = match assign.value.as_ref() {
            // Simple alias: UserId = int
            ast::Expr::Name(n) => {
                let type_name = n.id.as_str();
                if self.is_type_name(type_name) {
                    (TypeExtractor::extract_simple_type(type_name)?, false)
                } else {
                    return Ok(None); // Not a type name
                }
            }
            // Generic alias: UserId = Optional[int]
            // Only treat as type alias if the subscript base is a type name
            ast::Expr::Subscript(sub) => {
                if let ast::Expr::Name(n) = sub.value.as_ref() {
                    if self.is_type_name(n.id.as_str()) {
                        (TypeExtractor::extract_type(&assign.value)?, false)
                    } else {
                        return Ok(None); // Not a type subscript (e.g., dict["key"])
                    }
                } else {
                    return Ok(None); // Complex subscript (e.g., obj.attr["key"])
                }
            }
            // NewType pattern: UserId = NewType('UserId', int)
            ast::Expr::Call(call) => {
                if let ast::Expr::Name(func_name) = call.func.as_ref() {
                    if func_name.id.as_str() == "NewType" && call.args.len() == 2 {
                        // Second argument should be the base type
                        let base_type = TypeExtractor::extract_type(&call.args[1])?;
                        (base_type, true)
                    } else {
                        return Ok(None); // Not a NewType call
                    }
                } else {
                    return Ok(None); // Complex function call
                }
            }
            _ => return Ok(None), // Not a type alias pattern
        };

        Ok(Some(TypeAlias {
            name: target.to_string(),
            target_type,
            is_newtype,
        }))
    }

    pub(super) fn try_convert_annotated_type_alias(
        &self,
        ann_assign: &ast::StmtAnnAssign,
    ) -> Result<Option<TypeAlias>> {
        // Look for patterns like: UserId: TypeAlias = int
        let target = match ann_assign.target.as_ref() {
            ast::Expr::Name(name) => name.id.as_str(),
            _ => return Ok(None), // Skip complex assignment targets
        };

        // Check if annotation is TypeAlias
        let is_type_alias = match ann_assign.annotation.as_ref() {
            ast::Expr::Name(n) => n.id.as_str() == "TypeAlias",
            _ => false,
        };

        if !is_type_alias {
            return Ok(None); // Not explicitly marked as TypeAlias
        }

        if let Some(value) = &ann_assign.value {
            let (target_type, is_newtype) = match value.as_ref() {
                // Simple alias: UserId: TypeAlias = int
                ast::Expr::Name(n) => {
                    let type_name = n.id.as_str();
                    (TypeExtractor::extract_simple_type(type_name)?, false)
                }
                // Generic alias: UserId: TypeAlias = Optional[int]
                ast::Expr::Subscript(_) => (TypeExtractor::extract_type(value)?, false),
                // NewType pattern: UserId: TypeAlias = NewType('UserId', int)
                ast::Expr::Call(call) => {
                    if let ast::Expr::Name(func_name) = call.func.as_ref() {
                        if func_name.id.as_str() == "NewType" && call.args.len() == 2 {
                            let base_type = TypeExtractor::extract_type(&call.args[1])?;
                            (base_type, true)
                        } else {
                            return Ok(None);
                        }
                    } else {
                        return Ok(None);
                    }
                }
                _ => return Ok(None),
            };

            Ok(Some(TypeAlias {
                name: target.to_string(),
                target_type,
                is_newtype,
            }))
        } else {
            Ok(None) // No value assigned
        }
    }

    pub(super) fn convert_type_alias_stmt(
        &self,
        type_alias_stmt: &ast::StmtTypeAlias,
    ) -> Result<Option<TypeAlias>> {
        // Extract the name from the type alias statement
        let name = match type_alias_stmt.name.as_ref() {
            ast::Expr::Name(n) => n.id.as_str(),
            _ => return Ok(None), // Complex names not supported yet
        };

        // Extract the target type from the value expression
        let target_type = TypeExtractor::extract_type(&type_alias_stmt.value)?;

        // Type alias statements are always simple aliases (not newtypes)
        Ok(Some(TypeAlias {
            name: name.to_string(),
            target_type,
            is_newtype: false,
        }))
    }

    /// Check if a statement is a TypeVar assignment that should be elided
    pub(super) fn is_typevar_assignment(stmt: &ast::Stmt) -> bool {
        match stmt {
            ast::Stmt::Assign(assign) => {
                if let ast::Expr::Call(call) = assign.value.as_ref() {
                    if let ast::Expr::Name(name) = call.func.as_ref() {
                        return name.id.as_str() == "TypeVar";
                    }
                }
                false
            }
            ast::Stmt::AnnAssign(ann_assign) => {
                if let Some(value) = &ann_assign.value {
                    if let ast::Expr::Call(call) = value.as_ref() {
                        if let ast::Expr::Name(name) = call.func.as_ref() {
                            return name.id.as_str() == "TypeVar";
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Try to convert a simple assignment to a module-level constant
    pub(super) fn try_convert_constant(
        &self,
        assign: &ast::StmtAssign,
    ) -> Result<Option<HirConstant>> {
        // Only handle single assignment targets
        if assign.targets.len() != 1 {
            return Ok(None);
        }

        let name = match &assign.targets[0] {
            ast::Expr::Name(n) => n.id.to_string(),
            _ => return Ok(None), // Skip complex assignment targets
        };

        // Convert the value expression
        let value = convert_expr(*assign.value.clone())?;

        // Skip TypeVar assignments - they're only used for generic type parameters
        if let HirExpr::Call { func, .. } = &value {
            if func == "TypeVar" {
                return Ok(None);
            }
        }

        Ok(Some(HirConstant {
            name,
            value,
            type_annotation: None,
        }))
    }

    /// Try to convert an annotated assignment to a module-level constant
    pub(super) fn try_convert_annotated_constant(
        &self,
        ann_assign: &ast::StmtAnnAssign,
    ) -> Result<Option<HirConstant>> {
        let name = match ann_assign.target.as_ref() {
            ast::Expr::Name(n) => n.id.to_string(),
            _ => return Ok(None), // Skip complex assignment targets
        };

        // Extract type annotation
        let type_annotation = Some(TypeExtractor::extract_type(&ann_assign.annotation)?);

        // Get the value (annotated assignments at module level should have values)
        if let Some(value_expr) = &ann_assign.value {
            let value = convert_expr(*value_expr.clone())?;

            // Skip TypeVar assignments - they're only used for generic type parameters
            if let HirExpr::Call { func, .. } = &value {
                if func == "TypeVar" {
                    return Ok(None);
                }
            }

            Ok(Some(HirConstant {
                name,
                value,
                type_annotation,
            }))
        } else {
            Ok(None) // No value, skip it
        }
    }

    pub(super) fn is_type_name(&self, name: &str) -> bool {
        matches!(
            name,
            "int"
                | "float"
                | "str"
                | "bool"
                | "None"
                | "list"
                | "dict"
                | "tuple"
                | "set"
                | "frozenset"
                | "List"
                | "Dict"
                | "Tuple"
                | "Set"
                | "FrozenSet"
                | "Optional"
                | "Union"
                | "Callable"
                | "Any"
                | "TypeVar"
        )
    }

    pub(super) fn try_convert_protocol(
        &self,
        class: &ast::StmtClassDef,
    ) -> Result<Option<Protocol>> {
        // Check if this class inherits from Protocol
        let is_protocol = class
            .bases
            .iter()
            .any(|base| matches!(base, ast::Expr::Name(n) if n.id.as_str() == "Protocol"));

        if !is_protocol {
            return Ok(None);
        }

        let name = class.name.to_string();

        // Extract type parameters from class definition
        let type_params = self.extract_class_type_params(class);

        // Check for @runtime_checkable decorator
        let is_runtime_checkable = class
            .decorator_list
            .iter()
            .any(|decorator| matches!(decorator, ast::Expr::Name(n) if n.id.as_str() == "runtime_checkable"));

        // Extract methods from class body
        let mut methods = Vec::new();
        for stmt in &class.body {
            if let ast::Stmt::FunctionDef(func) = stmt {
                // Skip special methods like __init__, but include abstract methods
                if !func.name.as_str().starts_with("__") || func.name.as_str() == "__call__" {
                    let method = self.convert_protocol_method(func)?;
                    methods.push(method);
                }
            }
        }

        Ok(Some(Protocol {
            name,
            type_params,
            methods,
            is_runtime_checkable,
        }))
    }

    pub(super) fn try_convert_class(&self, class: &ast::StmtClassDef) -> Result<Option<HirClass>> {
        // Extract docstring if present
        let docstring = self.extract_class_docstring(&class.body);

        // Extract class annotations (from docstring or source comments)
        let annotations = self.extract_class_annotations(class);

        // Check if it's a dataclass
        let is_dataclass = class.decorator_list.iter().any(|d| {
            matches!(d, ast::Expr::Name(n) if n.id.as_str() == "dataclass")
                || matches!(d, ast::Expr::Attribute(a) if a.attr.as_str() == "dataclass")
        });

        // Extract base classes (for now, just store the names)
        let base_classes: Vec<String> = class
            .bases
            .iter()
            .filter_map(|base| {
                if let ast::Expr::Name(n) = base {
                    Some(n.id.to_string())
                } else {
                    None
                }
            })
            .collect();

        let is_enum = base_classes
            .iter()
            .any(|b| b == "IntEnum" || b == "Enum" || b == "IntFlag");
        let is_intflag = base_classes.iter().any(|b| b == "IntFlag");

        // Check if this is an ABC (Abstract Base Class)
        // A class is an ABC if it:
        // 1. Inherits from ABC
        // 2. Uses ABCMeta as metaclass
        let is_abc = base_classes.iter().any(|b| b == "ABC")
            || class.keywords.iter().any(|kw| {
                kw.arg
                    .as_ref()
                    .map_or(false, |name| name.as_str() == "metaclass")
                    && matches!(&kw.value, ast::Expr::Name(n) if n.id.as_str() == "ABCMeta")
            });

        // Debug output
        if !base_classes.is_empty() {
            eprintln!(
                "Class: {}, base_classes: {:?}, is_abc: {}",
                class.name, base_classes, is_abc
            );
        }

        // Convert methods and fields
        let mut methods = Vec::new();
        let mut fields = Vec::new();
        let mut init_method = None;

        for stmt in &class.body {
            match stmt {
                ast::Stmt::FunctionDef(method) => {
                    if method.name.as_str() == "__init__" {
                        // Store __init__ for field inference
                        init_method = Some(method);
                    }
                    if let Some(hir_method) = self.convert_method(method, false)? {
                        methods.push(hir_method);
                    }
                }
                ast::Stmt::AsyncFunctionDef(method) => {
                    if let Some(hir_method) = self.convert_async_method(method)? {
                        methods.push(hir_method);
                    }
                }
                ast::Stmt::AnnAssign(ann_assign) => {
                    // Handle annotated fields (class attributes)
                    if let ast::Expr::Name(target) = ann_assign.target.as_ref() {
                        let field_name = target.id.to_string();
                        let field_type = TypeExtractor::extract_type(&ann_assign.annotation)?;

                        // Determine if this is a class variable or instance field
                        let (is_class_var, default_value) = if let Some(value) = &ann_assign.value {
                            // Check if this is a field() call (dataclass field with metadata)
                            let is_field_call = matches!(value.as_ref(),
                                ast::Expr::Call(call) if matches!(call.func.as_ref(),
                                    ast::Expr::Name(n) if n.id.as_str() == "field"
                                )
                            );

                            // For dataclasses:
                            // - field() calls with default_factory are instance fields
                            // - Regular default values are also instance fields (unless ClassVar)
                            // For non-dataclasses:
                            // - Default values make it a class constant
                            let is_instance_field = is_dataclass && (is_field_call || true);

                            // Convert the default value expression
                            let converted_value = ExprConverter::convert(value.as_ref().clone())?;

                            (!is_instance_field, Some(converted_value))
                        } else {
                            // Instance attribute - no default value
                            (false, None)
                        };

                        fields.push(HirField {
                            name: field_name,
                            field_type,
                            default_value,
                            is_class_var,
                        });
                    }
                }
                ast::Stmt::Assign(assign) => {
                    // Extract class variables from simple assignments
                    // For enums, these are enum members
                    // For regular classes, these are class-level variables (like static in Rust)
                    for target in &assign.targets {
                        if let ast::Expr::Name(name) = target {
                            let field_name = name.id.to_string();

                            // For enums, use Int type; for regular classes, infer from value
                            let field_type = if is_enum {
                                Type::Int
                            } else {
                                // Try to infer type from the assignment value
                                self.infer_type_from_expr(&assign.value)
                                    .unwrap_or(Type::Unknown)
                            };

                            let converted_value =
                                ExprConverter::convert(assign.value.as_ref().clone())?;

                            fields.push(HirField {
                                name: field_name,
                                field_type,
                                default_value: Some(converted_value),
                                is_class_var: true, // All simple assignments in class body are class variables
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        // Infer instance fields from __init__ if no explicit instance fields are defined
        // Check if we have any instance fields (non-class-var fields)
        let has_instance_fields = fields.iter().any(|f| !f.is_class_var);

        if !has_instance_fields && !is_dataclass {
            if let Some(init) = init_method {
                // Infer instance fields and add them to the existing fields (which may contain class vars)
                let inferred_fields = self.infer_fields_from_init(init)?;
                fields.extend(inferred_fields);
            }
        }

        // Check if class defines __getattr__ or __setattr__ methods
        // If so, we need to generate _get_field/_set_field methods for dynamic attribute access
        let needs_dynamic_field_access = methods
            .iter()
            .any(|m| m.name == "__getattr__" || m.name == "__setattr__");

        Ok(Some(HirClass {
            name: class.name.to_string(),
            base_classes,
            methods,
            fields,
            is_dataclass,
            is_enum,
            is_intflag,
            is_abc,
            docstring,
            annotations,
            needs_dynamic_field_access,
        }))
    }

    pub(super) fn convert_method(
        &self,
        method: &ast::StmtFunctionDef,
        is_async: bool,
    ) -> Result<Option<HirMethod>> {
        use smallvec::smallvec;

        let name = method.name.to_string();

        // Skip dunder methods except __init__, __del__, __iter__, __next__, __enter__, __exit__
        if name.starts_with("__")
            && name.ends_with("__")
            && !matches!(
                name.as_str(),
                "__init__" | "__del__" | "__iter__" | "__next__" | "__enter__" | "__exit__"
            )
        {
            return Ok(None);
        }

        // Extract docstring
        let docstring = self.extract_class_docstring(&method.body);

        // Check decorators
        let is_static = method
            .decorator_list
            .iter()
            .any(|d| matches!(d, ast::Expr::Name(n) if n.id.as_str() == "staticmethod"));
        let is_classmethod = method
            .decorator_list
            .iter()
            .any(|d| matches!(d, ast::Expr::Name(n) if n.id.as_str() == "classmethod"));
        let is_property = method
            .decorator_list
            .iter()
            .any(|d| matches!(d, ast::Expr::Name(n) if n.id.as_str() == "property"));
        let is_abstract = method
            .decorator_list
            .iter()
            .any(|d| matches!(d, ast::Expr::Name(n) if n.id.as_str() == "abstractmethod"));

        // Convert parameters (skip 'self' for regular methods, 'cls' for classmethods)
        let mut params = smallvec![];
        let skip_first = if is_static {
            false
        } else if is_classmethod {
            // Skip 'cls' parameter for classmethods
            method
                .args
                .args
                .first()
                .map(|arg| arg.def.arg.as_str() == "cls")
                .unwrap_or(false)
        } else {
            // Skip 'self' parameter for instance methods
            method
                .args
                .args
                .first()
                .map(|arg| arg.def.arg.as_str() == "self")
                .unwrap_or(false)
        };

        let args_to_process = if skip_first {
            &method.args.args[1..]
        } else {
            &method.args.args[..]
        };

        for arg in args_to_process {
            let param_name = arg.def.arg.to_string();
            let param_type = if let Some(ann) = &arg.def.annotation {
                TypeExtractor::extract_type(ann)?
            } else {
                Type::Unknown
            };
            params.push(HirParam {
                name: param_name,
                ty: param_type,
                default: None, // Note: Method defaults extraction requires AST alignment with convert_parameters()
            });
        }

        // Convert return type
        let ret_type = if let Some(ret) = &method.returns {
            TypeExtractor::extract_type(ret)?
        } else if self.check_returns_self(&method.body) {
            // If method returns self without annotation, infer &Self
            Type::Custom("&Self".to_string())
        } else {
            Type::None
        };

        // Convert body (filter out docstring)
        let filtered_body = if docstring.is_some() && method.body.len() > 1 {
            // Skip first statement if it's the docstring
            method.body[1..].to_vec()
        } else if docstring.is_some() && method.body.len() == 1 {
            // Only docstring, no actual body
            vec![]
        } else {
            method.body.clone()
        };
        let body = convert_body(filtered_body)?;

        let mut hir_method = HirMethod {
            name,
            params,
            ret_type,
            body,
            is_static,
            is_classmethod,
            is_property,
            is_async,
            is_abstract,
            docstring,
        };

        // Infer parameter types if any parameters have Type::Unknown
        self.infer_method_parameter_types(&mut hir_method);

        Ok(Some(hir_method))
    }

    /// Infer parameter types from method body usage
    pub(super) fn infer_method_parameter_types(&self, method: &mut HirMethod) {
        // Check if any parameters need type inference
        let needs_inference = method.params.iter().any(|p| matches!(p.ty, Type::Unknown));
        if !needs_inference {
            return;
        }

        // Convert HirMethod to HirFunction for type inference
        let temp_function = HirFunction {
            name: method.name.clone(),
            params: method.params.clone(),
            ret_type: method.ret_type.clone(),
            body: method.body.clone(),
            properties: FunctionProperties::default(),
            annotations: Default::default(),
            docstring: method.docstring.clone(),
        };

        // Use type hint provider to analyze usage patterns
        let mut hint_provider = TypeHintProvider::new();
        if let Ok(hints) = hint_provider.analyze_function(&temp_function) {
            // Update parameter types with inferred types
            for param in &mut method.params {
                if matches!(param.ty, Type::Unknown) {
                    // Find hint for this parameter
                    for hint in &hints {
                        if let crate::types::type_hints::HintTarget::Parameter(param_name) =
                            &hint.target
                        {
                            if param_name == &param.name {
                                // For class/instance methods, be more lenient and accept any confidence level
                                // if the type is a simple concrete type (Int, Float, String, Bool)
                                let is_simple_type = matches!(
                                    hint.suggested_type,
                                    Type::Int | Type::Float | Type::String | Type::Bool
                                );
                                if is_simple_type {
                                    param.ty = hint.suggested_type.clone();
                                } else if hint.confidence
                                    >= crate::types::type_hints::Confidence::High
                                {
                                    param.ty = hint.suggested_type.clone();
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Infer parameter types from function body usage
    pub(super) fn infer_function_parameter_types(&self, function: &mut HirFunction) {
        // Check if any parameters need type inference
        let needs_inference = function
            .params
            .iter()
            .any(|p| matches!(p.ty, Type::Unknown));
        if !needs_inference {
            return;
        }

        // Use type hint provider to analyze usage patterns
        let mut hint_provider = TypeHintProvider::new();
        if let Ok(hints) = hint_provider.analyze_function(function) {
            // Update parameter types with inferred types
            for param in &mut function.params {
                if matches!(param.ty, Type::Unknown) {
                    // Find hint for this parameter
                    for hint in &hints {
                        if let crate::types::type_hints::HintTarget::Parameter(param_name) =
                            &hint.target
                        {
                            if param_name == &param.name {
                                // For standalone functions, be lenient and accept any confidence level
                                // if the type is a simple concrete type (Int, Float, String, Bool)
                                let is_simple_type = matches!(
                                    hint.suggested_type,
                                    Type::Int | Type::Float | Type::String | Type::Bool
                                );
                                if is_simple_type {
                                    param.ty = hint.suggested_type.clone();
                                } else if hint.confidence
                                    >= crate::types::type_hints::Confidence::High
                                {
                                    param.ty = hint.suggested_type.clone();
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    pub(super) fn convert_async_method(
        &self,
        method: &ast::StmtAsyncFunctionDef,
    ) -> Result<Option<HirMethod>> {
        use smallvec::smallvec;

        let name = method.name.to_string();

        // Skip dunder methods except __init__, __del__, __iter__, __next__, __enter__, __exit__, __aenter__, __aexit__
        if name.starts_with("__")
            && name.ends_with("__")
            && !matches!(
                name.as_str(),
                "__init__"
                    | "__del__"
                    | "__iter__"
                    | "__next__"
                    | "__enter__"
                    | "__exit__"
                    | "__aenter__"
                    | "__aexit__"
                    | "__anext__"
                    | "__aiter__"
            )
        {
            return Ok(None);
        }

        // Extract docstring
        let docstring = self.extract_class_docstring(&method.body);

        // Check decorators
        let is_static = method
            .decorator_list
            .iter()
            .any(|d| matches!(d, ast::Expr::Name(n) if n.id.as_str() == "staticmethod"));
        let is_classmethod = method
            .decorator_list
            .iter()
            .any(|d| matches!(d, ast::Expr::Name(n) if n.id.as_str() == "classmethod"));
        let is_property = method
            .decorator_list
            .iter()
            .any(|d| matches!(d, ast::Expr::Name(n) if n.id.as_str() == "property"));
        let is_abstract = method
            .decorator_list
            .iter()
            .any(|d| matches!(d, ast::Expr::Name(n) if n.id.as_str() == "abstractmethod"));

        // Convert parameters
        let mut params = smallvec![];
        let skip_first = if is_static {
            false
        } else if is_classmethod {
            // Skip 'cls' parameter for classmethods
            method
                .args
                .args
                .first()
                .map(|arg| arg.def.arg.as_str() == "cls")
                .unwrap_or(false)
        } else {
            // Skip 'self' parameter for instance methods
            method
                .args
                .args
                .first()
                .map(|arg| arg.def.arg.as_str() == "self")
                .unwrap_or(false)
        };

        let args_to_process = if skip_first {
            &method.args.args[1..]
        } else {
            &method.args.args[..]
        };

        for arg in args_to_process {
            let param_name = arg.def.arg.to_string();
            let param_type = if let Some(ann) = &arg.def.annotation {
                TypeExtractor::extract_type(ann)?
            } else {
                Type::Unknown
            };
            params.push(HirParam {
                name: param_name,
                ty: param_type,
                default: None, // Note: Method defaults extraction requires AST alignment with convert_parameters()
            });
        }

        // Convert return type
        let ret_type = if let Some(ret) = &method.returns {
            TypeExtractor::extract_type(ret)?
        } else if self.check_returns_self(&method.body) {
            // If method returns self without annotation, infer &Self
            Type::Custom("&Self".to_string())
        } else {
            Type::None
        };

        // Convert body (filter out docstring)
        let filtered_body = if docstring.is_some() && method.body.len() > 1 {
            // Skip first statement if it's the docstring
            method.body[1..].to_vec()
        } else if docstring.is_some() && method.body.len() == 1 {
            // Only docstring, no actual body
            vec![]
        } else {
            method.body.clone()
        };
        let body = convert_body(filtered_body)?;

        let mut hir_method = HirMethod {
            name,
            params,
            ret_type,
            body,
            is_static,
            is_classmethod,
            is_property,
            is_async: true,
            is_abstract,
            docstring,
        };

        // Infer parameter types if any parameters have Type::Unknown
        self.infer_method_parameter_types(&mut hir_method);

        Ok(Some(hir_method))
    }

    pub(super) fn extract_class_docstring(&self, body: &[ast::Stmt]) -> Option<String> {
        if let Some(ast::Stmt::Expr(expr)) = body.first() {
            if let ast::Expr::Constant(c) = expr.value.as_ref() {
                if let ast::Constant::Str(s) = &c.value {
                    return Some(s.to_string());
                }
            }
        }
        None
    }

    pub(super) fn extract_class_type_params(&self, class: &ast::StmtClassDef) -> Vec<String> {
        // Look for Generic[T, U] in base classes
        for base in &class.bases {
            if let ast::Expr::Subscript(subscript) = base {
                if let ast::Expr::Name(n) = subscript.value.as_ref() {
                    if n.id.as_str() == "Generic" {
                        return self.extract_generic_params(&subscript.slice);
                    }
                }
            }
        }
        Vec::new()
    }

    pub(super) fn extract_generic_params(&self, slice: &ast::Expr) -> Vec<String> {
        match slice {
            ast::Expr::Name(n) => vec![n.id.to_string()],
            ast::Expr::Tuple(tuple) => tuple
                .elts
                .iter()
                .filter_map(|elt| {
                    if let ast::Expr::Name(n) = elt {
                        Some(n.id.to_string())
                    } else {
                        None
                    }
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    pub(super) fn convert_protocol_method(
        &self,
        func: &ast::StmtFunctionDef,
    ) -> Result<ProtocolMethod> {
        let name = func.name.to_string();
        let params = convert_parameters(&func.args)?;
        let ret_type = TypeExtractor::extract_return_type(&func.returns)?;

        // Check if method has @abstractmethod decorator
        let is_optional = !func
            .decorator_list
            .iter()
            .any(|decorator| matches!(decorator, ast::Expr::Name(n) if n.id.as_str() == "abstractmethod"));

        // Check if method has a default implementation (non-empty body beyond docstring)
        let has_default = self.method_has_default_implementation(&func.body);

        Ok(ProtocolMethod {
            name,
            params: params.into(),
            ret_type,
            is_optional,
            has_default,
        })
    }

    pub(super) fn method_has_default_implementation(&self, body: &[ast::Stmt]) -> bool {
        // Filter out docstrings and ellipsis statements
        let meaningful_stmts: Vec<_> = body
            .iter()
            .filter(|stmt| {
                match stmt {
                    // Skip docstring
                    ast::Stmt::Expr(expr)
                        if matches!(expr.value.as_ref(),
                        ast::Expr::Constant(c) if matches!(c.value, ast::Constant::Str(_))) =>
                    {
                        false
                    }
                    // Skip ellipsis (...)
                    ast::Stmt::Expr(expr)
                        if matches!(expr.value.as_ref(),
                        ast::Expr::Constant(c) if matches!(c.value, ast::Constant::Ellipsis)) =>
                    {
                        false
                    }
                    _ => true,
                }
            })
            .collect();

        !meaningful_stmts.is_empty()
    }

    pub(super) fn infer_fields_from_init(
        &self,
        init: &ast::StmtFunctionDef,
    ) -> Result<Vec<HirField>> {
        let mut fields = Vec::new();

        // Get parameter types from __init__ signature
        let mut param_types = std::collections::HashMap::new();
        for arg in &init.args.args {
            if arg.def.arg.as_str() != "self" {
                let param_name = arg.def.arg.to_string();
                let param_type = if let Some(annotation) = &arg.def.annotation {
                    TypeExtractor::extract_type(annotation)?
                } else {
                    Type::Unknown
                };
                param_types.insert(param_name, param_type);
            }
        }

        // Look for self.field assignments in __init__
        for stmt in &init.body {
            // Handle annotated assignments: self.field: type = value
            if let ast::Stmt::AnnAssign(ann_assign) = stmt {
                if let ast::Expr::Attribute(attr) = ann_assign.target.as_ref() {
                    if let ast::Expr::Name(name) = attr.value.as_ref() {
                        if name.id.as_str() == "self" {
                            let field_name = attr.attr.to_string();
                            let field_type = TypeExtractor::extract_type(&ann_assign.annotation)?;
                            fields.push(HirField {
                                name: field_name,
                                field_type,
                                default_value: ann_assign
                                    .value
                                    .as_ref()
                                    .and_then(|v| ExprConverter::convert(v.as_ref().clone()).ok()),
                                is_class_var: false,
                            });
                        }
                    }
                }
            }
            if let ast::Stmt::Assign(assign) = stmt {
                // Check if it's a self.field assignment
                if assign.targets.len() == 1 {
                    if let ast::Expr::Attribute(attr) = &assign.targets[0] {
                        if let ast::Expr::Name(name) = attr.value.as_ref() {
                            if name.id.as_str() == "self" {
                                let field_name = attr.attr.to_string();

                                // Try to infer type from the assigned value
                                let field_type =
                                    if let ast::Expr::Name(value_name) = assign.value.as_ref() {
                                        // If assigning a parameter, use its type
                                        param_types
                                            .get(value_name.id.as_str())
                                            .cloned()
                                            .unwrap_or(Type::Unknown)
                                    } else {
                                        // Otherwise, try to infer from literal or default to Unknown
                                        self.infer_type_from_expr(assign.value.as_ref())
                                            .unwrap_or(Type::Unknown)
                                    };

                                fields.push(HirField {
                                    name: field_name,
                                    field_type,
                                    default_value: None,
                                    is_class_var: false,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(fields)
    }

    pub(super) fn infer_type_from_expr(&self, expr: &ast::Expr) -> Option<Type> {
        match expr {
            ast::Expr::Constant(c) => match &c.value {
                ast::Constant::Int(_) => Some(Type::Int),
                ast::Constant::Float(_) => Some(Type::Float),
                ast::Constant::Str(_) => Some(Type::String),
                ast::Constant::Bool(_) => Some(Type::Bool),
                ast::Constant::None => Some(Type::None),
                _ => None,
            },
            ast::Expr::List(list) => {
                if list.elts.is_empty() {
                    Some(Type::List(Box::new(Type::Unknown)))
                } else {
                    let elem_type = self
                        .infer_type_from_expr(&list.elts[0])
                        .unwrap_or(Type::Unknown);
                    Some(Type::List(Box::new(elem_type)))
                }
            }
            ast::Expr::Dict(dict) => {
                if dict.keys.is_empty() {
                    Some(Type::Dict(Box::new(Type::Unknown), Box::new(Type::Unknown)))
                } else {
                    let key_type = dict.keys[0]
                        .as_ref()
                        .and_then(|k| self.infer_type_from_expr(k))
                        .unwrap_or(Type::Unknown);
                    let value_type = self
                        .infer_type_from_expr(&dict.values[0])
                        .unwrap_or(Type::Unknown);
                    Some(Type::Dict(Box::new(key_type), Box::new(value_type)))
                }
            }
            ast::Expr::Set(set) => {
                if set.elts.is_empty() {
                    Some(Type::Set(Box::new(Type::Unknown)))
                } else {
                    let elem_type = self
                        .infer_type_from_expr(&set.elts[0])
                        .unwrap_or(Type::Unknown);
                    Some(Type::Set(Box::new(elem_type)))
                }
            }
            _ => None,
        }
    }

    pub(super) fn check_returns_self(&self, body: &[ast::Stmt]) -> bool {
        for stmt in body {
            if let ast::Stmt::Return(ret_stmt) = stmt {
                if let Some(value) = &ret_stmt.value {
                    if let ast::Expr::Name(n) = value.as_ref() {
                        if n.id.as_str() == "self" {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}

///
/// This function performs a fixed-point iteration to propagate the `can_fail` property
/// from callees to callers. If function A calls function B, and B can fail, then A
/// can also fail (unless it catches the error).
///
/// This is essential for correct Result type propagation in recursive functions.
///
/// Complexity: O(n * m) where n = number of functions, m = max call depth
pub(super) fn propagate_can_fail_through_calls(functions: &mut [HirFunction]) {
    // Build a map of function names to can_fail status for quick lookup
    let mut can_fail_map: std::collections::HashMap<String, bool> = functions
        .iter()
        .map(|f| (f.name.clone(), f.properties.can_fail))
        .collect();

    // Fixed-point iteration: keep propagating until no changes occur
    let mut changed = true;
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 100; // Prevent infinite loops

    while changed && iterations < MAX_ITERATIONS {
        changed = false;
        iterations += 1;

        for func in functions.iter_mut() {
            // Skip if already marked as can_fail
            if func.properties.can_fail {
                continue;
            }

            // Check if this function calls any function that can fail
            if calls_failing_function(&func.body, &can_fail_map) {
                func.properties.can_fail = true;
                can_fail_map.insert(func.name.clone(), true);
                changed = true;
            }
        }
    }
}

/// Check if a statement sequence contains calls to functions that can fail
pub(super) fn calls_failing_function(
    stmts: &[HirStmt],
    can_fail_map: &std::collections::HashMap<String, bool>,
) -> bool {
    for stmt in stmts {
        if stmt_calls_failing_function(stmt, can_fail_map) {
            return true;
        }
    }
    false
}

/// Check if a statement calls a function that can fail
pub(super) fn stmt_calls_failing_function(
    stmt: &HirStmt,
    can_fail_map: &std::collections::HashMap<String, bool>,
) -> bool {
    match stmt {
        HirStmt::Return(Some(expr)) | HirStmt::Expr(expr) | HirStmt::Assign { value: expr, .. } => {
            expr_calls_failing_function(expr, can_fail_map)
        }
        HirStmt::If {
            condition,
            then_body,
            else_body,
        } => {
            expr_calls_failing_function(condition, can_fail_map)
                || calls_failing_function(then_body, can_fail_map)
                || else_body
                    .as_ref()
                    .map(|body| calls_failing_function(body, can_fail_map))
                    .unwrap_or(false)
        }
        HirStmt::While { condition, body } => {
            expr_calls_failing_function(condition, can_fail_map)
                || calls_failing_function(body, can_fail_map)
        }
        HirStmt::For { iter, body, .. } => {
            expr_calls_failing_function(iter, can_fail_map)
                || calls_failing_function(body, can_fail_map)
        }
        HirStmt::Try {
            body,
            handlers,
            finalbody,
            ..
        } => {
            calls_failing_function(body, can_fail_map)
                || handlers
                    .iter()
                    .any(|h| calls_failing_function(&h.body, can_fail_map))
                || finalbody
                    .as_ref()
                    .map(|fb| calls_failing_function(fb, can_fail_map))
                    .unwrap_or(false)
        }
        _ => false,
    }
}

/// Check if an expression contains calls to functions that can fail
pub(super) fn expr_calls_failing_function(
    expr: &HirExpr,
    can_fail_map: &std::collections::HashMap<String, bool>,
) -> bool {
    match expr {
        HirExpr::Call { func, args, .. } => {
            // Check if the called function is known to fail
            if can_fail_map.get(func).copied().unwrap_or(false) {
                return true;
            }
            // Also check arguments recursively
            args.iter()
                .any(|arg| expr_calls_failing_function(arg, can_fail_map))
        }
        HirExpr::Binary { left, right, .. } => {
            expr_calls_failing_function(left, can_fail_map)
                || expr_calls_failing_function(right, can_fail_map)
        }
        HirExpr::Unary { operand, .. } => expr_calls_failing_function(operand, can_fail_map),
        HirExpr::List(elements) | HirExpr::Tuple(elements) | HirExpr::Set(elements) => elements
            .iter()
            .any(|e| expr_calls_failing_function(e, can_fail_map)),
        HirExpr::MethodCall { object, args, .. } => {
            expr_calls_failing_function(object, can_fail_map)
                || args
                    .iter()
                    .any(|arg| expr_calls_failing_function(arg, can_fail_map))
        }
        HirExpr::Index { base, index } => {
            expr_calls_failing_function(base, can_fail_map)
                || expr_calls_failing_function(index, can_fail_map)
        }
        HirExpr::Slice { base, .. } => expr_calls_failing_function(base, can_fail_map),
        _ => false,
    }
}

pub(crate) fn convert_parameters(args: &ast::Arguments) -> Result<Vec<HirParam>> {
    use crate::ast_bridge::converters::ExprConverter;
    let mut params = Vec::new();

    // Calculate number of args without defaults
    let num_args = args.args.len();
    let defaults_vec: Vec<_> = args.defaults().collect();
    let num_defaults = defaults_vec.len();
    let first_default_idx = num_args.saturating_sub(num_defaults);

    for (i, arg) in args.args.iter().enumerate() {
        let name = arg.def.arg.to_string();
        let ty = if let Some(annotation) = &arg.def.annotation {
            TypeExtractor::extract_type(annotation)?
        } else {
            Type::Unknown
        };

        // Check if this parameter has a default value
        let default = if i >= first_default_idx {
            let default_idx = i - first_default_idx;
            if let Some(default_expr) = defaults_vec.get(default_idx) {
                Some(ExprConverter::convert((*default_expr).clone())?)
            } else {
                None
            }
        } else {
            None
        };

        params.push(HirParam { name, ty, default });
    }

    Ok(params)
}

pub(crate) fn convert_body(body: Vec<ast::Stmt>) -> Result<Vec<HirStmt>> {
    body.into_iter().map(convert_stmt).collect()
}

pub(crate) fn convert_stmt(stmt: ast::Stmt) -> Result<HirStmt> {
    StmtConverter::convert(stmt)
}

pub(crate) fn extract_assign_target(expr: &ast::Expr) -> Result<AssignTarget> {
    use crate::ast_bridge::converters::ExprConverter;
    match expr {
        ast::Expr::Name(n) => Ok(AssignTarget::Symbol(n.id.to_string())),
        ast::Expr::Subscript(s) => {
            let base = Box::new(ExprConverter::convert(s.value.as_ref().clone())?);
            // Check if slice is a slice expression or a simple index
            match s.slice.as_ref() {
                ast::Expr::Slice(slice_expr) => {
                    let start = slice_expr
                        .lower
                        .as_ref()
                        .map(|e| ExprConverter::convert(e.as_ref().clone()))
                        .transpose()?
                        .map(Box::new);
                    let stop = slice_expr
                        .upper
                        .as_ref()
                        .map(|e| ExprConverter::convert(e.as_ref().clone()))
                        .transpose()?
                        .map(Box::new);
                    let step = slice_expr
                        .step
                        .as_ref()
                        .map(|e| ExprConverter::convert(e.as_ref().clone()))
                        .transpose()?
                        .map(Box::new);
                    Ok(AssignTarget::Slice {
                        base,
                        start,
                        stop,
                        step,
                    })
                }
                _ => {
                    let index = Box::new(ExprConverter::convert(s.slice.as_ref().clone())?);
                    Ok(AssignTarget::Index { base, index })
                }
            }
        }
        ast::Expr::Attribute(a) => {
            let value = Box::new(ExprConverter::convert(a.value.as_ref().clone())?);
            Ok(AssignTarget::Attribute {
                value,
                attr: a.attr.to_string(),
            })
        }
        ast::Expr::Tuple(t) => {
            let targets = t
                .elts
                .iter()
                .map(extract_assign_target)
                .collect::<Result<Vec<_>>>()?;
            Ok(AssignTarget::Tuple(targets))
        }
        ast::Expr::Starred(s) => {
            // Starred expression in unpacking: *rest = [1, 2, 3]
            // The value inside should be a simple name
            match s.value.as_ref() {
                ast::Expr::Name(n) => Ok(AssignTarget::Starred(n.id.to_string())),
                _ => bail!("Starred expression in assignment must be a simple name"),
            }
        }
        _ => bail!("Unsupported assignment target"),
    }
}

pub(crate) fn convert_expr(expr: ast::Expr) -> Result<HirExpr> {
    ExprConverter::convert(expr)
}

pub(crate) fn convert_binop(op: &ast::Operator) -> Result<BinOp> {
    Ok(match op {
        ast::Operator::Add => BinOp::Add,
        ast::Operator::Sub => BinOp::Sub,
        ast::Operator::Mult => BinOp::Mul,
        ast::Operator::Div => BinOp::Div,
        ast::Operator::FloorDiv => BinOp::FloorDiv,
        ast::Operator::Mod => BinOp::Mod,
        ast::Operator::Pow => BinOp::Pow,
        ast::Operator::MatMult => BinOp::MatMul,
        ast::Operator::BitAnd => BinOp::BitAnd,
        ast::Operator::BitOr => BinOp::BitOr,
        ast::Operator::BitXor => BinOp::BitXor,
        ast::Operator::LShift => BinOp::LShift,
        ast::Operator::RShift => BinOp::RShift,
        _ => bail!("Unsupported binary operator"),
    })
}

pub(crate) fn convert_aug_op(op: &ast::Operator) -> Result<BinOp> {
    // Augmented assignment operators map to the same binary operators
    convert_binop(op)
}

pub(crate) fn convert_unaryop(op: &ast::UnaryOp) -> Result<UnaryOp> {
    Ok(match op {
        ast::UnaryOp::Not => UnaryOp::Not,
        ast::UnaryOp::UAdd => UnaryOp::Pos,
        ast::UnaryOp::USub => UnaryOp::Neg,
        ast::UnaryOp::Invert => UnaryOp::BitNot,
    })
}

pub(crate) fn convert_cmpop(op: &ast::CmpOp) -> Result<BinOp> {
    Ok(match op {
        ast::CmpOp::Eq => BinOp::Eq,
        ast::CmpOp::NotEq => BinOp::NotEq,
        ast::CmpOp::Lt => BinOp::Lt,
        ast::CmpOp::LtE => BinOp::LtEq,
        ast::CmpOp::Gt => BinOp::Gt,
        ast::CmpOp::GtE => BinOp::GtEq,
        ast::CmpOp::In => BinOp::In,
        ast::CmpOp::NotIn => BinOp::NotIn,
        // Map identity comparisons to value equality as a pragmatic fallback
        ast::CmpOp::Is => BinOp::Is,
        ast::CmpOp::IsNot => BinOp::IsNot,
    })
}

pub(crate) fn convert_import(import: ast::StmtImport) -> Result<Vec<Import>> {
    import
        .names
        .into_iter()
        .map(|alias| {
            let module = alias.name.to_string();
            // For "import module" or "import module as alias", we import the whole module
            let items = vec![];
            Ok(Import { module, items })
        })
        .collect()
}

pub(crate) fn convert_import_from(import: ast::StmtImportFrom) -> Result<Vec<Import>> {
    let module = import.module.map(|m| m.to_string()).unwrap_or_default();

    let items = import
        .names
        .into_iter()
        .map(|alias| {
            let name = alias.name.to_string();
            if let Some(asname) = alias.asname {
                ImportItem::Aliased {
                    name,
                    alias: asname.to_string(),
                }
            } else {
                ImportItem::Named(name)
            }
        })
        .collect();

    Ok(vec![Import { module, items }])
}

pub(crate) fn extract_docstring_and_body(
    body: Vec<ast::Stmt>,
) -> Result<(Option<String>, Vec<HirStmt>)> {
    if body.is_empty() {
        return Ok((None, vec![]));
    }

    // Check if the first statement is a string literal (docstring)
    let docstring = if let ast::Stmt::Expr(expr) = &body[0] {
        if let ast::Expr::Constant(constant) = expr.value.as_ref() {
            if let ast::Constant::Str(s) = &constant.value {
                Some(s.clone())
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // Convert the body, skipping the docstring if it exists
    let start_index = if docstring.is_some() { 1 } else { 0 };
    let filtered_body = body
        .into_iter()
        .skip(start_index)
        .map(convert_stmt)
        .collect::<Result<Vec<_>>>()?;

    Ok((docstring, filtered_body))
}

/// Infer parameter and field types from class names in the module.
/// This handles the case where parameters or fields have no type annotation
/// but their names suggest they should be of a class type.
pub(crate) fn infer_parameter_types_from_classes(classes: &mut [HirClass]) {
    use crate::hir::Type;

    // Collect all class names
    let class_names: Vec<String> = classes.iter().map(|c| c.name.clone()).collect();

    // For each class, check method parameters and fields
    for class in classes.iter_mut() {
        // Check method parameters
        for method in &mut class.methods {
            for param in &mut method.params {
                if matches!(param.ty, Type::Unknown) {
                    // Check if parameter name matches a class name
                    // Try exact match (case-sensitive)
                    if let Some(matching_class) =
                        class_names.iter().find(|cn| cn.as_str() == param.name)
                    {
                        param.ty = Type::Custom(matching_class.clone());
                    } else {
                        // Try capitalized version of parameter name
                        let capitalized = capitalize_first(&param.name);
                        if let Some(matching_class) =
                            class_names.iter().find(|cn| cn.as_str() == capitalized)
                        {
                            param.ty = Type::Custom(matching_class.clone());
                        }
                    }
                }
            }
        }

        // Check fields with Unknown type
        for field in &mut class.fields {
            if matches!(field.field_type, Type::None) {
                // Field initialized to None - should be Option<T>
                // Try to infer T from the field name
                let capitalized = capitalize_first(&field.name);
                if let Some(matching_class) =
                    class_names.iter().find(|cn| cn.as_str() == capitalized)
                {
                    field.field_type =
                        Type::Optional(Box::new(Type::Custom(matching_class.clone())));
                } else if let Some(matching_class) =
                    class_names.iter().find(|cn| cn.as_str() == field.name)
                {
                    field.field_type =
                        Type::Optional(Box::new(Type::Custom(matching_class.clone())));
                }
            }
        }
    }
}

/// Capitalize the first character of a string
pub(super) fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
