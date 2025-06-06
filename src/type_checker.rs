use anyhow::{anyhow, Result};
use std::collections::HashMap;
use indexmap::IndexMap;

use crate::ast::*;

pub fn check_program(program: Program) -> Result<Program> {
    let mut checker = TypeChecker::new();
    checker.check_program(program)
}

struct TypeChecker {
    // Global type definitions
    types: HashMap<String, TypeDef>,
    // Global function signatures
    functions: HashMap<String, FunctionSig>,
    // Global static values
    statics: HashMap<String, TypeExpr>,
    // Stack of variable scopes
    scopes: Vec<HashMap<String, VarInfo>>,
    // Current function return type (if any)
    current_return_type: Option<TypeExpr>,
    // Loop labels in scope
    loop_labels: Vec<Option<String>>,
}

#[derive(Clone)]
struct TypeDef {
    generics: Vec<GenericParam>,
    type_expr: TypeExpr,
}

#[derive(Clone)]
struct FunctionSig {
    generics: Vec<GenericParam>,
    params: Vec<Param>,
    return_type: TypeExpr,
    where_clause: Vec<Constraint>,
}

#[derive(Clone)]
struct VarInfo {
    type_expr: TypeExpr,
    mutable: bool,
}

impl TypeChecker {
    fn new() -> Self {
        let mut checker = TypeChecker {
            types: HashMap::new(),
            functions: HashMap::new(),
            statics: HashMap::new(),
            scopes: vec![HashMap::new()],
            current_return_type: None,
            loop_labels: Vec::new(),
        };
        
        // Add predefined types
        checker.add_predefined_types();
        
        checker
    }
    
    fn add_predefined_types(&mut self) {
        // Numeric types
        let numeric_types = vec![
            "u8", "u16", "u32", "u64",
            "i8", "i16", "i32", "i64",
            "int", "uint", "float"
        ];
        
        for name in numeric_types {
            self.types.insert(name.to_string(), TypeDef {
                generics: vec![],
                type_expr: TypeExpr::Name(name.to_string(), vec![]),
            });
        }
        
        // Other predefined types
        self.types.insert("bool".to_string(), TypeDef {
            generics: vec![],
            type_expr: TypeExpr::Name("bool".to_string(), vec![]),
        });
        
        self.types.insert("nil".to_string(), TypeDef {
            generics: vec![],
            type_expr: TypeExpr::Name("nil".to_string(), vec![]),
        });
        
        self.types.insert("never".to_string(), TypeDef {
            generics: vec![],
            type_expr: TypeExpr::Never,
        });
        
        // str type (struct with data and len)
        self.types.insert("str".to_string(), TypeDef {
            generics: vec![],
            type_expr: TypeExpr::Struct {
                fields: vec![
                    StructField {
                        name: "data".to_string(),
                        type_expr: TypeExpr::Ptr {
                            mutable: false,
                            inner: Box::new(TypeExpr::Name("u8".to_string(), vec![])),
                        },
                    },
                    StructField {
                        name: "len".to_string(),
                        type_expr: TypeExpr::Name("int".to_string(), vec![]),
                    },
                ],
                methods: vec![],
            },
        });
        
        // array type constructor
        // This is a special type that will be handled during type checking
        self.types.insert("array".to_string(), TypeDef {
            generics: vec![GenericParam::Type { name: "T".to_string(), constraint: None }],
            type_expr: TypeExpr::Struct {
                fields: vec![
                    StructField {
                        name: "data".to_string(),
                        type_expr: TypeExpr::Ptr {
                            mutable: false,
                            inner: Box::new(TypeExpr::Name("T".to_string(), vec![])),
                        },
                    },
                    StructField {
                        name: "len".to_string(),
                        type_expr: TypeExpr::Name("int".to_string(), vec![]),
                    },
                ],
                methods: vec![],
            },
        });
    }
    
    fn check_program(&mut self, program: Program) -> Result<Program> {
        // First pass: collect all type and function declarations
        for item in &program.items {
            match item {
                TopLevelItem::TypeDecl(decl) => {
                    self.types.insert(decl.name.clone(), TypeDef {
                        generics: decl.generics.clone(),
                        type_expr: decl.type_expr.clone(),
                    });
                }
                TopLevelItem::FunctionDecl(decl) => {
                    self.functions.insert(decl.name.clone(), FunctionSig {
                        generics: decl.generics.clone(),
                        params: decl.params.clone(),
                        return_type: decl.return_type.clone(),
                        where_clause: decl.where_clause.clone(),
                    });
                }
                TopLevelItem::StaticDecl(decl) => {
                    self.statics.insert(decl.name.clone(), decl.type_expr.clone());
                }
            }
        }
        
        // Second pass: check all declarations
        for item in &program.items {
            match item {
                TopLevelItem::TypeDecl(decl) => {
                    self.check_type_decl(decl)?;
                }
                TopLevelItem::FunctionDecl(decl) => {
                    self.check_function_decl(decl)?;
                }
                TopLevelItem::StaticDecl(decl) => {
                    self.check_static_decl(decl)?;
                }
            }
        }
        
        // Check for main function
        if !self.functions.contains_key("main") {
            return Err(anyhow!("No main function found"));
        }
        
        Ok(program)
    }
    
    fn check_type_decl(&mut self, decl: &TypeDecl) -> Result<()> {
        self.push_generics(&decl.generics);
        self.validate_type_expr(&decl.type_expr)?;
        
        // Check methods in enum/struct declarations
        match &decl.type_expr {
            TypeExpr::Enum { methods, .. } | TypeExpr::Struct { methods, .. } => {
                let self_type = TypeExpr::Name(decl.name.clone(), 
                    decl.generics.iter().map(|g| match g {
                        GenericParam::Type { name, .. } => GenericArg::Type(TypeExpr::Name(name.clone(), vec![])),
                        GenericParam::Static { name, type_expr } => GenericArg::Type(type_expr.clone()),
                    }).collect()
                );
                
                for method in methods {
                    self.check_method_decl(method, &self_type)?;
                }
            }
            _ => {}
        }
        
        self.pop_generics(&decl.generics);
        Ok(())
    }
    
