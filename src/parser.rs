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
    let inner = pair.into_inner().next().unwrap();
    
    match inner.as_rule() {
        Rule::never_type => Ok(TypeExpr::Never),
        Rule::ptr_type => {
            let mut inner = inner.into_inner();
            let mutable = inner.as_str().contains("mut");
            let inner_type = inner.find(|p| p.as_rule() == Rule::type_expr).unwrap();
            Ok(TypeExpr::Ptr {
                mutable,
                inner: Box::new(parse_type_expr(inner_type)?),
            })
        }
        Rule::array_type => {
            let mut inner = inner.into_inner();
            let first = inner.next().unwrap();
            
            if first.as_rule() == Rule::expr {
                let size = parse_expr(first)?;
                let element = parse_type_expr(inner.next().unwrap())?;
                Ok(TypeExpr::Array {
                    size: Some(Box::new(size)),
                    element: Box::new(element),
                })
            } else {
                let element = parse_type_expr(first)?;
                Ok(TypeExpr::Array {
                    size: None,
                    element: Box::new(element),
                })
            }
        }
        Rule::struct_type => {
            let members = inner.into_inner().next().unwrap();
            let (fields, methods) = parse_struct_members(members)?;
            Ok(TypeExpr::Struct { fields, methods })
        }
        Rule::enum_type => {
            let mut inner = inner.into_inner();
            let mut backing_type = None;
            let mut variants = Vec::new();
            
            for pair in inner {
                match pair.as_rule() {
                    Rule::type_name => backing_type = Some(pair.as_str().to_string()),
                    Rule::enum_variants => variants = parse_enum_variants(pair)?,
                    _ => {}
                }
            }
            
            Ok(TypeExpr::Enum { backing_type, variants })
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
            let args = inner.next()
                .map(|p| parse_generic_args(p))
                .transpose()?
                .unwrap_or_default();
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
    
    Ok((params, return_type.unwrap()))
}

fn parse_param_list(pair: Pair<Rule>) -> Result<Vec<Param>> {
    let mut params = Vec::new();
    
    for param in pair.into_inner() {
        params.push(parse_param(param)?);
    }
    
    Ok(params)
}

fn parse_param(pair: Pair<Rule>) -> Result<Param> {
    let mut inner = pair.into_inner();
    let first = inner.next().unwrap();
    
    if first.as_str() == "self" || first.as_str() == "mut" {
        let mutable = first.as_str() == "mut";
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
    
    Ok(MethodDecl {
        name,
        generics,
        params,
        return_type: return_type.unwrap(),
        where_clause,
        body: body.unwrap(),
    })
}

fn parse_enum_variants(pair: Pair<Rule>) -> Result<Vec<EnumVariant>> {
    let mut variants = Vec::new();
    
    for variant in pair.into_inner() {
        if variant.as_rule() == Rule::enum_variant {
            variants.push(parse_enum_variant(variant)?);
        }
    }
    
    Ok(variants)
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

fn parse_block(pair: Pair<Rule>) -> Result<Block> {
    let mut stmts = Vec::new();
    
    for stmt in pair.into_inner() {
        if stmt.as_rule() == Rule::stmt {
            stmts.push(parse_stmt(stmt)?);
        }
    }
    
    Ok(Block { stmts })
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
            let condition = parse_expr(inner.next().unwrap())?;
            let then_block = parse_block(inner.next().unwrap())?;
            
            let else_part = inner.next().map(|p| {
                match p.as_rule() {
                    Rule::if_stmt => {
                        // Parse the if_stmt directly
                        let mut if_inner = p.into_inner();
                        let cond = parse_expr(if_inner.next().unwrap()).unwrap();
                        let then_b = parse_block(if_inner.next().unwrap()).unwrap();
                        let else_p = if_inner.next().map(|ep| {
                            Box::new(Stmt::Block(parse_block(ep).unwrap()))
                        });
                        Box::new(Stmt::If { condition: cond, then_block: then_b, else_part: else_p })
                    }
                    Rule::block => Box::new(Stmt::Block(parse_block(p).unwrap())),
                    _ => unreachable!(),
                }
            });
            
            Ok(Stmt::If { condition, then_block, else_part })
        }
        Rule::while_stmt => {
            let mut inner = inner.into_inner();
            let first = inner.next().unwrap();
            
            let (label, condition, body) = if first.as_rule() == Rule::label {
                let label = Some(first.as_str().to_string());
                let condition = parse_expr(inner.next().unwrap())?;
                let body = parse_block(inner.next().unwrap())?;
                (label, condition, body)
            } else {
                let condition = parse_expr(first)?;
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
                let body = parse_block(inner.next().unwrap())?;
                (label, parse_for_kind(kind_pair)?, body)
            } else {
                let body = parse_block(inner.next().unwrap())?;
                (None, parse_for_kind(first)?, body)
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

fn parse_for_kind(pair: Pair<Rule>) -> Result<ForKind> {
    match pair.as_rule() {
        Rule::for_in_stmt => {
            let mut inner = pair.into_inner();
            inner.next(); // skip "for"
            
            let first = inner.next().unwrap();
            let (mutable, var) = if first.as_str() == "mut" {
                (true, inner.next().unwrap().as_str().to_string())
            } else {
                (false, first.as_str().to_string())
            };
            
            let iter = parse_expr(inner.next().unwrap())?;
            
            Ok(ForKind::In { mutable, var, iter })
        }
        Rule::for_c_stmt => {
            let mut inner = pair.into_inner();
            inner.next(); // skip "for"
            
            let mut init = None;
            let mut condition = None;
            let mut update = None;
            
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
                    _ => {}
                }
            }
            
            Ok(ForKind::C { init, condition, update })
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
    let mut inner = pair.into_inner();
    let mut left = parse_expr(inner.next().unwrap())?;
    
    // For binary expressions, the grammar produces pairs like:
    // left_expr, right_expr, right_expr, ...
    // The operator is implicit in the grammar rule and position
    let mut operands = vec![left];
    while let Some(right) = inner.next() {
        operands.push(parse_expr(right)?);
    }
    
    // If we only have one operand, return it
    if operands.len() == 1 {
        return Ok(operands.into_iter().next().unwrap());
    }
    
    // Build the expression left-to-right with the appropriate operator
    // Since all operators in ops[] have the same precedence and are left-associative
    let mut expr = operands[0].clone();
    for i in 1..operands.len() {
        // Determine which operator to use based on the rule and position
        // For now, use the first operator in the list (they should all be the same for a given rule)
        let op = ops[0];
        expr = Expr::Binary {
            op,
            left: Box::new(expr),
            right: Box::new(operands[i].clone()),
        };
    }
    
    Ok(expr)
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
                let op_inner = op.into_inner().next().unwrap();
                match op_inner.as_rule() {
                    Rule::expr => Expr::Index {
                        array: Box::new(expr),
                        index: Box::new(parse_expr(op_inner)?),
                    },
                    Rule::expr_list => Expr::Call {
                        func: Box::new(expr),
                        args: parse_expr_list(op_inner)?,
                    },
                    Rule::identifier => Expr::Field {
                        expr: Box::new(expr),
                        field: op_inner.as_str().to_string(),
                    },
                    Rule::generic_args => {
                        // Handle generic instantiation
                        // For now, we'll ignore this in expressions
                        expr
                    }
                    _ => unreachable!("Unexpected postfix op: {:?}", op_inner.as_rule()),
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
        Rule::struct_literal => parse_struct_literal(inner),
        Rule::function_literal => parse_function_literal(inner),
        Rule::if_expr => parse_if_expr(inner),
        Rule::match_expr => parse_match_expr(inner),
        Rule::expr => parse_expr(inner),
        _ if inner.as_str() == "nil" => Ok(Expr::Literal(Literal::Nil)),
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

fn parse_struct_literal(pair: Pair<Rule>) -> Result<Expr> {
    let mut fields = Vec::new();
    
    if let Some(field_list) = pair.into_inner().next() {
        for field in field_list.into_inner() {
            if field.as_rule() == Rule::field_init {
                let mut inner = field.into_inner();
                let name = inner.next().unwrap().as_str().to_string();
                let expr = parse_expr(inner.next().unwrap())?;
                fields.push((name, expr));
            }
        }
    }
    
    Ok(Expr::StructLiteral { fields })
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
    // Use a match expression as a workaround for block expressions
    Expr::Match {
        is_type: false,
        expr: Box::new(Expr::Literal(Literal::Int(0))),
        arms: vec![MatchArm {
            pattern: Pattern::Wildcard,
            body: MatchArmBody::Block(block),
        }],
    }
}