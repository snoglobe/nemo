use anyhow::{anyhow, Result};
use pest::Parser;
use pest_derive::Parser;
use pest::iterators::{Pair, Pairs};

use crate::ast::*;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct LangParser;

pub fn parse(source: &str) -> Result<Program> {
    let pairs = LangParser::parse(Rule::program, source)
        .map_err(|e| anyhow!("Parse error: {}", e))?;
    
    let mut items = Vec::new();
    
    for pair in pairs {
        match pair.as_rule() {
            Rule::program => {
                for inner in pair.into_inner() {
                    match inner.as_rule() {
                        Rule::top_level_item => {
                            items.push(parse_top_level_item(inner)?);
                        }
                        Rule::EOI => {}
                        _ => unreachable!("Unexpected rule in program: {:?}", inner.as_rule()),
                    }
                }
            }
            _ => unreachable!("Expected program rule, got {:?}", pair.as_rule()),
        }
    }
    
    Ok(Program { items })
}

fn parse_top_level_item(pair: Pair<Rule>) -> Result<TopLevelItem> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::type_decl => Ok(TopLevelItem::TypeDecl(parse_type_decl(inner)?)),
        Rule::function_decl => Ok(TopLevelItem::FunctionDecl(parse_function_decl(inner)?)),
        Rule::static_decl => Ok(TopLevelItem::StaticDecl(parse_static_decl(inner)?)),
        _ => unreachable!("Unexpected top level item: {:?}", inner.as_rule()),
    }
}

fn parse_type_decl(pair: Pair<Rule>) -> Result<TypeDecl> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    
    let mut generics = Vec::new();
    let mut type_expr = None;
    
    for pair in inner {
        match pair.as_rule() {
            Rule::generic_params => generics = parse_generic_params(pair)?,
            Rule::type_expr => type_expr = Some(parse_type_expr(pair)?),
            _ => {}
        }
    }
    
    Ok(TypeDecl {
        name,
        generics,
        type_expr: type_expr.unwrap(),
    })
}

fn parse_function_decl(pair: Pair<Rule>) -> Result<FunctionDecl> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    
    let mut generics = Vec::new();
    let mut params = Vec::new();
    let mut return_type = None;
    let mut where_clause = Vec::new();
    let mut body = None;
    
    for pair in inner {
        match pair.as_rule() {
            Rule::generic_params => generics = parse_generic_params(pair)?,
            Rule::function_type => {
                let (p, r) = parse_function_type(pair)?;
                params = p;
                return_type = Some(r);
            }
            Rule::where_clause => where_clause = parse_where_clause(pair)?,
            Rule::block => body = Some(parse_block(pair)?),
            _ => {}
        }
    }
    
    Ok(FunctionDecl {
        name,
        generics,
        params,
        return_type: return_type.unwrap(),
        where_clause,
        body,
    })
}

fn parse_static_decl(pair: Pair<Rule>) -> Result<StaticDecl> {
    let mut inner = pair.into_inner();
    inner.next(); // skip "static"
    let name = inner.next().unwrap().as_str().to_string();
    
    let mut generics = Vec::new();
    let mut type_expr = None;
    let mut where_clause = Vec::new();
    let mut value = None;
    
    for pair in inner {
        match pair.as_rule() {
            Rule::generic_params => generics = parse_generic_params(pair)?,
            Rule::type_expr => type_expr = Some(parse_type_expr(pair)?),
            Rule::where_clause => where_clause = parse_where_clause(pair)?,
            Rule::expr => value = Some(parse_expr(pair)?),
            _ => {}
        }
    }
    
    Ok(StaticDecl {
        name,
        generics,
        type_expr: type_expr.unwrap(),
        where_clause,
        value: value.unwrap(),
    })
}

fn parse_generic_params(pair: Pair<Rule>) -> Result<Vec<GenericParam>> {
    let mut params = Vec::new();
    
    for param in pair.into_inner() {
        params.push(parse_generic_param(param)?);
    }
    
    Ok(params)
}

fn parse_generic_param(pair: Pair<Rule>) -> Result<GenericParam> {
    let mut inner = pair.into_inner();
    let first = inner.next().unwrap();
    
    if first.as_rule() == Rule::identifier {
        let name = first.as_str().to_string();
        let constraint = inner.next().map(|p| parse_type_expr(p)).transpose()?;
        Ok(GenericParam::Type { name, constraint })
    } else {
        // static
        let name = inner.next().unwrap().as_str().to_string();
        let type_expr = parse_type_expr(inner.next().unwrap())?;
        Ok(GenericParam::Static { name, type_expr })
    }
}

fn parse_where_clause(pair: Pair<Rule>) -> Result<Vec<Constraint>> {
    let mut constraints = Vec::new();
    
    for constraint in pair.into_inner() {
        if constraint.as_rule() == Rule::constraint {
            constraints.push(parse_constraint(constraint)?);
        }
    }
    
    Ok(constraints)
}

