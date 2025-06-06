use anyhow::{anyhow, Result};
use std::fmt::Write;
use std::collections::{HashMap, HashSet};

use crate::ast::*;

pub fn generate(program: &Program) -> Result<String> {
    let mut generator = CodeGenerator::new();
    generator.generate_program(program)
}

struct CodeGenerator {
    output: String,
    indent_level: usize,
    // Track generated type names to avoid duplicates
    generated_types: HashSet<String>,
    // Track generic instantiations
    instantiations: HashMap<String, Vec<String>>,
    // Current function context for return statements
    current_function: Option<String>,
    // Track loop labels for break/continue
    loop_labels: Vec<Option<String>>,
}

impl CodeGenerator {
    fn new() -> Self {
        CodeGenerator {
            output: String::new(),
            indent_level: 0,
            generated_types: HashSet::new(),
            instantiations: HashMap::new(),
            current_function: None,
            loop_labels: Vec::new(),
        }
    }
    
    fn generate_program(&mut self, program: &Program) -> Result<String> {
        // Generate standard includes
        self.emit_line("#include <stdint.h>");
        self.emit_line("#include <stdbool.h>");
        self.emit_line("#include <stddef.h>");
        self.emit_line("");
        
        // Generate predefined types
        self.generate_predefined_types();
        self.emit_line("");
        
        // Forward declare all types and functions
        self.generate_forward_declarations(program)?;
        self.emit_line("");
        
        // Generate type definitions
        for item in &program.items {
            if let TopLevelItem::TypeDecl(decl) = item {
                self.generate_type_decl(decl)?;
            }
        }
        
        // Generate static declarations
        for item in &program.items {
            if let TopLevelItem::StaticDecl(decl) = item {
                self.generate_static_decl(decl)?;
            }
        }
        
        // Generate function implementations
        for item in &program.items {
            if let TopLevelItem::FunctionDecl(decl) = item {
                if decl.body.is_some() {
                    self.generate_function_impl(decl)?;
                }
            }
        }
        
        Ok(self.output.clone())
    }
    
    fn generate_predefined_types(&mut self) {
        // Type aliases for platform-dependent types
        self.emit_line("#if defined(__LP64__) || defined(_WIN64)");
        self.emit_line("typedef int64_t lang_int;");
        self.emit_line("typedef uint64_t lang_uint;");
        self.emit_line("#else");
        self.emit_line("typedef int32_t lang_int;");
        self.emit_line("typedef uint32_t lang_uint;");
        self.emit_line("#endif");
        self.emit_line("");
        
        // Float is always 32-bit
        self.emit_line("typedef float lang_float;");
        self.emit_line("");
        
        // nil type (empty struct)
        self.emit_line("typedef struct {} lang_nil;");
        self.emit_line("");
        
        // str type
        self.emit_line("typedef struct {");
        self.emit_line("    const uint8_t* data;");
        self.emit_line("    lang_int len;");
        self.emit_line("} lang_str;");
        self.emit_line("");
        
        // Array type template will be generated on demand
    }
    
    fn generate_forward_declarations(&mut self, program: &Program) -> Result<()> {
        // Forward declare all struct types
        for item in &program.items {
            if let TopLevelItem::TypeDecl(decl) = item {
                match &decl.type_expr {
                    TypeExpr::Struct { .. } => {
                        self.emit(&format!("typedef struct {} {};", decl.name, decl.name));
                        self.emit_line("");
                    }
                    TypeExpr::Enum { backing_type: Some(backing), .. } => {
                        let c_type = self.c_type_name(backing);
                        self.emit(&format!("typedef {} {};", c_type, decl.name));
                        self.emit_line("");
                    }
                    TypeExpr::Enum { backing_type: None, .. } => {
                        self.emit(&format!("typedef struct {} {};", decl.name, decl.name));
                        self.emit_line("");
                    }
                    _ => {}
                }
            }
        }
        
        // Forward declare all functions
        for item in &program.items {
            if let TopLevelItem::FunctionDecl(decl) = item {
                self.generate_function_signature(decl)?;
                self.emit_line(";");
            }
        }
        
        Ok(())
    }
    