    fn check_method_decl(&mut self, method: &MethodDecl, self_type: &TypeExpr) -> Result<()> {
        self.push_generics(&method.generics);
        
        // Check parameter types
        for param in &method.params {
            if let Some(ref type_expr) = param.type_expr {
                self.validate_type_expr(type_expr)?;
            }
        }
        
        // Check return type
        self.validate_type_expr(&method.return_type)?;
        
        // Check where clause
        for constraint in &method.where_clause {
            self.validate_type_expr(&constraint.type_expr)?;
        }
        
        // Check method body
        self.push_scope();
        
        // Add parameters to scope, including self
        for param in &method.params {
            if param.name == "self" {
                // Add self with the containing type
                self.add_var("self", self_type.clone(), param.mutable)?;
            } else if let Some(ref type_expr) = param.type_expr {
                self.add_var(&param.name, type_expr.clone(), param.mutable)?;
            }
        }
        
        self.current_return_type = Some(method.return_type.clone());
        let _ = self.check_block(&method.body)?;
        self.current_return_type = None;
        
        self.pop_scope();
        self.pop_generics(&method.generics);
        Ok(())
    }
    
    fn check_function_decl(&mut self, decl: &FunctionDecl) -> Result<()> {
        self.push_generics(&decl.generics);
        
        // Check parameter types
        for param in &decl.params {
            if let Some(ref type_expr) = param.type_expr {
                self.validate_type_expr(type_expr)?;
            }
        }
        
        // Check return type
        self.validate_type_expr(&decl.return_type)?;
        
        // Check where clause
        for constraint in &decl.where_clause {
            self.validate_type_expr(&constraint.type_expr)?;
        }
        
        // Check function body if present
        if let Some(ref body) = decl.body {
            self.push_scope();
            
            // Add parameters to scope
            for param in &decl.params {
                if param.name == "self" {
                    // Handle self parameter - need to determine the type from context
                    // For now, we'll skip this
                } else if let Some(ref type_expr) = param.type_expr {
                    self.add_var(&param.name, type_expr.clone(), param.mutable)?;
                }
            }
            
            self.current_return_type = Some(decl.return_type.clone());
            let _ = self.check_block(body)?;
            self.current_return_type = None;
            
            self.pop_scope();
        } else {
            // No body = external function declaration
        }
        
        self.pop_generics(&decl.generics);
        Ok(())
    }
    
    fn check_static_decl(&mut self, decl: &StaticDecl) -> Result<()> {
        self.push_generics(&decl.generics);
        
        self.validate_type_expr(&decl.type_expr)?;
        
        // Check where clause
        for constraint in &decl.where_clause {
            self.validate_type_expr(&constraint.type_expr)?;
        }
        
        // Check that the value expression matches the declared type
        let value_type = self.infer_expr_type(&decl.value)?;
        self.check_type_compatibility(&value_type, &decl.type_expr)?;
        
        self.pop_generics(&decl.generics);
        Ok(())
    }
    
    fn check_block(&mut self, block: &Block) -> Result<Option<TypeExpr>> {
        for stmt in &block.stmts {
            self.check_stmt(stmt)?;
        }
        
        // If the block has a final expression, check it and return its type
        if let Some(ref expr) = block.expr {
            let expr_type = self.infer_expr_type(expr)?;
            Ok(Some(expr_type))
        } else {
            Ok(None)
        }
    }
    