fn parse_constraint(pair: Pair<Rule>) -> Result<Constraint> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let type_expr = parse_type_expr(inner.next().unwrap())?;
    
    Ok(Constraint { name, type_expr })
}

fn parse_type_expr(pair: Pair<Rule>) -> Result<TypeExpr> {
    match pair.as_rule() {
        Rule::type_expr => parse_type_expr(pair.into_inner().next().unwrap()),
        Rule::type_union => {
            let types: Result<Vec<_>> = pair.into_inner()
                .map(parse_type_expr)
                .collect();
            let types = types?;
            if types.len() == 1 {
                Ok(types.into_iter().next().unwrap())
            } else {
                Ok(TypeExpr::Union(types))
            }
        }
        Rule::type_intersection => {
            let types: Result<Vec<_>> = pair.into_inner()
                .map(parse_type_expr)
                .collect();
            let types = types?;
            if types.len() == 1 {
                Ok(types.into_iter().next().unwrap())
            } else {
                Ok(TypeExpr::Intersection(types))
            }
        }
        Rule::type_primary => parse_type_primary(pair),
        _ => unreachable!("Unexpected type expr rule: {:?}", pair.as_rule()),
    }
}

fn parse_type_primary(pair: Pair<Rule>) -> Result<TypeExpr> {
    let rule = pair.as_rule();
    let inner = pair.into_inner().next().unwrap();
    
    match inner.as_rule() {
        Rule::never_type => Ok(TypeExpr::Never),
        Rule::ptr_type => {
            let mut inner = inner.into_inner();
            let mutable = inner.peek().map(|p| p.as_str() == "mut").unwrap_or(false);
            if mutable {
                inner.next();
            }
            inner.next(); // skip "ptr"
            let inner_type = parse_type_expr(inner.next().unwrap())?;
            Ok(TypeExpr::Ptr { mutable, inner: Box::new(inner_type) })
        }
        Rule::array_type => {
            let mut inner = inner.into_inner();
            let first = inner.next().unwrap();
            
            // Check if it's [size type] or just [type]
            if inner.peek().is_some() {
                // Has size
                let size = Some(Box::new(parse_expr(first)?));
                let element = Box::new(parse_type_expr(inner.next().unwrap())?);
                Ok(TypeExpr::Array { size, element })
            } else {
                // No size
                let element = Box::new(parse_type_expr(first)?);
                Ok(TypeExpr::Array { size: None, element })
            }
        }
        Rule::struct_type => {
            let members = inner.into_inner().next().unwrap();
            let (fields, methods) = parse_struct_members(members)?;
            Ok(TypeExpr::Struct { fields, methods })
        }
        Rule::enum_type => {
            let mut inner = inner.into_inner();
            
            // Skip the "enum" keyword
            inner.next(); // This should be enum_kw
            
            // Check for optional backing type (": type_name")
            let mut backing_type = None;
            if let Some(next) = inner.peek() {
                if next.as_rule() == Rule::type_name {
                    backing_type = Some(inner.next().unwrap().into_inner().next().unwrap().as_str().to_string());
                }
            }
            
            let (variants, methods) = if let Some(pair) = inner.next() {
                parse_enum_variants(pair)?
            } else {
                (Vec::new(), Vec::new())
            };
            
            Ok(TypeExpr::Enum { backing_type, variants, methods })
        }
        Rule::function_type => {
            let (params, return_type) = parse_function_type(inner)?;
            Ok(TypeExpr::Function {
                params,
                return_type: Box::new(return_type),
            })
        }
        Rule::type_name => {
            let mut inner = inner.into_inner();
            let name = inner.next().unwrap().as_str().to_string();
            let args = if let Some(generic_args) = inner.next() {
                parse_generic_args(generic_args)?
            } else {
                Vec::new()
            };
            Ok(TypeExpr::Name(name, args))
        }
        _ => unreachable!("Unexpected type primary: {:?}", inner.as_rule()),
    }
}

fn parse_function_type(pair: Pair<Rule>) -> Result<(Vec<Param>, TypeExpr)> {
    let mut inner = pair.into_inner();
    let mut params = Vec::new();
    let mut return_type = None;
    
    for pair in inner {
        match pair.as_rule() {
            Rule::param_list => params = parse_param_list(pair)?,
            Rule::type_expr => return_type = Some(parse_type_expr(pair)?),
            _ => {}
        }
    }
    
    // Default return type is nil
    let return_type = return_type.unwrap_or(TypeExpr::Name("nil".to_string(), vec![]));
    Ok((params, return_type))
}

fn parse_param_list(pair: Pair<Rule>) -> Result<Vec<Param>> {
    let mut params = Vec::new();
    
    for param in pair.into_inner() {
        params.push(parse_param(param)?);
    }
    
    Ok(params)
}