    fn generate_type_decl(&mut self, decl: &TypeDecl) -> Result<()> {
        if self.generated_types.contains(&decl.name) {
            return Ok(());
        }
        self.generated_types.insert(decl.name.clone());
        
        match &decl.type_expr {
            TypeExpr::Struct { fields, methods } => {
                self.emit(&format!("struct {} {{", decl.name));
                self.emit_line("");
                self.indent();
                
                // Generate fields
                for field in fields {
                    let c_type = self.type_to_c(&field.type_expr)?;
                    self.emit(&format!("{} {};", c_type, field.name));
                    self.emit_line("");
                }
                
                self.dedent();
                self.emit_line("};");
                self.emit_line("");
                
                // Generate method implementations
                for method in methods {
                    self.generate_method_impl(&decl.name, method)?;
                }
            }
            TypeExpr::Enum { backing_type, variants, methods } => {
                if let Some(backing) = backing_type {
                    // Numeric enum - generate constants
                    let c_type = self.c_type_name(backing);
                    let mut value = 0i64;
                    
                    for variant in variants {
                        match &variant.data {
                            EnumVariantData::Value(v) => {
                                value = *v;
                                self.emit(&format!("#define {}_{} (({}){})", 
                                    decl.name, variant.name, decl.name, value));
                                self.emit_line("");
                                value += 1;
                            }
                            EnumVariantData::Unit => {
                                self.emit(&format!("#define {}_{} (({}){})", 
                                    decl.name, variant.name, decl.name, value));
                                self.emit_line("");
                                value += 1;
                            }
                            _ => return Err(anyhow!("Numeric enum cannot have data variants")),
                        }
                    }
                } else {
                    // Tagged union enum
                    self.emit(&format!("struct {} {{", decl.name));
                    self.emit_line("");
                    self.indent();
                    
                    // Generate tag enum
                    self.emit("enum {");
                    self.emit_line("");
                    self.indent();
                    for (i, variant) in variants.iter().enumerate() {
                        self.emit(&format!("{}_{}_{}", decl.name, variant.name, "TAG"));
                        if i < variants.len() - 1 {
                            self.emit(",");
                        }
                        self.emit_line("");
                    }
                    self.dedent();
                    self.emit("} tag;");
                    self.emit_line("");
                    
                    // Generate union for data
                    self.emit("union {");
                    self.emit_line("");
                    self.indent();
                    for variant in variants {
                        match &variant.data {
                            EnumVariantData::Type(type_expr) => {
                                let c_type = self.type_to_c(type_expr)?;
                                self.emit(&format!("{} {};", c_type, variant.name));
                                self.emit_line("");
                            }
                            _ => {}
                        }
                    }
                    self.dedent();
                    self.emit("} data;");
                    self.emit_line("");
                    
                    self.dedent();
                    self.emit_line("};");
                    self.emit_line("");
                    
                    // Generate constructor functions
                    for variant in variants {
                        match &variant.data {
                            EnumVariantData::Unit => {
                                self.emit(&format!("static inline {} {}_{}_MAKE(void) {{", 
                                    decl.name, decl.name, variant.name));
                                self.emit_line("");
                                self.indent();
                                self.emit(&format!("{} result;", decl.name));
                                self.emit_line("");
                                self.emit(&format!("result.tag = {}_{}_TAG;", decl.name, variant.name));
                                self.emit_line("");
                                self.emit("return result;");
                                self.emit_line("");
                                self.dedent();
                                self.emit_line("}");
                                self.emit_line("");
                            }
                            EnumVariantData::Type(type_expr) => {
                                let c_type = self.type_to_c(type_expr)?;
                                self.emit(&format!("static inline {} {}_{}_MAKE({} value) {{", 
                                    decl.name, decl.name, variant.name, c_type));
                                self.emit_line("");
                                self.indent();
                                self.emit(&format!("{} result;", decl.name));
                                self.emit_line("");
                                self.emit(&format!("result.tag = {}_{}_TAG;", decl.name, variant.name));
                                self.emit_line("");
                                self.emit(&format!("result.data.{} = value;", variant.name));
                                self.emit_line("");
                                self.emit("return result;");
                                self.emit_line("");
                                self.dedent();
                                self.emit_line("}");
                                self.emit_line("");
                            }
                            _ => {}
                        }
                    }
                    
                    // Generate methods for enums
                    for method in methods {
                        self.generate_method_impl(&decl.name, method)?;
                    }
                }
            }
            _ => {
                // Type alias
                let c_type = self.type_to_c(&decl.type_expr)?;
                self.emit(&format!("typedef {} {};", c_type, decl.name));
                self.emit_line("");
            }
        }
        
        Ok(())
    }
    