    fn check_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match stmt {
            Stmt::Expr(expr) => {
                self.infer_expr_type(expr)?;
                Ok(())
            }
            Stmt::Decl { mutable, name, type_expr, value } => {
                self.validate_type_expr(type_expr)?;
                
                // Special handling for literals that need type context
                match (value, type_expr) {
                    // Integer literal assignments
                    (Expr::Literal(Literal::Int(val)), TypeExpr::Name(type_name, _)) 
                        if self.is_integer_type_name(type_name) => {
                        if self.int_fits_in_type(*val, type_name) {
                            self.add_var(name, type_expr.clone(), *mutable)?;
                            return Ok(());
                        }
                    }
                    // Unary minus on integer literal
                    (Expr::Unary { op: UnaryOp::Minus, expr }, TypeExpr::Name(type_name, _))
                        if self.is_integer_type_name(type_name) => {
                        if let Expr::Literal(Literal::Int(val)) = expr.as_ref() {
                            let neg_val = -(*val);
                            if self.int_fits_in_type(neg_val, type_name) {
                                self.add_var(name, type_expr.clone(), *mutable)?;
                                return Ok(());
                            }
                        }
                    }
                    // Array literal assignments
                    (Expr::ArrayLiteral(elements), TypeExpr::Array { size, element }) => {
                        // Check array size if specified
                        if let Some(size_expr) = size {
                            if let Expr::Literal(Literal::Int(expected_size)) = size_expr.as_ref() {
                                if elements.len() != *expected_size as usize {
                                    return Err(anyhow!(
                                        "Array literal has {} elements but type expects {}",
                                        elements.len(),
                                        expected_size
                                    ));
                                }
                            }
                        }
                        
                        // Type check each element
                        for elem in elements {
                            let elem_type = self.infer_expr_type(elem)?;
                            self.check_type_compatibility(&elem_type, element)?;
                        }
                        
                        self.add_var(name, type_expr.clone(), *mutable)?;
                        return Ok(());
                    }
                    // Struct literal assignments
                    (Expr::StructLiteral { fields }, TypeExpr::Struct { fields: struct_fields, .. }) => {
                        // Check that all required fields are present and have correct types
                        for struct_field in struct_fields {
                            if let Some((_, field_expr)) = fields.iter().find(|(name, _)| name == &struct_field.name) {
                                let field_type = self.infer_expr_type(field_expr)?;
                                self.check_type_compatibility(&field_type, &struct_field.type_expr)?;
                            } else {
                                return Err(anyhow!("Missing field '{}' in struct literal", struct_field.name));
                            }
                        }
                        
                        // Check that no extra fields are present
                        for (field_name, _) in fields {
                            if !struct_fields.iter().any(|f| &f.name == field_name) {
                                return Err(anyhow!("Unknown field '{}' in struct literal", field_name));
                            }
                        }
                        
                        self.add_var(name, type_expr.clone(), *mutable)?;
                        return Ok(());
                    }
                    _ => {}
                }
                
                // Default handling
                let value_type = self.infer_expr_type(value)?;
                self.check_type_compatibility(&value_type, type_expr)?;
                self.add_var(name, type_expr.clone(), *mutable)?;
                Ok(())
            }
            Stmt::Block(block) => {
                self.push_scope();
                let _ = self.check_block(block)?;
                self.pop_scope();
                Ok(())
            }
            Stmt::If { condition, then_block, else_part } => {
                let cond_type = self.infer_expr_type(condition)?;
                self.check_type_compatibility(&cond_type, &TypeExpr::Name("bool".to_string(), vec![]))?;
                
                self.push_scope();
                let _ = self.check_block(then_block)?;
                self.pop_scope();
                
                if let Some(else_stmt) = else_part {
                    self.check_stmt(else_stmt)?;
                }
                Ok(())
            }
            Stmt::While { label, condition, body } => {
                let cond_type = self.infer_expr_type(condition)?;
                self.check_type_compatibility(&cond_type, &TypeExpr::Name("bool".to_string(), vec![]))?;
                
                self.loop_labels.push(label.clone());
                self.push_scope();
                let _ = self.check_block(body)?;
                self.pop_scope();
                self.loop_labels.pop();
                Ok(())
            }
            Stmt::For { label, kind, body } => {
                self.loop_labels.push(label.clone());
                self.push_scope();
                
                match kind {
                    ForKind::In { mutable, var, iter } => {
                        let iter_type = self.infer_expr_type(iter)?;
                        // Check that iter is an array type
                        match &iter_type {
                            TypeExpr::Array { element, .. } => {
                                if *mutable {
                                    // For mutable iteration, var has type mut ptr[T]
                                    let ptr_type = TypeExpr::Ptr {
                                        mutable: true,
                                        inner: element.clone(),
                                    };
                                    self.add_var(var, ptr_type, false)?;
                                } else {
                                    // For immutable iteration, var has type T
                                    self.add_var(var, (**element).clone(), false)?;
                                }
                            }
                            _ => return Err(anyhow!("For-in loop requires array type")),
                        }
                    }
                    ForKind::C { init, condition, update } => {
                        if let Some(init) = init {
                            self.validate_type_expr(&init.type_expr)?;
                            let value_type = self.infer_expr_type(&init.value)?;
                            self.check_type_compatibility(&value_type, &init.type_expr)?;
                            self.add_var(&init.name, init.type_expr.clone(), init.mutable)?;
                        }
                        
                        if let Some(condition) = condition {
                            let cond_type = self.infer_expr_type(condition)?;
                            self.check_type_compatibility(&cond_type, &TypeExpr::Name("bool".to_string(), vec![]))?;
                        }
                        
                        if let Some(update) = update {
                            self.infer_expr_type(update)?;
                        }
                    }
                }
                
                let _ = self.check_block(body)?;
                self.pop_scope();
                self.loop_labels.pop();
                Ok(())
            }
            Stmt::Match { is_type, expr, arms } => {
                let expr_type = self.infer_expr_type(expr)?;
                
                if *is_type {
                    // Type match - check that expr is an enum
                    let resolved_type = self.resolve_type(&expr_type)?;
                    match &resolved_type {
                        TypeExpr::Enum { .. } => {
                            // TODO: Check exhaustiveness
                            for arm in arms {
                                self.check_match_arm(arm, &expr_type, *is_type)?;
                            }
                        }
                        _ => return Err(anyhow!("Type match requires enum type")),
                    }
                } else {
                    // Value match
                    for arm in arms {
                        self.check_match_arm(arm, &expr_type, *is_type)?;
                    }
                }
                Ok(())
            }
            Stmt::Return(expr) => {
                if let Some(ref return_type) = self.current_return_type {
                    let return_type = return_type.clone();
                    if let Some(expr) = expr {
                        let expr_type = self.infer_expr_type(expr)?;
                        self.check_type_compatibility(&expr_type, &return_type)?;
                    } else {
                        self.check_type_compatibility(&TypeExpr::Name("nil".to_string(), vec![]), &return_type)?;
                    }
                } else {
                    return Err(anyhow!("Return statement outside of function"));
                }
                Ok(())
            }
            Stmt::Break(label) => {
                if let Some(label) = label {
                    if !self.loop_labels.iter().any(|l| l.as_ref() == Some(label)) {
                        return Err(anyhow!("Unknown loop label: {}", label));
                    }
                } else if self.loop_labels.is_empty() {
                    return Err(anyhow!("Break statement outside of loop"));
                }
                Ok(())
            }
            Stmt::Continue(label) => {
                if let Some(label) = label {
                    if !self.loop_labels.iter().any(|l| l.as_ref() == Some(label)) {
                        return Err(anyhow!("Unknown loop label: {}", label));
                    }
                } else if self.loop_labels.is_empty() {
                    return Err(anyhow!("Continue statement outside of loop"));
                }
                Ok(())
            }
            Stmt::Labeled { label: _, stmt } => {
                self.check_stmt(stmt)?;
                Ok(())
            }
        }
    }
    
    fn check_match_arm(&mut self, arm: &MatchArm, expr_type: &TypeExpr, is_type: bool) -> Result<()> {
        self.push_scope();
        
        // Check pattern and bind variables
        match &arm.pattern {
            Pattern::Identifier(name) => {
                // Clone to avoid borrow checker issues
                let name = name.clone();
                let expr_type = expr_type.clone();
                self.add_var(&name, expr_type, false)?;
            }
            Pattern::Type(_, _) => {
                if !is_type {
                    return Err(anyhow!("Type pattern in value match"));
                }
                // TODO: Bind the extracted value
            }
            Pattern::Literal(lit) => {
                let lit_type = self.literal_type(lit);
                self.check_type_compatibility(&lit_type, expr_type)?;
            }
            Pattern::Wildcard => {}
        }
        
        // Check arm body
        match &arm.body {
            MatchArmBody::Block(block) => {
                self.check_block(block)?;
                ()
            }
            MatchArmBody::Expr(expr) => {
                self.infer_expr_type(expr)?;
            }
        }
        
        self.pop_scope();
        Ok(())
    }
    
    fn infer_expr_type(&mut self, expr: &Expr) -> Result<TypeExpr> {
        match expr {
            Expr::Identifier(name) => {
                // Look up variable
                for scope in self.scopes.iter().rev() {
                    if let Some(info) = scope.get(name) {
                        return Ok(info.type_expr.clone());
                    }
                }
                
                // Look up static
                if let Some(type_expr) = self.statics.get(name) {
                    return Ok(type_expr.clone());
                }
                
                // Look up function
                if let Some(sig) = self.functions.get(name) {
                    return Ok(TypeExpr::Function {
                        params: sig.params.clone(),
                        return_type: Box::new(sig.return_type.clone()),
                    });
                }
                
                // Look up type name
                if let Some(typedef) = self.types.get(name) {
                    // This is a type constructor
                    return Ok(TypeExpr::Name(name.clone(), vec![]));
                }
                
                Err(anyhow!("Unknown identifier: {}", name))
            }
            Expr::Literal(lit) => Ok(self.literal_type(lit)),
            Expr::Binary { op, left, right } => {
                let left_type = self.infer_expr_type(left)?;
                let right_type = self.infer_expr_type(right)?;
                
                use BinaryOp::*;
                match op {
                    // Arithmetic ops
                    Add | Sub | Mul | Div | Mod => {
                        // Check for pointer arithmetic
                        match (&left_type, &right_type, op) {
                            // Pointer + integer
                            (TypeExpr::Ptr { .. }, _, Add) => {
                                self.check_integer_type(&right_type)?;
                                return Ok(left_type);
                            }
                            // Pointer - integer
                            (TypeExpr::Ptr { .. }, _, Sub) => {
                                if let TypeExpr::Ptr { .. } = &right_type {
                                    // Pointer - pointer = integer difference
                                    return Ok(TypeExpr::Name("int".to_string(), vec![]));
                                } else {
                                    // Pointer - integer = pointer
                                    self.check_integer_type(&right_type)?;
                                    return Ok(left_type);
                                }
                            }
                            _ => {}
                        }
                        
                        // Special handling for integer literals
                        if self.can_assign_int_literal(left, &right_type) {
                            self.check_numeric_type(&right_type)?;
                            return Ok(right_type);
                        }
                        if self.can_assign_int_literal(right, &left_type) {
                            self.check_numeric_type(&left_type)?;
                            return Ok(left_type);
                        }
                        
                        self.check_type_compatibility(&left_type, &right_type)?;
                        self.check_numeric_type(&left_type)?;
                        Ok(left_type)
                    }
                    // Bitwise ops
                    BitAnd | BitOr | BitXor | Shl | Shr => {
                        // Special handling for integer literals
                        if self.can_assign_int_literal(left, &right_type) {
                            self.check_integer_type(&right_type)?;
                            return Ok(right_type);
                        }
                        if self.can_assign_int_literal(right, &left_type) {
                            self.check_integer_type(&left_type)?;
                            return Ok(left_type);
                        }
                        
                        self.check_type_compatibility(&left_type, &right_type)?;
                        self.check_integer_type(&left_type)?;
                        Ok(left_type)
                    }
                    // Comparison ops
                    Eq | Ne => {
                        // Special handling for integer literal comparisons
                        if self.can_assign_int_literal(left, &right_type) || 
                           self.can_assign_int_literal(right, &left_type) {
                            return Ok(TypeExpr::Name("bool".to_string(), vec![]));
                        }
                        
                        self.check_type_compatibility(&left_type, &right_type)?;
                        Ok(TypeExpr::Name("bool".to_string(), vec![]))
                    }
                    Lt | Le | Gt | Ge => {
                        // Special handling for integer literal comparisons
                        if self.can_assign_int_literal(left, &right_type) || 
                           self.can_assign_int_literal(right, &left_type) {
                            self.check_numeric_type(&left_type)?;
                            return Ok(TypeExpr::Name("bool".to_string(), vec![]));
                        }
                        
                        self.check_type_compatibility(&left_type, &right_type)?;
                        self.check_numeric_type(&left_type)?;
                        Ok(TypeExpr::Name("bool".to_string(), vec![]))
                    }
                    // Logical ops
                    And | Or => {
                        self.check_type_compatibility(&left_type, &TypeExpr::Name("bool".to_string(), vec![]))?;
                        self.check_type_compatibility(&right_type, &TypeExpr::Name("bool".to_string(), vec![]))?;
                        Ok(TypeExpr::Name("bool".to_string(), vec![]))
                    }
                    // Assignment ops
                    Assign | AddAssign | SubAssign | MulAssign | DivAssign | ModAssign |
                    BitAndAssign | BitOrAssign | BitXorAssign | ShlAssign | ShrAssign => {
                        self.check_assignable(left)?;
                        
                        // Special handling for integer literal assignments
                        if *op == BinaryOp::Assign && self.can_assign_int_literal(right, &left_type) {
                            return Ok(left_type);
                        }
                        
                        self.check_type_compatibility(&right_type, &left_type)?;
                        Ok(left_type)
                    }
                }
            }
            Expr::Unary { op, expr } => {
                let expr_type = self.infer_expr_type(expr)?;
                
                use UnaryOp::*;
                match op {
                    Plus | Minus => {
                        self.check_numeric_type(&expr_type)?;
                        Ok(expr_type)
                    }
                    BitNot => {
                        self.check_integer_type(&expr_type)?;
                        Ok(expr_type)
                    }
                    Not => {
                        self.check_type_compatibility(&expr_type, &TypeExpr::Name("bool".to_string(), vec![]))?;
                        Ok(TypeExpr::Name("bool".to_string(), vec![]))
                    }
                    Deref => {
                        match &expr_type {
                            TypeExpr::Ptr { inner, .. } => Ok((**inner).clone()),
                            _ => Err(anyhow!("Cannot dereference non-pointer type")),
                        }
                    }
                    AddrOf => {
                        // Check that expr is an lvalue
                        self.check_lvalue(expr)?;
                        let mutable = self.is_mutable_lvalue(expr)?;
                        Ok(TypeExpr::Ptr {
                            mutable,
                            inner: Box::new(expr_type),
                        })
                    }
                }
            }
            Expr::Call { func, args } => {
                let func_type = self.infer_expr_type(func)?;
                match func_type {
                    TypeExpr::Function { params, return_type } => {
                        if args.len() != params.len() {
                            return Err(anyhow!("Function expects {} arguments, got {}", params.len(), args.len()));
                        }
                        
                        for (arg, param) in args.iter().zip(params.iter()) {
                            let arg_type = self.infer_expr_type(arg)?;
                            if let Some(ref param_type) = param.type_expr {
                                // Special handling for integer literal arguments
                                if self.can_assign_int_literal(arg, param_type) {
                                    continue;
                                }
                                self.check_type_compatibility(&arg_type, param_type)?;
                            }
                        }
                        
                        Ok(*return_type)
                    }
                    _ => Err(anyhow!("Cannot call non-function type")),
                }
            }
            Expr::Index { array, index } => {
                let array_type = self.infer_expr_type(array)?;
                let index_type = self.infer_expr_type(index)?;
                
                // Check that index is an integer
                self.check_integer_type(&index_type)?;
                
                match array_type {
                    TypeExpr::Array { element, .. } => Ok(*element),
                    _ => Err(anyhow!("Cannot index non-array type")),
                }
            }
            Expr::Field { expr, field } => {
                let expr_type = self.infer_expr_type(expr)?;
                self.lookup_field(&expr_type, field)
            }
            Expr::ArrayLiteral(elements) => {
                // Cannot infer array type from literal alone - need context
                // This will be handled when we have better type inference
                Err(anyhow!("Cannot infer type of array literal without context"))
            }
            Expr::StructLiteral { fields } => {
                // Cannot infer struct type from literal alone
                Err(anyhow!("Cannot infer type of struct literal without context"))
            }
            Expr::FunctionLiteral { params, return_type, body } => {
                self.push_scope();
                
                // Add parameters to scope
                for param in params {
                    if let Some(ref type_expr) = param.type_expr {
                        self.add_var(&param.name, type_expr.clone(), param.mutable)?;
                    }
                }
                
                // Check function body
                let old_return_type = self.current_return_type.clone();
                self.current_return_type = Some(return_type.clone());
                let _ = self.check_block(body)?;
                self.current_return_type = old_return_type;
                
                self.pop_scope();
                
                Ok(TypeExpr::Function {
                    params: params.clone(),
                    return_type: Box::new(return_type.clone()),
                })
            }
            Expr::If { condition, then_expr, else_expr } => {
                let cond_type = self.infer_expr_type(condition)?;
                self.check_type_compatibility(&cond_type, &TypeExpr::Name("bool".to_string(), vec![]))?;
                
                let then_type = self.infer_expr_type(then_expr)?;
                let else_type = self.infer_expr_type(else_expr)?;
                
                self.check_type_compatibility(&then_type, &else_type)?;
                Ok(then_type)
            }
            Expr::Match { is_type, expr, arms } => {
                let expr_type = self.infer_expr_type(expr)?;
                
                if arms.is_empty() {
                    return Err(anyhow!("Match expression must have at least one arm"));
                }
                
                // Infer type from first arm
                let first_arm_type = match &arms[0].body {
                    MatchArmBody::Block(_) => TypeExpr::Name("nil".to_string(), vec![]),
                    MatchArmBody::Expr(expr) => self.infer_expr_type(expr)?,
                };
                
                // Check remaining arms have same type
                for arm in &arms[1..] {
                    let arm_type = match &arm.body {
                        MatchArmBody::Block(_) => TypeExpr::Name("nil".to_string(), vec![]),
                        MatchArmBody::Expr(expr) => self.infer_expr_type(expr)?,
                    };
                    self.check_type_compatibility(&arm_type, &first_arm_type)?;
                }
                
                Ok(first_arm_type)
            }
            Expr::Sizeof(operand) => {
                match operand {
                    SizeofOperand::Type(type_expr) => self.validate_type_expr(type_expr)?,
                    SizeofOperand::Expr(expr) => {
                        self.infer_expr_type(expr)?;
                    }
                }
                Ok(TypeExpr::Name("int".to_string(), vec![]))
            }
            Expr::Cast { expr, type_expr } => {
                let expr_type = self.infer_expr_type(expr)?;
                self.validate_type_expr(type_expr)?;
                
                // Check if cast is valid
                self.check_valid_cast(&expr_type, type_expr)?;
                
                Ok(type_expr.clone())
            }
            Expr::Type(type_expr) => {
                // For a type used as an expression, return the type itself
                // This is used for accessing static members/constructors on types
                self.validate_type_expr(type_expr)?;
                Ok(type_expr.clone())
            }
        }
    }
    
    fn literal_type(&self, lit: &Literal) -> TypeExpr {
        match lit {
            Literal::Int(_) => TypeExpr::Name("i32".to_string(), vec![]), // Default to i32
            Literal::Float(_) => TypeExpr::Name("float".to_string(), vec![]),
            Literal::Bool(_) => TypeExpr::Name("bool".to_string(), vec![]),
            Literal::String(_) => TypeExpr::Name("str".to_string(), vec![]),
            Literal::Nil => TypeExpr::Name("nil".to_string(), vec![]),
        }
    }
    
    fn validate_type_expr(&self, type_expr: &TypeExpr) -> Result<()> {
        match type_expr {
            TypeExpr::Name(name, args) => {
                if !self.types.contains_key(name) && !self.is_generic_param(name) {
                    return Err(anyhow!("Unknown type: {}", name));
                }
                
                // TODO: Check generic arguments match expected parameters
                
                Ok(())
            }
            TypeExpr::Ptr { inner, .. } => self.validate_type_expr(inner),
            TypeExpr::Array { size, element } => {
                if let Some(size) = size {
                    // TODO: Check that size is a compile-time constant
                }
                self.validate_type_expr(element)
            }
            TypeExpr::Struct { fields, methods } => {
                for field in fields {
                    self.validate_type_expr(&field.type_expr)?;
                }
                // TODO: Check methods
                Ok(())
            }
            TypeExpr::Enum { variants, backing_type, methods } => {
                for variant in variants {
                    match &variant.data {
                        EnumVariantData::Type(type_expr) => self.validate_type_expr(type_expr)?,
                        _ => {}
                    }
                }
                Ok(())
            }
            TypeExpr::Function { params, return_type } => {
                for param in params {
                    if let Some(ref type_expr) = param.type_expr {
                        self.validate_type_expr(type_expr)?;
                    }
                }
                self.validate_type_expr(return_type)
            }
            TypeExpr::Union(types) | TypeExpr::Intersection(types) => {
                for t in types {
                    self.validate_type_expr(t)?;
                }
                Ok(())
            }
            TypeExpr::Never => Ok(()),
        }
    }
    
    fn check_type_compatibility(&self, from: &TypeExpr, to: &TypeExpr) -> Result<()> {
        // For now, just check structural equality
        // TODO: Implement proper structural typing rules
        if !self.types_equal(from, to) {
            return Err(anyhow!("Type mismatch: expected {:?}, got {:?}", to, from));
        }
        Ok(())
    }
    
    fn types_equal(&self, a: &TypeExpr, b: &TypeExpr) -> bool {
        match (a, b) {
            (TypeExpr::Name(a_name, a_args), TypeExpr::Name(b_name, b_args)) => {
                a_name == b_name && a_args.len() == b_args.len()
                // TODO: Check generic arguments
            }
            (TypeExpr::Ptr { mutable: a_mut, inner: a_inner }, 
             TypeExpr::Ptr { mutable: b_mut, inner: b_inner }) => {
                // Mutable pointer can be used as immutable
                (a_mut == b_mut || (*a_mut && !*b_mut)) && self.types_equal(a_inner, b_inner)
            }
            (TypeExpr::Array { size: a_size, element: a_elem },
             TypeExpr::Array { size: b_size, element: b_elem }) => {
                // TODO: Check sizes
                self.types_equal(a_elem, b_elem)
            }
            (TypeExpr::Never, _) => true, // Never is subtype of all types
            _ => false, // TODO: Implement more cases
        }
    }
    
    fn check_numeric_type(&self, type_expr: &TypeExpr) -> Result<()> {
        match type_expr {
            TypeExpr::Name(name, _) => {
                let numeric_types = vec![
                    "u8", "u16", "u32", "u64",
                    "i8", "i16", "i32", "i64",
                    "int", "uint", "float"
                ];
                if numeric_types.contains(&name.as_str()) {
                    Ok(())
                } else {
                    Err(anyhow!("Expected numeric type, got {}", name))
                }
            }
            _ => Err(anyhow!("Expected numeric type")),
        }
    }
    
    fn check_integer_type(&self, type_expr: &TypeExpr) -> Result<()> {
        match type_expr {
            TypeExpr::Name(name, _) => {
                let integer_types = vec![
                    "u8", "u16", "u32", "u64",
                    "i8", "i16", "i32", "i64",
                    "int", "uint"
                ];
                if integer_types.contains(&name.as_str()) {
                    Ok(())
                } else {
                    Err(anyhow!("Expected integer type, got {}", name))
                }
            }
            _ => Err(anyhow!("Expected integer type")),
        }
    }
    
    fn check_lvalue(&self, expr: &Expr) -> Result<()> {
        match expr {
            Expr::Identifier(_) => Ok(()),
            Expr::Index { .. } => Ok(()),
            Expr::Field { .. } => Ok(()),
            Expr::Unary { op: UnaryOp::Deref, .. } => Ok(()),
            Expr::Type(_) => Err(anyhow!("Type expression is not an lvalue")),
            _ => Err(anyhow!("Expression is not an lvalue")),
        }
    }
    
    fn is_mutable_lvalue(&mut self, expr: &Expr) -> Result<bool> {
        match expr {
            Expr::Identifier(name) => {
                for scope in self.scopes.iter().rev() {
                    if let Some(info) = scope.get(name) {
                        return Ok(info.mutable);
                    }
                }
                Err(anyhow!("Unknown identifier: {}", name))
            }
            Expr::Index { array, .. } => self.is_mutable_lvalue(array),
            Expr::Field { expr, .. } => self.is_mutable_lvalue(expr),
            Expr::Unary { op: UnaryOp::Deref, expr } => {
                let ptr_type = self.infer_expr_type(expr)?;
                match ptr_type {
                    TypeExpr::Ptr { mutable, .. } => Ok(mutable),
                    _ => Err(anyhow!("Cannot dereference non-pointer")),
                }
            }
            Expr::Type(_) => Err(anyhow!("Type expression is not an lvalue")),
            _ => Err(anyhow!("Not an lvalue")),
        }
    }
    
    fn check_assignable(&mut self, expr: &Expr) -> Result<()> {
        self.check_lvalue(expr)?;
        if !self.is_mutable_lvalue(expr)? {
            return Err(anyhow!("Cannot assign to immutable lvalue"));
        }
        Ok(())
    }
    
    fn check_valid_cast(&self, from: &TypeExpr, to: &TypeExpr) -> Result<()> {
        // Check valid casts according to spec
        match (from, to) {
            // Numeric casts
            (TypeExpr::Name(from_name, _), TypeExpr::Name(to_name, _)) => {
                let numeric_types = vec![
                    "u8", "u16", "u32", "u64",
                    "i8", "i16", "i32", "i64",
                    "int", "uint", "float"
                ];
                
                if numeric_types.contains(&from_name.as_str()) && numeric_types.contains(&to_name.as_str()) {
                    return Ok(());
                }
            }
            
            // Pointer casts
            (TypeExpr::Ptr { .. }, TypeExpr::Ptr { .. }) => return Ok(()),
            
            // Integer to pointer
            (TypeExpr::Name(name, _), TypeExpr::Ptr { .. }) => {
                let integer_types = vec![
                    "u8", "u16", "u32", "u64",
                    "i8", "i16", "i32", "i64",
                    "int", "uint"
                ];
                if integer_types.contains(&name.as_str()) {
                    return Ok(());
                }
            }
            
            // Pointer to integer
            (TypeExpr::Ptr { .. }, TypeExpr::Name(name, _)) => {
                let integer_types = vec![
                    "u8", "u16", "u32", "u64",
                    "i8", "i16", "i32", "i64",
                    "int", "uint"
                ];
                if integer_types.contains(&name.as_str()) {
                    return Ok(());
                }
            }
            
            _ => {}
        }
        
        Err(anyhow!("Invalid cast from {:?} to {:?}", from, to))
    }
    
    fn lookup_field(&self, type_expr: &TypeExpr, field_name: &str) -> Result<TypeExpr> {
        match type_expr {
            TypeExpr::Struct { fields, methods } => {
                // First check fields
                for field in fields {
                    if field.name == field_name {
                        return Ok(field.type_expr.clone());
                    }
                }
                // Then check methods
                for method in methods {
                    if method.name == field_name {
                        return Ok(TypeExpr::Function {
                            params: method.params.clone(),
                            return_type: Box::new(method.return_type.clone()),
                        });
                    }
                }
                Err(anyhow!("No field or method '{}' in struct", field_name))
            }
            TypeExpr::Enum { variants, backing_type, methods } => {
                // First check if the field name matches a variant
                for variant in variants {
                    if variant.name == field_name {
                        match &variant.data {
                            EnumVariantData::Type(type_expr) => return Ok(type_expr.clone()),
                            EnumVariantData::Unit => return Ok(TypeExpr::Name("nil".to_string(), vec![])),
                            EnumVariantData::Value(_) => return Err(anyhow!("Cannot access numeric enum variant as field")),
                        }
                    }
                }
                // Then check methods
                for method in methods {
                    if method.name == field_name {
                        return Ok(TypeExpr::Function {
                            params: method.params.clone(),
                            return_type: Box::new(method.return_type.clone()),
                        });
                    }
                }
                Err(anyhow!("No variant or method '{}' in enum", field_name))
            }
            TypeExpr::Name(name, args) => {
                // Look up type definition
                if let Some(typedef) = self.types.get(name) {
                    // Check if this is accessing a field on an instance of the type
                    match &typedef.type_expr {
                        TypeExpr::Enum { variants, backing_type, methods } => {
                            // First check methods (for instance access like v.get())
                            for method in methods {
                                if method.name == field_name {
                                    return Ok(TypeExpr::Function {
                                        params: method.params.clone(),
                                        return_type: Box::new(method.return_type.clone()),
                                    });
                                }
                            }
                            
                            // Then check variants (for instance field access like v.int_val)
                            for variant in variants {
                                if variant.name == field_name {
                                    match &variant.data {
                                        EnumVariantData::Type(type_expr) => return Ok(type_expr.clone()),
                                        EnumVariantData::Unit => return Ok(TypeExpr::Name("nil".to_string(), vec![])),
                                        EnumVariantData::Value(_) => return Err(anyhow!("Cannot access numeric enum variant as field")),
                                    }
                                }
                            }
                            
                            // Finally check if this is a variant constructor (for type access like value.int_val(...))
                            // This happens when accessing the type name itself, not an instance
                            // We can distinguish by checking if this is being called with arguments
                            for variant in variants {
                                if variant.name == field_name {
                                    // Return a constructor function for this variant
                                    match &variant.data {
                                        EnumVariantData::Type(variant_type) => {
                                            // If the enum is generic, we need to instantiate the variant type
                                            let instantiated_variant_type = if !args.is_empty() && !typedef.generics.is_empty() {
                                                // Create substitution map from generic params to args
                                                let mut substitutions = HashMap::new();
                                                for (i, param) in typedef.generics.iter().enumerate() {
                                                    if let GenericParam::Type { name: param_name, .. } = param {
                                                        if i < args.len() {
                                                            if let GenericArg::Type(arg_type) = &args[i] {
                                                                substitutions.insert(param_name.clone(), arg_type.clone());
                                                            }
                                                        }
                                                    }
                                                }
                                                // Apply substitutions to variant type
                                                self.substitute_type(variant_type, &substitutions)
                                            } else {
                                                variant_type.clone()
                                            };
                                            
                                            // Constructor takes the variant data type and returns the enum type
                                            return Ok(TypeExpr::Function {
                                                params: vec![Param {
                                                    mutable: false,
                                                    name: "value".to_string(),
                                                    type_expr: Some(instantiated_variant_type),
                                                }],
                                                return_type: Box::new(TypeExpr::Name(name.clone(), args.clone())),
                                            });
                                        }
                                        EnumVariantData::Unit => {
                                            // Unit variant constructor takes no arguments
                                            return Ok(TypeExpr::Function {
                                                params: vec![],
                                                return_type: Box::new(TypeExpr::Name(name.clone(), args.clone())),
                                            });
                                        }
                                        EnumVariantData::Value(_) => {
                                            return Err(anyhow!("Cannot use numeric enum variant {} as constructor", field_name));
                                        }
                                    }
                                }
                            }
                            
                            // If we didn't find anything, it's not a valid field
                            return Err(anyhow!("No method or variant '{}' in enum {}", field_name, name));
                        }
                        TypeExpr::Struct { fields, methods } => {
                            // Check struct fields
                            for field in fields {
                                if field.name == field_name {
                                    return Ok(field.type_expr.clone());
                                }
                            }
                            // Check struct methods
                            for method in methods {
                                if method.name == field_name {
                                    return Ok(TypeExpr::Function {
                                        params: method.params.clone(),
                                        return_type: Box::new(method.return_type.clone()),
                                    });
                                }
                            }
                            return Err(anyhow!("No field or method '{}' in struct {}", field_name, name));
                        }
                        _ => {
                            // For non-enum/struct types accessed as type names, 
                            // we don't support field access
                            return Err(anyhow!("Cannot access field '{}' on type {}", field_name, name));
                        }
                    }
                }
                Err(anyhow!("Unknown type: {}", name))
            }
            _ => Err(anyhow!("Cannot access field on non-struct/enum type")),
        }
    }
    
    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }
    
    fn pop_scope(&mut self) {
        self.scopes.pop();
    }
    
    fn add_var(&mut self, name: &str, type_expr: TypeExpr, mutable: bool) -> Result<()> {
        let current_scope = self.scopes.last_mut().unwrap();
        if current_scope.contains_key(name) {
            return Err(anyhow!("Variable '{}' already declared in this scope", name));
        }
        current_scope.insert(name.to_string(), VarInfo { type_expr, mutable });
        Ok(())
    }
    
    fn push_generics(&mut self, generics: &[GenericParam]) {
        // Add generic type parameters to type map
        for param in generics {
            match param {
                GenericParam::Type { name, constraint } => {
                    self.types.insert(name.clone(), TypeDef {
                        generics: vec![],
                        type_expr: TypeExpr::Name(name.clone(), vec![]),
                    });
                }
                GenericParam::Static { .. } => {
                    // TODO: Handle static parameters
                }
            }
        }
    }
    
    fn pop_generics(&mut self, generics: &[GenericParam]) {
        // Remove generic type parameters from type map
        for param in generics {
            match param {
                GenericParam::Type { name, .. } => {
                    self.types.remove(name);
                }
                GenericParam::Static { .. } => {
                    // TODO: Handle static parameters
                }
            }
        }
    }
    
    fn is_generic_param(&self, name: &str) -> bool {
        // Check if name is a generic parameter in current context
        // For now, just check if it's a single uppercase letter
        name.len() == 1 && name.chars().next().unwrap().is_uppercase()
    }
    
    fn is_integer_type_name(&self, name: &str) -> bool {
        let integer_types = vec![
            "u8", "u16", "u32", "u64",
            "i8", "i16", "i32", "i64",
            "int", "uint"
        ];
        integer_types.contains(&name)
    }
    
    fn int_fits_in_type(&self, value: i64, type_name: &str) -> bool {
        match type_name {
            "u8" => value >= 0 && value <= 255,
            "u16" => value >= 0 && value <= 65535,
            "u32" => value >= 0 && value <= 4294967295,
            "u64" => value >= 0,
            "i8" => value >= -128 && value <= 127,
            "i16" => value >= -32768 && value <= 32767,
            "i32" => value >= -2147483648 && value <= 2147483647,
            "i64" => true,
            "int" => true,
            "uint" => value >= 0,
            _ => false,
        }
    }
    
    // Helper to extract integer literal value from an expression
    fn get_integer_literal_value(&self, expr: &Expr) -> Option<i64> {
        match expr {
            Expr::Literal(Literal::Int(val)) => Some(*val),
            Expr::Unary { op: UnaryOp::Minus, expr } => {
                if let Expr::Literal(Literal::Int(val)) = expr.as_ref() {
                    Some(-(*val))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    
    // Check if an integer literal can be assigned to a type
    fn can_assign_int_literal(&self, expr: &Expr, target_type: &TypeExpr) -> bool {
        if let TypeExpr::Name(type_name, _) = target_type {
            if self.is_integer_type_name(type_name) {
                if let Some(val) = self.get_integer_literal_value(expr) {
                    return self.int_fits_in_type(val, type_name);
                }
            }
        }
        false
    }
    
    // Substitute type parameters in a type expression
    fn substitute_type(&self, type_expr: &TypeExpr, substitutions: &HashMap<String, TypeExpr>) -> TypeExpr {
        match type_expr {
            TypeExpr::Name(name, args) => {
                // Check if this is a type parameter that should be substituted
                if let Some(subst_type) = substitutions.get(name) {
                    subst_type.clone()
                } else {
                    // Recursively substitute in generic arguments
                    let new_args = args.iter().map(|arg| {
                        match arg {
                            GenericArg::Type(t) => GenericArg::Type(self.substitute_type(t, substitutions)),
                            _ => arg.clone(),
                        }
                    }).collect();
                    TypeExpr::Name(name.clone(), new_args)
                }
            }
            TypeExpr::Ptr { mutable, inner } => {
                TypeExpr::Ptr {
                    mutable: *mutable,
                    inner: Box::new(self.substitute_type(inner, substitutions)),
                }
            }
            TypeExpr::Array { size, element } => {
                TypeExpr::Array {
                    size: size.clone(),
                    element: Box::new(self.substitute_type(element, substitutions)),
                }
            }
            TypeExpr::Function { params, return_type } => {
                let new_params = params.iter().map(|p| {
                    Param {
                        mutable: p.mutable,
                        name: p.name.clone(),
                        type_expr: p.type_expr.as_ref().map(|t| self.substitute_type(t, substitutions)),
                    }
                }).collect();
                TypeExpr::Function {
                    params: new_params,
                    return_type: Box::new(self.substitute_type(return_type, substitutions)),
                }
            }
            TypeExpr::Struct { fields, methods } => {
                let new_fields = fields.iter().map(|f| {
                    StructField {
                        name: f.name.clone(),
                        type_expr: self.substitute_type(&f.type_expr, substitutions),
                    }
                }).collect();
                // TODO: Handle method substitution if needed
                TypeExpr::Struct {
                    fields: new_fields,
                    methods: methods.clone(),
                }
            }
            TypeExpr::Enum { variants, backing_type, methods } => {
                let new_variants = variants.iter().map(|v| {
                    let new_data = match &v.data {
                        EnumVariantData::Type(t) => EnumVariantData::Type(self.substitute_type(t, substitutions)),
                        _ => v.data.clone(),
                    };
                    EnumVariant {
                        name: v.name.clone(),
                        data: new_data,
                    }
                }).collect();
                // TODO: Handle method substitution if needed
                TypeExpr::Enum {
                    variants: new_variants,
                    backing_type: backing_type.clone(),
                    methods: methods.clone(),
                }
            }
            TypeExpr::Union(types) => {
                TypeExpr::Union(types.iter().map(|t| self.substitute_type(t, substitutions)).collect())
            }
            TypeExpr::Intersection(types) => {
                TypeExpr::Intersection(types.iter().map(|t| self.substitute_type(t, substitutions)).collect())
            }
            TypeExpr::Never => TypeExpr::Never,
        }
    }
    
    // Resolve a type name to its definition
    fn resolve_type(&self, type_expr: &TypeExpr) -> Result<TypeExpr> {
        match type_expr {
            TypeExpr::Name(name, args) => {
                if let Some(typedef) = self.types.get(name) {
                    // If there are generic arguments, we should substitute them
                    // For now, just return the type definition
                    Ok(typedef.type_expr.clone())
                } else {
                    Ok(type_expr.clone())
                }
            }
            _ => Ok(type_expr.clone()),
        }
    }
}