fn parse_param(pair: Pair<Rule>) -> Result<Param> {
    // If the param is just "self" with no inner parts
    if pair.as_str() == "self" {
        return Ok(Param {
            mutable: false,
            name: "self".to_string(),
            type_expr: None,
        });
    }
    
    let mut inner = pair.into_inner();
    
    // Handle empty params (shouldn't happen but let's be safe)
    let first = match inner.next() {
        Some(p) => p,
        None => return Err(anyhow!("Empty parameter")),
    };
    
    if first.as_str() == "self" || first.as_rule() == Rule::mut_kw {
        let mutable = first.as_rule() == Rule::mut_kw;
        // Check if next element is "self"
        if mutable {
            // After mut, we expect "self"
            let next = inner.next();
            if next.is_none() || next.unwrap().as_str() != "self" {
                return Err(anyhow!("Expected 'self' after 'mut'"));
            }
        }
        Ok(Param {
            mutable,
            name: "self".to_string(),
            type_expr: None,
        })
    } else {
        let mut mutable = false;
        let mut name = first.as_str().to_string();
        
        if name == "mut" {
            mutable = true;
            name = inner.next().unwrap().as_str().to_string();
        }
        
        let type_expr = Some(parse_type_expr(inner.next().unwrap())?);
        
        Ok(Param { mutable, name, type_expr })
    }
}

fn parse_generic_args(pair: Pair<Rule>) -> Result<Vec<GenericArg>> {
    let mut args = Vec::new();
    
    for arg in pair.into_inner() {
        args.push(parse_generic_arg(arg)?);
    }
    
    Ok(args)
}

fn parse_generic_arg(pair: Pair<Rule>) -> Result<GenericArg> {
    let inner = pair.into_inner().next().unwrap();
    
    match inner.as_rule() {
        Rule::type_expr => Ok(GenericArg::Type(parse_type_expr(inner)?)),
        Rule::expr => Ok(GenericArg::Value(parse_expr(inner)?)),
        _ => unreachable!("Unexpected generic arg: {:?}", inner.as_rule()),
    }
}

fn parse_struct_members(pair: Pair<Rule>) -> Result<(Vec<StructField>, Vec<MethodDecl>)> {
    let mut fields = Vec::new();
    let mut methods = Vec::new();
    
    for member in pair.into_inner() {
        match member.as_rule() {
            Rule::struct_field => {
                let mut inner = member.into_inner();
                let name = inner.next().unwrap().as_str().to_string();
                let type_expr = parse_type_expr(inner.next().unwrap())?;
                fields.push(StructField { name, type_expr });
            }
            Rule::method_decl => methods.push(parse_method_decl(member)?),
            _ => {}
        }
    }
    
    Ok((fields, methods))
}

fn parse_method_decl(pair: Pair<Rule>) -> Result<MethodDecl> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    
    let mut generics = Vec::new();
    let mut params = Vec::new();
    let mut return_type = TypeExpr::Name("void".to_string(), Vec::new());
    let mut where_clause = Vec::new();
    let mut body = None;
    
    for pair in inner {
        match pair.as_rule() {
            Rule::generic_params => generics = parse_generic_params(pair)?,
            Rule::function_type => {
                let (p, r) = parse_function_type(pair)?;
                params = p;
                return_type = r;
            }
            Rule::where_clause => where_clause = parse_where_clause(pair)?,
            Rule::block => body = Some(parse_block(pair)?),
            _ => {}
        }
    }
    
    Ok(MethodDecl {
        name,
        generics,
        params,
        return_type,
        where_clause,
        body: body.unwrap(),
    })
}

fn parse_block(pair: Pair<Rule>) -> Result<Block> {
    let mut stmts = Vec::new();
    let mut expr = None;
    
    let mut inner = pair.into_inner().peekable();
    
    while let Some(pair) = inner.next() {
        match pair.as_rule() {
            Rule::stmt => stmts.push(parse_stmt(pair)?),
            Rule::expr => {
                // If this is the last item, it's the block expression
                if inner.peek().is_none() {
                    expr = Some(Box::new(parse_expr(pair)?));
                } else {
                    // Otherwise, treat it as an expression statement
                    stmts.push(Stmt::Expr(parse_expr(pair)?));
                }
            }
            _ => {}
        }
    }
    
    Ok(Block { stmts, expr })
}

fn parse_enum_variants(pair: Pair<Rule>) -> Result<(Vec<EnumVariant>, Vec<MethodDecl>)> {
    let mut variants = Vec::new();
    let mut methods = Vec::new();
    
    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::enum_variant => variants.push(parse_enum_variant(item)?),
            Rule::method_decl => methods.push(parse_method_decl(item)?),
            _ => {}
        }
    }
    
    Ok((variants, methods))
}