    fn generate_static_decl(&mut self, decl: &StaticDecl) -> Result<()> {
        let c_type = self.type_to_c(&decl.type_expr)?;
        self.emit(&format!("static {} {} = ", c_type, decl.name));
        self.generate_expr(&decl.value)?;
        self.emit_line(";");
        self.emit_line("");
        Ok(())
    }
    
    fn generate_function_signature(&mut self, decl: &FunctionDecl) -> Result<()> {
        let return_type = self.type_to_c(&decl.return_type)?;
        self.emit(&format!("{} {}(", return_type, decl.name));
        
        if decl.params.is_empty() {
            self.emit("void");
        } else {
            for (i, param) in decl.params.iter().enumerate() {
                if i > 0 {
                    self.emit(", ");
                }
                
                if param.name == "self" {
                    // Self parameter - need to determine type from context
                    // For now, assume it's the enclosing struct type
                    self.emit("void* self");
                } else if let Some(ref type_expr) = param.type_expr {
                    let c_type = self.type_to_c(type_expr)?;
                    if param.mutable {
                        self.emit(&format!("{} {}", c_type, param.name));
                    } else {
                        self.emit(&format!("const {} {}", c_type, param.name));
                    }
                }
            }
        }
        
        self.emit(")");
        Ok(())
    }
    
    fn generate_function_impl(&mut self, decl: &FunctionDecl) -> Result<()> {
        self.current_function = Some(decl.name.clone());
        
        self.generate_function_signature(decl)?;
        self.emit(" ");
        
        if let Some(ref body) = decl.body {
            self.generate_block(body)?;
        }
        
        self.emit_line("");
        self.current_function = None;
        Ok(())
    }
    
    fn generate_method_impl(&mut self, struct_name: &str, method: &MethodDecl) -> Result<()> {
        let method_name = format!("{}_{}", struct_name, method.name);
        
        // Generate method as a regular function with mangled name
        let return_type = self.type_to_c(&method.return_type)?;
        self.emit(&format!("{} {}(", return_type, method_name));
        
        // First parameter is always the struct
        let mut first = true;
        for param in &method.params {
            if !first {
                self.emit(", ");
            }
            first = false;
            
            if param.name == "self" {
                if param.mutable {
                    self.emit(&format!("{}* self", struct_name));
                } else {
                    self.emit(&format!("const {}* self", struct_name));
                }
            } else if let Some(ref type_expr) = param.type_expr {
                let c_type = self.type_to_c(type_expr)?;
                self.emit(&format!("{} {}", c_type, param.name));
            }
        }
        
        self.emit(") ");
        self.generate_block(&method.body)?;
        self.emit_line("");
        Ok(())
    }
    
    fn generate_block(&mut self, block: &Block) -> Result<()> {
        self.emit_line("{");
        self.indent();
        
        for stmt in &block.stmts {
            self.generate_stmt(stmt)?;
        }
        
        // If the block has a final expression, generate it
        if let Some(ref expr) = block.expr {
            self.generate_expr(expr)?;
            self.emit_line(";");
        }
        
        self.dedent();
        self.emit("}");
        Ok(())
    }
    