fn parse_enum_variant(pair: Pair<Rule>) -> Result<EnumVariant> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    
    let data = if let Some(next) = inner.next() {
        match next.as_rule() {
            Rule::type_expr => EnumVariantData::Type(parse_type_expr(next)?),
            Rule::expr => {
                let expr = parse_expr(next)?;
                if let Expr::Literal(Literal::Int(val)) = expr {
                    EnumVariantData::Value(val)
                } else {
                    return Err(anyhow!("Enum variant value must be an integer literal"));
                }
            }
            _ => EnumVariantData::Unit,
        }
    } else {
        EnumVariantData::Unit
    };
    
    Ok(EnumVariant { name, data })
}

fn parse_stmt(pair: Pair<Rule>) -> Result<Stmt> {
    let inner = pair.into_inner().next().unwrap();
    
    match inner.as_rule() {
        Rule::expr_stmt => {
            let expr = parse_expr(inner.into_inner().next().unwrap())?;
            Ok(Stmt::Expr(expr))
        }
        Rule::decl_stmt => {
            let mut inner = inner.into_inner();
            let first = inner.next().unwrap();
            
            let (mutable, name) = if first.as_str() == "mut" {
                (true, inner.next().unwrap().as_str().to_string())
            } else {
                (false, first.as_str().to_string())
            };
            
            let type_expr = parse_type_expr(inner.next().unwrap())?;
            let value = parse_expr(inner.next().unwrap())?;
            
            Ok(Stmt::Decl { mutable, name, type_expr, value })
        }
        Rule::block_stmt => Ok(Stmt::Block(parse_block(inner.into_inner().next().unwrap())?)),
        Rule::if_stmt => {
            let mut inner = inner.into_inner();
            inner.next(); // skip "if" keyword
            let condition = parse_expr(inner.next().unwrap())?;
            let then_block = parse_block(inner.next().unwrap())?;
            
            let else_part = if inner.next().is_some() { // skip "else" keyword
                inner.next().map(|p| {
                    match p.as_rule() {
                        Rule::if_stmt => {
                            // Parse the if_stmt as a stmt
                            Box::new(parse_stmt(p).unwrap())
                        }
                        Rule::block => Box::new(Stmt::Block(parse_block(p).unwrap())),
                        _ => unreachable!(),
                    }
                })
            } else {
                None
            };
            
            Ok(Stmt::If { condition, then_block, else_part })
        }
        Rule::while_stmt => {
            let mut inner = inner.into_inner();
            let first = inner.next().unwrap();
            
            let (label, condition, body) = if first.as_rule() == Rule::label {
                let label = Some(first.as_str().to_string());
                inner.next(); // skip "while" keyword
                let condition = parse_expr(inner.next().unwrap())?;
                let body = parse_block(inner.next().unwrap())?;
                (label, condition, body)
            } else {
                // first is "while" keyword, skip it
                let condition = parse_expr(inner.next().unwrap())?;
                let body = parse_block(inner.next().unwrap())?;
                (None, condition, body)
            };
            
            Ok(Stmt::While { label, condition, body })
        }
        Rule::for_stmt => {
            let mut inner = inner.into_inner();
            let first = inner.next().unwrap();
            
            let (label, kind, body) = if first.as_rule() == Rule::label {
                let label = Some(first.as_str().to_string());
                let kind_pair = inner.next().unwrap();
                let (kind, body) = parse_for_kind_with_body(kind_pair)?;
                (label, kind, body)
            } else {
                let (kind, body) = parse_for_kind_with_body(first)?;
                (None, kind, body)
            };
            
            Ok(Stmt::For { label, kind, body })
        }
        Rule::match_stmt => {
            let mut inner = inner.into_inner();
            let is_type = inner.as_str().contains("type");
            let expr = parse_expr(inner.find(|p| p.as_rule() == Rule::expr).unwrap())?;
            let arms = parse_match_arms(inner.find(|p| p.as_rule() == Rule::match_arms).unwrap())?;
            
            Ok(Stmt::Match { is_type, expr, arms })
        }
        Rule::return_stmt => {
            let mut inner = inner.into_inner();
            inner.next(); // skip "return"
            let expr = inner.next().map(parse_expr).transpose()?;
            Ok(Stmt::Return(expr))
        }
        Rule::break_stmt => {
            let mut inner = inner.into_inner();
            inner.next(); // skip "break"
            let label = inner.next().map(|p| p.as_str().to_string());
            Ok(Stmt::Break(label))
        }
        Rule::continue_stmt => {
            let mut inner = inner.into_inner();
            inner.next(); // skip "continue"
            let label = inner.next().map(|p| p.as_str().to_string());
            Ok(Stmt::Continue(label))
        }
        Rule::labeled_stmt => {
            let mut inner = inner.into_inner();
            let label = inner.next().unwrap().as_str().to_string();
            let stmt = Box::new(parse_stmt(inner.next().unwrap())?);
            Ok(Stmt::Labeled { label, stmt })
        }
        _ => unreachable!("Unexpected stmt: {:?}", inner.as_rule()),
    }
}

fn parse_for_kind_with_body(pair: Pair<Rule>) -> Result<(ForKind, Block)> {
    match pair.as_rule() {
        Rule::for_in_stmt => {
            let mut inner = pair.into_inner();
            
            // Skip for_kw
            inner.next(); // Skip "for"
            
            let first = inner.next().unwrap();
            let (mutable, var) = if first.as_rule() == Rule::mut_kw {
                (true, inner.next().unwrap().as_str().to_string())
            } else {
                (false, first.as_str().to_string())
            };
            
            // Skip the "in" keyword
            inner.next(); // This should be in_kw
            
            let iter = parse_expr(inner.next().unwrap())?;
            let body = parse_block(inner.next().unwrap())?;
            
            Ok((ForKind::In { mutable, var, iter }, body))
        }
        Rule::for_c_stmt => {
            let mut inner = pair.into_inner();
            // Don't skip "for" - it's not included as a separate token
            
            let mut init = None;
            let mut condition = None;
            let mut update = None;
            let mut body = None;
            
            for pair in inner {
                match pair.as_rule() {
                    Rule::for_init => init = Some(Box::new(parse_for_init(pair)?)),
                    Rule::expr => {
                        if condition.is_none() {
                            condition = Some(parse_expr(pair)?);
                        } else {
                            update = Some(parse_expr(pair)?);
                        }
                    }
                    Rule::block => body = Some(parse_block(pair)?),
                    _ => {}
                }
            }
            
            Ok((ForKind::C { init, condition, update }, body.unwrap()))
        }
        _ => unreachable!("Unexpected for kind: {:?}", pair.as_rule()),
    }
}

fn parse_for_init(pair: Pair<Rule>) -> Result<ForInit> {
    let mut inner = pair.into_inner();
    let first = inner.next().unwrap();
    
    let (mutable, name) = if first.as_str() == "mut" {
        (true, inner.next().unwrap().as_str().to_string())
    } else {
        (false, first.as_str().to_string())
    };
    
    let type_expr = parse_type_expr(inner.next().unwrap())?;
    let value = parse_expr(inner.next().unwrap())?;
    
    Ok(ForInit { mutable, name, type_expr, value })
}

fn parse_match_arms(pair: Pair<Rule>) -> Result<Vec<MatchArm>> {
    let mut arms = Vec::new();
    
    for arm in pair.into_inner() {
        if arm.as_rule() == Rule::match_arm {
            arms.push(parse_match_arm(arm)?);
        }
    }
    
    Ok(arms)
}

fn parse_match_arm(pair: Pair<Rule>) -> Result<MatchArm> {
    let mut inner = pair.into_inner();
    let pattern = parse_pattern(inner.next().unwrap())?;
    
    let body_pair = inner.next().unwrap();
    let body = match body_pair.as_rule() {
        Rule::block => MatchArmBody::Block(parse_block(body_pair)?),
        Rule::expr => MatchArmBody::Expr(parse_expr(body_pair)?),
        _ => unreachable!("Unexpected match arm body: {:?}", body_pair.as_rule()),
    };
    
    Ok(MatchArm { pattern, body })
}

fn parse_pattern(pair: Pair<Rule>) -> Result<Pattern> {
    let inner = pair.into_inner().next().unwrap();
    
    match inner.as_rule() {
        Rule::identifier => Ok(Pattern::Identifier(inner.as_str().to_string())),
        Rule::type_pattern => {
            let mut inner = inner.into_inner();
            let type_expr = parse_type_expr(inner.next().unwrap())?;
            let field = inner.next().unwrap().as_str().to_string();
            Ok(Pattern::Type(type_expr, field))
        }
        Rule::literal_pattern => {
            let literal = parse_literal(inner.into_inner().next().unwrap())?;
            Ok(Pattern::Literal(literal))
        }
        _ if inner.as_str() == "_" => Ok(Pattern::Wildcard),
        _ => unreachable!("Unexpected pattern: {:?}", inner.as_rule()),
    }
}

fn parse_expr(pair: Pair<Rule>) -> Result<Expr> {
    match pair.as_rule() {
        Rule::expr => parse_expr(pair.into_inner().next().unwrap()),
        Rule::cast_expr => parse_cast_expr(pair),
        Rule::assignment_expr => parse_assignment_expr(pair),
        Rule::logical_or_expr => parse_binary_expr(pair, &[BinaryOp::Or]),
        Rule::logical_and_expr => parse_binary_expr(pair, &[BinaryOp::And]),
        Rule::bitwise_or_expr => parse_binary_expr(pair, &[BinaryOp::BitOr]),
        Rule::bitwise_xor_expr => parse_binary_expr(pair, &[BinaryOp::BitXor]),
        Rule::bitwise_and_expr => parse_binary_expr(pair, &[BinaryOp::BitAnd]),
        Rule::equality_expr => parse_binary_expr(pair, &[BinaryOp::Eq, BinaryOp::Ne]),
        Rule::relational_expr => parse_binary_expr(pair, &[BinaryOp::Lt, BinaryOp::Le, BinaryOp::Gt, BinaryOp::Ge]),
        Rule::shift_expr => parse_binary_expr(pair, &[BinaryOp::Shl, BinaryOp::Shr]),
        Rule::additive_expr => parse_binary_expr(pair, &[BinaryOp::Add, BinaryOp::Sub]),
        Rule::multiplicative_expr => parse_binary_expr(pair, &[BinaryOp::Mul, BinaryOp::Div, BinaryOp::Mod]),
        Rule::unary_expr => parse_unary_expr(pair),
        Rule::postfix_expr => parse_postfix_expr(pair),
        Rule::primary_expr => parse_primary_expr(pair),
        _ => unreachable!("Unexpected expr rule: {:?}", pair.as_rule()),
    }
}