    fn generate_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match stmt {
            Stmt::Expr(expr) => {
                self.generate_expr(expr)?;
                self.emit_line(";");
            }
            Stmt::Decl { mutable, name, type_expr, value } => {
                let c_type = self.type_to_c(type_expr)?;
                if !mutable {
                    self.emit("const ");
                }
                self.emit(&format!("{} {} = ", c_type, name));
                self.generate_expr(value)?;
                self.emit_line(";");
            }
            Stmt::Block(block) => {
                self.generate_block(block)?;
                self.emit_line("");
            }
            Stmt::If { condition, then_block, else_part } => {
                self.emit("if (");
                self.generate_expr(condition)?;
                self.emit(") ");
                self.generate_block(then_block)?;
                
                if let Some(else_stmt) = else_part {
                    self.emit(" else ");
                    match &**else_stmt {
                        Stmt::If { .. } => {
                            // else if - don't add newline
                            self.generate_stmt(else_stmt)?;
                        }
                        Stmt::Block(block) => {
                            self.generate_block(block)?;
                            self.emit_line("");
                        }
                        _ => {
                            self.emit_line("{");
                            self.indent();
                            self.generate_stmt(else_stmt)?;
                            self.dedent();
                            self.emit_line("}");
                        }
                    }
                } else {
                    self.emit_line("");
                }
            }
            Stmt::While { label, condition, body } => {
                if let Some(label) = label {
                    self.emit(&format!("{}: ", label));
                }
                self.emit("while (");
                self.generate_expr(condition)?;
                self.emit(") ");
                
                self.loop_labels.push(label.clone());
                self.generate_block(body)?;
                self.loop_labels.pop();
                self.emit_line("");
            }
            Stmt::For { label, kind, body } => {
                match kind {
                    ForKind::In { mutable, var, iter } => {
                        // For-in loops are only for fixed-size arrays
                        // Generate as a regular for loop
                        self.emit("for (int _i = 0; _i < ");
                        
                        // Get array size - this is tricky without type info
                        // For now, assume we have the size
                        self.emit("/* array size */");
                        
                        self.emit("; _i++) ");
                        self.generate_block(body)?;
                        self.emit_line("");
                    }
                    ForKind::C { init, condition, update } => {
                        if let Some(label) = label {
                            self.emit(&format!("{}: ", label));
                        }
                        
                        self.emit("for (");
                        
                        if let Some(init) = init {
                            let c_type = self.type_to_c(&init.type_expr)?;
                            if !init.mutable {
                                self.emit("const ");
                            }
                            self.emit(&format!("{} {} = ", c_type, init.name));
                            self.generate_expr(&init.value)?;
                        }
                        self.emit("; ");
                        
                        if let Some(condition) = condition {
                            self.generate_expr(condition)?;
                        }
                        self.emit("; ");
                        
                        if let Some(update) = update {
                            self.generate_expr(update)?;
                        }
                        self.emit(") ");
                        
                        self.loop_labels.push(label.clone());
                        self.generate_block(body)?;
                        self.loop_labels.pop();
                        self.emit_line("");
                    }
                }
            }
            Stmt::Match { is_type, expr, arms } => {
                // Generate as switch statement
                self.emit("switch (");
                if *is_type {
                    // For type match, switch on the tag
                    self.generate_expr(expr)?;
                    self.emit(".tag");
                } else {
                    self.generate_expr(expr)?;
                }
                self.emit_line(") {");
                self.indent();
                
                for arm in arms {
                    match &arm.pattern {
                        Pattern::Literal(lit) => {
                            self.emit("case ");
                            self.generate_literal(lit)?;
                            self.emit_line(":");
                        }
                        Pattern::Type(type_expr, variant) => {
                            // Generate case for enum variant
                            if let TypeExpr::Name(enum_name, _) = type_expr {
                                self.emit(&format!("case {}_{}_TAG:", enum_name, variant));
                                self.emit_line("");
                            }
                        }
                        Pattern::Identifier(_) | Pattern::Wildcard => {
                            self.emit("default:");
                            self.emit_line("");
                        }
                    }
                    
                    self.indent();
                    match &arm.body {
                        MatchArmBody::Block(block) => {
                            for stmt in &block.stmts {
                                self.generate_stmt(stmt)?;
                            }
                        }
                        MatchArmBody::Expr(expr) => {
                            self.generate_expr(expr)?;
                            self.emit_line(";");
                        }
                    }
                    self.emit_line("break;");
                    self.dedent();
                }
                
                self.dedent();
                self.emit_line("}");
            }
            Stmt::Return(expr) => {
                self.emit("return");
                if let Some(expr) = expr {
                    self.emit(" ");
                    self.generate_expr(expr)?;
                }
                self.emit_line(";");
            }
            Stmt::Break(label) => {
                if let Some(label) = label {
                    self.emit(&format!("goto {}_break;", label));
                } else {
                    self.emit("break");
                }
                self.emit_line(";");
            }
            Stmt::Continue(label) => {
                if let Some(label) = label {
                    self.emit(&format!("goto {}_continue;", label));
                } else {
                    self.emit("continue");
                }
                self.emit_line(";");
            }
            Stmt::Labeled { label, stmt } => {
                self.emit(&format!("{}: ", label));
                self.generate_stmt(stmt)?;
            }
        }
        Ok(())
    }
    
    fn generate_expr(&mut self, expr: &Expr) -> Result<()> {
        match expr {
            Expr::Identifier(name) => {
                self.emit(name);
            }
            Expr::Literal(lit) => {
                self.generate_literal(lit)?;
            }
            Expr::Binary { op, left, right } => {
                self.emit("(");
                self.generate_expr(left)?;
                self.emit(&format!(" {} ", self.binary_op_to_c(op)));
                self.generate_expr(right)?;
                self.emit(")");
            }
            Expr::Unary { op, expr } => {
                self.emit("(");
                self.emit(self.unary_op_to_c(op));
                self.generate_expr(expr)?;
                self.emit(")");
            }
            Expr::Call { func, args } => {
                // Check if it's a method call or enum constructor call
                if let Expr::Field { expr: obj, field } = &**func {
                    // Check if obj is a Type expression (enum constructor)
                    if let Expr::Type(_) = &**obj {
                        // This is an enum constructor call like result[i32, str].err(...)
                        // The Field expression should already generate the constructor function name
                        self.generate_expr(func)?;
                        self.emit("(");
                        for (i, arg) in args.iter().enumerate() {
                            if i > 0 {
                                self.emit(", ");
                            }
                            self.generate_expr(arg)?;
                        }
                        self.emit(")");
                    } else {
                        // Method call - need to transform to function call
                        // Get the type of obj to determine the struct name
                        // For now, we'll assume the method name is Type_method
                        self.emit(&format!("/* method call: {}.{} */", "TYPE", field));
                        self.emit(&format!("{}_{}", "TYPE", field));
                        self.emit("(");
                        
                        // First argument is the object
                        self.emit("&");
                        self.generate_expr(obj)?;
                        
                        for arg in args {
                            self.emit(", ");
                            self.generate_expr(arg)?;
                        }
                        self.emit(")");
                    }
                } else {
                    self.generate_expr(func)?;
                    self.emit("(");
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            self.emit(", ");
                        }
                        self.generate_expr(arg)?;
                    }
                    self.emit(")");
                }
            }
            Expr::Index { array, index } => {
                self.generate_expr(array)?;
                self.emit("[");
                self.generate_expr(index)?;
                self.emit("]");
            }
            Expr::Field { expr, field } => {
                // Check if this is accessing a variant constructor on a type
                if let Expr::Type(type_expr) = &**expr {
                    // This is a type access like result[i32, str].err
                    // Generate the constructor function name
                    match type_expr {
                        TypeExpr::Name(name, args) => {
                            // For generic types, we need to mangle the name
                            let mangled_name = if args.is_empty() {
                                name.clone()
                            } else {
                                // Generic instantiation - mangle the name
                                let mut mangled = name.clone();
                                for arg in args {
                                    mangled.push_str("_");
                                    match arg {
                                        GenericArg::Type(t) => {
                                            mangled.push_str(&self.mangle_type(t)?);
                                        }
                                        GenericArg::Value(_) => {
                                            mangled.push_str("V");
                                        }
                                    }
                                }
                                mangled
                            };
                            // Generate constructor function name
                            self.emit(&format!("{}_{}_MAKE", mangled_name, field));
                        }
                        _ => {
                            return Err(anyhow!("Cannot access field on non-named type"));
                        }
                    }
                } else {
                    // Normal field access
                    self.generate_expr(expr)?;
                    self.emit(&format!(".{}", field));
                }
            }
            Expr::ArrayLiteral(elements) => {
                self.emit("{");
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        self.emit(", ");
                    }
                    self.generate_expr(elem)?;
                }
                self.emit("}");
            }
            Expr::StructLiteral { fields } => {
                self.emit("{");
                for (i, (name, expr)) in fields.iter().enumerate() {
                    if i > 0 {
                        self.emit(", ");
                    }
                    self.emit(&format!(".{} = ", name));
                    self.generate_expr(expr)?;
                }
                self.emit("}");
            }
            Expr::FunctionLiteral { .. } => {
                // Function literals need to be generated as separate functions
                return Err(anyhow!("Function literals not yet implemented"));
            }
            Expr::If { condition, then_expr, else_expr } => {
                self.emit("(");
                self.generate_expr(condition)?;
                self.emit(" ? ");
                self.generate_expr(then_expr)?;
                self.emit(" : ");
                self.generate_expr(else_expr)?;
                self.emit(")");
            }
            Expr::Match { .. } => {
                // Match expressions need special handling
                return Err(anyhow!("Match expressions not yet implemented"));
            }
            Expr::Sizeof(operand) => {
                self.emit("sizeof(");
                match operand {
                    SizeofOperand::Type(type_expr) => {
                        let c_type = self.type_to_c(type_expr)?;
                        self.emit(&c_type);
                    }
                    SizeofOperand::Expr(expr) => {
                        self.generate_expr(expr)?;
                    }
                }
                self.emit(")");
            }
            Expr::Cast { expr, type_expr } => {
                self.emit("((");
                let c_type = self.type_to_c(type_expr)?;
                self.emit(&c_type);
                self.emit(")");
                self.generate_expr(expr)?;
                self.emit(")");
            }
            Expr::Type(_) => {
                // Types as expressions are only used for accessing constructors/static members
                // They don't have a runtime representation
                return Err(anyhow!("Type expressions cannot be used as values"));
            }
        }
        Ok(())
    }
    
    fn generate_literal(&mut self, lit: &Literal) -> Result<()> {
        match lit {
            Literal::Int(n) => self.emit(&n.to_string()),
            Literal::Float(f) => self.emit(&format!("{}f", f)),
            Literal::Bool(b) => self.emit(if *b { "true" } else { "false" }),
            Literal::String(s) => {
                // Generate string literal as compound literal
                self.emit(&format!("(lang_str){{.data = (const uint8_t*){:?}, .len = {}}}", s, s.len()));
            }
            Literal::Nil => self.emit("(lang_nil){}"),
        }
        Ok(())
    }
    
    fn type_to_c(&self, type_expr: &TypeExpr) -> Result<String> {
        match type_expr {
            TypeExpr::Name(name, args) => {
                if args.is_empty() {
                    Ok(self.c_type_name(name))
                } else {
                    // Generic instantiation - mangle the name
                    let mut mangled = name.clone();
                    for arg in args {
                        mangled.push_str("_");
                        match arg {
                            GenericArg::Type(t) => {
                                // Simplified mangling
                                mangled.push_str(&self.mangle_type(t)?);
                            }
                            GenericArg::Value(e) => {
                                // Value parameters - use the literal value
                                mangled.push_str("V");
                            }
                        }
                    }
                    Ok(mangled)
                }
            }
            TypeExpr::Ptr { mutable, inner } => {
                let inner_type = self.type_to_c(inner)?;
                if *mutable {
                    Ok(format!("{}*", inner_type))
                } else {
                    Ok(format!("const {}*", inner_type))
                }
            }
            TypeExpr::Array { size, element } => {
                let elem_type = self.type_to_c(element)?;
                if let Some(size_expr) = size {
                    // Fixed size array
                    Ok(format!("{}[]", elem_type)) // Size goes in declaration
                } else {
                    // Indexable array (pointer)
                    Ok(format!("{}*", elem_type))
                }
            }
            TypeExpr::Function { params, return_type } => {
                // Function pointer type
                let ret = self.type_to_c(return_type)?;
                let mut param_types = Vec::new();
                for param in params {
                    if let Some(ref t) = param.type_expr {
                        param_types.push(self.type_to_c(t)?);
                    }
                }
                Ok(format!("{} (*)({})", ret, param_types.join(", ")))
            }
            TypeExpr::Never => Ok("void".to_string()),
            _ => Err(anyhow!("Cannot convert type expression to C: {:?}", type_expr)),
        }
    }
    
    fn c_type_name(&self, name: &str) -> String {
        match name {
            "u8" => "uint8_t".to_string(),
            "u16" => "uint16_t".to_string(),
            "u32" => "uint32_t".to_string(),
            "u64" => "uint64_t".to_string(),
            "i8" => "int8_t".to_string(),
            "i16" => "int16_t".to_string(),
            "i32" => "int32_t".to_string(),
            "i64" => "int64_t".to_string(),
            "int" => "lang_int".to_string(),
            "uint" => "lang_uint".to_string(),
            "float" => "lang_float".to_string(),
            "bool" => "bool".to_string(),
            "nil" => "lang_nil".to_string(),
            "str" => "lang_str".to_string(),
            _ => name.to_string(),
        }
    }
    
    fn mangle_type(&self, type_expr: &TypeExpr) -> Result<String> {
        match type_expr {
            TypeExpr::Name(name, _) => Ok(name.clone()),
            TypeExpr::Ptr { mutable, inner } => {
                let prefix = if *mutable { "Pm" } else { "Pc" };
                Ok(format!("{}{}", prefix, self.mangle_type(inner)?))
            }
            TypeExpr::Array { .. } => Ok("A".to_string()),
            _ => Ok("X".to_string()),
        }
    }
    
    fn binary_op_to_c(&self, op: &BinaryOp) -> &'static str {
        use BinaryOp::*;
        match op {
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
            Lt => "<",
            Le => "<=",
            Gt => ">",
            Ge => ">=",
            And => "&&",
            Or => "||",
            Assign => "=",
            AddAssign => "+=",
            SubAssign => "-=",
            MulAssign => "*=",
            DivAssign => "/=",
            ModAssign => "%=",
            BitAndAssign => "&=",
            BitOrAssign => "|=",
            BitXorAssign => "^=",
            ShlAssign => "<<=",
            ShrAssign => ">>=",
        }
    }
    
    fn unary_op_to_c(&self, op: &UnaryOp) -> &'static str {
        use UnaryOp::*;
        match op {
            Plus => "+",
            Minus => "-",
            BitNot => "~",
            Not => "!",
            Deref => "*",
            AddrOf => "&", // Note: source uses ^ but C uses &
        }
    }
    
    // Helper methods for output generation
    fn emit(&mut self, s: &str) {
        self.output.push_str(s);
    }
    
    fn emit_line(&mut self, s: &str) {
        for _ in 0..self.indent_level {
            self.output.push_str("    ");
        }
        if !s.is_empty() {
            self.output.push_str(s);
        }
        self.output.push('\n');
    }
    
    fn indent(&mut self) {
        self.indent_level += 1;
    }
    
    fn dedent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }
}