fn parse_cast_expr(pair: Pair<Rule>) -> Result<Expr> {
    let mut inner = pair.into_inner();
    let expr = parse_expr(inner.next().unwrap())?;
    
    if let Some(_as_kw) = inner.next() {
        // Skip the "as" keyword
        let type_expr = parse_type_expr(inner.next().unwrap())?;
        Ok(Expr::Cast {
            expr: Box::new(expr),
            type_expr,
        })
    } else {
        Ok(expr)
    }
}

fn parse_assignment_expr(pair: Pair<Rule>) -> Result<Expr> {
    let mut inner = pair.into_inner();
    let left = parse_expr(inner.next().unwrap())?;
    
    if let Some(op_pair) = inner.next() {
        let op = match op_pair.as_str() {
            "=" => BinaryOp::Assign,
            "+=" => BinaryOp::AddAssign,
            "-=" => BinaryOp::SubAssign,
            "*=" => BinaryOp::MulAssign,
            "/=" => BinaryOp::DivAssign,
            "%=" => BinaryOp::ModAssign,
            "&=" => BinaryOp::BitAndAssign,
            "|=" => BinaryOp::BitOrAssign,
            "^=" => BinaryOp::BitXorAssign,
            "<<=" => BinaryOp::ShlAssign,
            ">>=" => BinaryOp::ShrAssign,
            _ => unreachable!("Unknown assignment op: {}", op_pair.as_str()),
        };
        
        let right = parse_expr(inner.next().unwrap())?;
        Ok(Expr::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        })
    } else {
        Ok(left)
    }
}

fn parse_binary_expr(pair: Pair<Rule>, ops: &[BinaryOp]) -> Result<Expr> {
    let span = pair.as_span();
    let text = span.as_str();
    let mut inner = pair.into_inner();
    let mut left = parse_expr(inner.next().unwrap())?;
    
    // For binary expressions, the grammar produces pairs like:
    // left_expr, right_expr, right_expr, ...
    // The operator is between each pair
    while let Some(right_pair) = inner.next() {
        let right = parse_expr(right_pair)?;
        
        // Determine which operator was used by looking at the text
        let op = determine_operator(text, ops)?;
        
        left = Expr::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        };
    }
    
    Ok(left)
}

// Helper function to determine which operator was used
fn determine_operator(text: &str, ops: &[BinaryOp]) -> Result<BinaryOp> {
    use BinaryOp::*;
    
    // Check each possible operator
    for &op in ops {
        let op_str = match op {
            Add => "+",
            Sub => "-",
            Mul => "*",
            Div => "/",
            Mod => "%",
            BitAnd => "&",
            BitOr => "|",
            BitXor => "^",
            Shl => "<<",
            Shr => ">>",
            Eq => "==",
            Ne => "!=",
            Le => "<=",
            Ge => ">=",
            Lt => "<",
            Gt => ">",
            And => "&&",
            Or => "||",
            _ => continue,
        };
        
        // Check if this operator appears in the text
        if text.contains(op_str) {
            // Make sure it's not part of a larger operator
            match op {
                Lt => {
                    // Make sure it's not << or <=
                    if !text.contains("<<") && !text.contains("<=") {
                        return Ok(op);
                    }
                }
                Gt => {
                    // Make sure it's not >> or >=
                    if !text.contains(">>") && !text.contains(">=") {
                        return Ok(op);
                    }
                }
                BitAnd => {
                    // Make sure it's not &&
                    let count_single = text.matches('&').count();
                    let count_double = text.matches("&&").count();
                    if count_single > count_double * 2 {
                        return Ok(op);
                    }
                }
                BitOr => {
                    // Make sure it's not ||
                    let count_single = text.matches('|').count();
                    let count_double = text.matches("||").count();
                    if count_single > count_double * 2 {
                        return Ok(op);
                    }
                }
                _ => return Ok(op),
            }
        }
    }
    
    // If we can't find an operator, default to the first one (shouldn't happen)
    Ok(ops[0])
}

fn parse_unary_expr(pair: Pair<Rule>) -> Result<Expr> {
    let mut inner = pair.into_inner();
    let first = inner.next().unwrap();
    
    if first.as_rule() == Rule::unary_op {
        let op = match first.as_str() {
            "+" => UnaryOp::Plus,
            "-" => UnaryOp::Minus,
            "~" => UnaryOp::BitNot,
            "!" => UnaryOp::Not,
            "*" => UnaryOp::Deref,
            "^" => UnaryOp::AddrOf,
            _ => unreachable!("Unknown unary op: {}", first.as_str()),
        };
        
        let expr = parse_expr(inner.next().unwrap())?;
        Ok(Expr::Unary {
            op,
            expr: Box::new(expr),
        })
    } else {
        parse_expr(first)
    }
}

fn parse_postfix_expr(pair: Pair<Rule>) -> Result<Expr> {
    let mut inner = pair.into_inner();
    let mut expr = parse_expr(inner.next().unwrap())?;
    
    for op in inner {
        expr = match op.as_rule() {
            Rule::postfix_op => {
                // postfix_op is the actual operation, not a wrapper
                // Check what's inside this specific postfix_op
                let op_str = op.as_str();
                if op_str.starts_with('[') {
                    // Array index: [expr]
                    let index_expr = op.into_inner().next().unwrap();
                    Expr::Index {
                        array: Box::new(expr),
                        index: Box::new(parse_expr(index_expr)?),
                    }
                } else if op_str.starts_with('(') {
                    // Function call: (expr_list?)
                    if let Some(expr_list) = op.into_inner().next() {
                        Expr::Call {
                            func: Box::new(expr),
                            args: parse_expr_list(expr_list)?,
                        }
                    } else {
                        // Empty call
                        Expr::Call {
                            func: Box::new(expr),
                            args: vec![],
                        }
                    }
                } else if op_str.starts_with('.') {
                    // Field access: .identifier
                    let field_name = op.into_inner().next().unwrap().as_str().to_string();
                    Expr::Field {
                        expr: Box::new(expr),
                        field: field_name,
                    }
                } else {
                    // Generic args - ignore for now
                    expr
                }
            }
            _ => unreachable!("Expected postfix_op, got {:?}", op.as_rule()),
        };
    }
    
    Ok(expr)
}

fn parse_primary_expr(pair: Pair<Rule>) -> Result<Expr> {
    let inner = pair.into_inner().next().unwrap();
    
    match inner.as_rule() {
        Rule::identifier => Ok(Expr::Identifier(inner.as_str().to_string())),
        Rule::int_literal => Ok(Expr::Literal(parse_int_literal(inner)?)),
        Rule::float_literal => Ok(Expr::Literal(parse_float_literal(inner)?)),
        Rule::bool_literal => Ok(Expr::Literal(parse_bool_literal(inner)?)),
        Rule::string_literal => Ok(Expr::Literal(parse_string_literal(inner)?)),
        Rule::sizeof_expr => parse_sizeof_expr(inner),
        Rule::brace_literal => parse_brace_literal(inner),
        Rule::function_literal => parse_function_literal(inner),
        Rule::if_expr => parse_if_expr(inner),
        Rule::match_expr => parse_match_expr(inner),
        Rule::expr => parse_expr(inner),
        _ if inner.as_str() == "nil" => Ok(Expr::Literal(Literal::Nil)),
        _ if inner.as_str() == "self" => Ok(Expr::Identifier("self".to_string())),
        _ => unreachable!("Unexpected primary expr: {:?}", inner.as_rule()),
    }
}

fn parse_expr_list(pair: Pair<Rule>) -> Result<Vec<Expr>> {
    let mut exprs = Vec::new();
    
    for expr in pair.into_inner() {
        if expr.as_rule() == Rule::expr {
            exprs.push(parse_expr(expr)?);
        }
    }
    
    Ok(exprs)
}

fn parse_sizeof_expr(pair: Pair<Rule>) -> Result<Expr> {
    let inner = pair.into_inner().skip(1).next().unwrap(); // skip "sizeof"
    
    let operand = match inner.as_rule() {
        Rule::type_expr => SizeofOperand::Type(parse_type_expr(inner)?),
        Rule::expr => SizeofOperand::Expr(Box::new(parse_expr(inner)?)),
        _ => unreachable!("Unexpected sizeof operand: {:?}", inner.as_rule()),
    };
    
    Ok(Expr::Sizeof(operand))
}

fn parse_brace_literal(pair: Pair<Rule>) -> Result<Expr> {
    if let Some(content) = pair.into_inner().next() {
        match content.as_rule() {
            Rule::brace_content => {
                // Need to determine if it's field_init_list or expr_list
                if let Some(inner) = content.into_inner().next() {
                    match inner.as_rule() {
                        Rule::field_init_list => {
                            // It's a struct literal
                            let mut fields = Vec::new();
                            for field in inner.into_inner() {
                                if field.as_rule() == Rule::field_init {
                                    let mut field_inner = field.into_inner();
                                    let name = field_inner.next().unwrap().as_str().to_string();
                                    let expr = parse_expr(field_inner.next().unwrap())?;
                                    fields.push((name, expr));
                                }
                            }
                            Ok(Expr::StructLiteral { fields })
                        }
                        Rule::expr_list => {
                            // It's an array literal
                            let elements = parse_expr_list(inner)?;
                            Ok(Expr::ArrayLiteral(elements))
                        }
                        _ => unreachable!("Unexpected brace content: {:?}", inner.as_rule()),
                    }
                } else {
                    // Empty braces - could be empty array or struct
                    // Default to empty array
                    Ok(Expr::ArrayLiteral(vec![]))
                }
            }
            _ => unreachable!("Unexpected content in brace literal: {:?}", content.as_rule()),
        }
    } else {
        // Empty braces
        Ok(Expr::ArrayLiteral(vec![]))
    }
}

fn parse_function_literal(pair: Pair<Rule>) -> Result<Expr> {
    let mut inner = pair.into_inner();
    inner.next(); // skip "|"
    
    let mut params = Vec::new();
    let mut return_type = None;
    let mut body = None;
    
    for pair in inner {
        match pair.as_rule() {
            Rule::param_list => params = parse_param_list(pair)?,
            Rule::type_expr => return_type = Some(parse_type_expr(pair)?),
            Rule::block => body = Some(parse_block(pair)?),
            _ => {}
        }
    }
    
    Ok(Expr::FunctionLiteral {
        params,
        return_type: return_type.unwrap(),
        body: body.unwrap(),
    })
}

fn parse_if_expr(pair: Pair<Rule>) -> Result<Expr> {
    let mut inner = pair.into_inner();
    inner.next(); // skip "if"
    
    let condition = Box::new(parse_expr(inner.next().unwrap())?);
    let then_block = parse_block(inner.next().unwrap())?;
    let then_expr = Box::new(block_to_expr(then_block));
    
    inner.next(); // skip "else"
    let else_pair = inner.next().unwrap();
    let else_expr = Box::new(match else_pair.as_rule() {
        Rule::if_expr => parse_if_expr(else_pair)?,
        Rule::block => block_to_expr(parse_block(else_pair)?),
        _ => unreachable!(),
    });
    
    Ok(Expr::If { condition, then_expr, else_expr })
}

fn parse_match_expr(pair: Pair<Rule>) -> Result<Expr> {
    let mut inner = pair.into_inner();
    inner.next(); // skip "match"
    
    let is_type = inner.as_str().contains("type");
    let expr = Box::new(parse_expr(inner.find(|p| p.as_rule() == Rule::expr).unwrap())?);
    let arms = parse_match_arms(inner.find(|p| p.as_rule() == Rule::match_arms).unwrap())?;
    
    Ok(Expr::Match { is_type, expr, arms })
}

fn parse_literal(pair: Pair<Rule>) -> Result<Literal> {
    match pair.as_rule() {
        Rule::int_literal => parse_int_literal(pair),
        Rule::float_literal => parse_float_literal(pair),
        Rule::bool_literal => parse_bool_literal(pair),
        Rule::string_literal => parse_string_literal(pair),
        _ => unreachable!("Unexpected literal: {:?}", pair.as_rule()),
    }
}

fn parse_int_literal(pair: Pair<Rule>) -> Result<Literal> {
    let s = pair.as_str();
    let val = if s.starts_with("0x") {
        i64::from_str_radix(&s[2..], 16)?
    } else if s.starts_with("0b") {
        i64::from_str_radix(&s[2..], 2)?
    } else {
        s.parse()?
    };
    Ok(Literal::Int(val))
}

fn parse_float_literal(pair: Pair<Rule>) -> Result<Literal> {
    Ok(Literal::Float(pair.as_str().parse()?))
}

fn parse_bool_literal(pair: Pair<Rule>) -> Result<Literal> {
    Ok(Literal::Bool(pair.as_str() == "true"))
}

fn parse_string_literal(pair: Pair<Rule>) -> Result<Literal> {
    let s = pair.as_str();
    let s = &s[1..s.len()-1]; // Remove quotes
    
    // TODO: Handle escape sequences properly
    Ok(Literal::String(s.to_string()))
}

// Helper to convert a block to an expression
fn block_to_expr(block: Block) -> Expr {
    // If the block has a final expression, return it directly
    if let Some(expr) = block.expr {
        return *expr;
    }
    
    // If the block is just statements with no expression, 
    // it evaluates to nil
    if block.stmts.is_empty() {
        return Expr::Literal(Literal::Nil);
    }
    
    // Otherwise, use a match expression as a workaround for statement blocks
    Expr::Match {
        is_type: false,
        expr: Box::new(Expr::Literal(Literal::Int(0))),
        arms: vec![MatchArm {
            pattern: Pattern::Wildcard,
            body: MatchArmBody::Block(block),
        }],
    }
}