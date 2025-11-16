use super::HandlerInfo;
use std::collections::HashMap;
use syn::Pat;

#[derive(Default, Debug, PartialEq, Eq, Hash, Clone)]
pub struct ReturnTypeVisitor {
    pub found_type: Option<String>,
    pub is_importable: bool,
}

impl ReturnTypeVisitor {
    fn visit_expr(&mut self, expr: &syn::Expr) {
        if self.found_type.is_some() {
            return;
        }

        match expr {
            // format::json(Type::from(...))
            syn::Expr::Call(call_expr) => {
                if let syn::Expr::Path(path_expr) = &*call_expr.func {
                    if let Some(segment) = path_expr.path.segments.last() {
                        if segment.ident == "json" {
                            // This is format::json call
                            if let Some(first_arg) = call_expr.args.first() {
                                self.visit_conversion_expr(first_arg);
                                return; // Found our type, no need to continue
                            }
                        }
                    }
                }

                // Also visit all arguments recursively
                for arg in &call_expr.args {
                    self.visit_expr(arg);
                }
            }
            // Return statements
            syn::Expr::Return(return_expr) => {
                if let Some(expr) = &return_expr.expr {
                    self.visit_expr(expr);
                }
            }
            // Method calls
            syn::Expr::MethodCall(method_call) => {
                self.visit_expr(&method_call.receiver);
                for arg in &method_call.args {
                    self.visit_expr(arg);
                }
            }
            // Block expressions (like if blocks, match arms, etc.)
            syn::Expr::Block(block_expr) => {
                for stmt in &block_expr.block.stmts {
                    self.visit_stmt(stmt);
                }
                // Check for tail expression (last statement without semicolon)
                if let Some(syn::Stmt::Expr(expr, _)) = block_expr.block.stmts.last() {
                    self.visit_expr(expr);
                }
            }
            // If expressions
            syn::Expr::If(if_expr) => {
                self.visit_expr(&if_expr.cond);
                // Visit the then branch as a block
                for stmt in &if_expr.then_branch.stmts {
                    self.visit_stmt(stmt);
                }
                // Check for tail expression in then branch
                if let Some(syn::Stmt::Expr(expr, _)) = if_expr.then_branch.stmts.last() {
                    self.visit_expr(expr);
                }
                if let Some((_, else_expr)) = &if_expr.else_branch {
                    self.visit_expr(else_expr);
                }
            }
            // Match expressions
            syn::Expr::Match(match_expr) => {
                self.visit_expr(&match_expr.expr);
                for arm in &match_expr.arms {
                    self.visit_expr(&arm.body);
                }
            }
            _ => {}
        }
    }

    fn visit_conversion_expr(&mut self, expr: &syn::Expr) {
        match expr {
            // Type::from(value)
            syn::Expr::Call(call_expr) => {
                if let syn::Expr::Path(path_expr) = &*call_expr.func {
                    let path = &path_expr.path;

                    // Handle Vec::<T>::new() pattern - convert to Array<T>
                    if let Some(vec_type) = self.extract_vec_type(path) {
                        self.found_type = Some(vec_type);
                        self.is_importable = true;
                        return;
                    }

                    // Look for patterns like "SwitchResponse::from"
                    if path.segments.len() >= 2 {
                        if let Some(segment) = path.segments.iter().nth(path.segments.len() - 2) {
                            self.found_type = Some(segment.ident.to_string());
                            self.is_importable = true; // Custom types are importable
                        }
                    } else if let Some(segment) = path.segments.last() {
                        // Direct constructor call: Type(value)
                        self.found_type = Some(segment.ident.to_string());
                        self.is_importable = true; // Custom types are importable
                    }
                }
            }
            // Method calls on variables (like value.into())
            syn::Expr::MethodCall(method_call) => {
                if method_call.method == "into" {
                    // For into(), we can't easily determine the target type
                    // Skip and let signature analysis handle it
                }
            }
            // Simple path (like a variable name)
            syn::Expr::Path(_path_expr) => {
                // If we have a simple variable, we can't determine its type
                // without type inference, so we skip it
            }
            _ => {}
        }
    }

    // In the extract_vec_type method, update to mark the inner type as importable
    fn extract_vec_type(&mut self, path: &syn::Path) -> Option<String> {
        // Look for Vec::<T>::new pattern
        if path.segments.len() >= 2 {
            if let Some(vec_segment) = path.segments.first() {
                if vec_segment.ident == "Vec" {
                    if let syn::PathArguments::AngleBracketed(generics) = &vec_segment.arguments {
                        if let Some(syn::GenericArgument::Type(syn::Type::Path(type_path))) =
                            generics.args.first()
                        {
                            if let Some(inner_segment) = type_path.path.segments.last() {
                                let inner_type = inner_segment.ident.to_string();

                                // Check if the inner type is importable (not a built-in)
                                if !ReturnTypeVisitor::is_builtin_type_rust(&inner_type) {
                                    self.is_importable = true;
                                }

                                // Convert Vec<T> to Array<T>
                                return Some(format!("Array<{}>", inner_type));
                            }
                        }
                    }
                }
            }
        }
        None
    }
    fn is_builtin_type_rust(type_name: &str) -> bool {
        matches!(
            type_name,
            "i8" | "i16"
                | "i32"
                | "i64"
                | "i128"
                | "isize"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "u128"
                | "usize"
                | "f32"
                | "f64"
                | "bool"
                | "char"
                | "str"
                | "String"
        )
    }
    fn visit_stmt(&mut self, stmt: &syn::Stmt) {
        if self.found_type.is_some() {
            return;
        }

        match stmt {
            syn::Stmt::Expr(expr, _) => {
                self.visit_expr(expr);
            }
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    self.visit_expr(&init.expr);
                }
            }
            // Skip Item and Macro statements for now
            syn::Stmt::Item(_) | syn::Stmt::Macro(_) => {}
        }
    }

    fn visit_item_fn(&mut self, func: &syn::ItemFn) {
        for stmt in &func.block.stmts {
            self.visit_stmt(stmt);
        }
        // Check for tail expression in the function block
        if let Some(syn::Stmt::Expr(expr, _)) = func.block.stmts.last() {
            self.visit_expr(expr);
        }
    }
}

/// Extract body parameter types and auth requirements from handler functions
pub fn extract_handler_info(
    syntax: &syn::File,
) -> Result<HashMap<String, HandlerInfo>, Box<dyn std::error::Error>> {
    let mut handler_info = HashMap::new();

    for item in &syntax.items {
        if let syn::Item::Fn(func) = item {
            let handler_name = func.sig.ident.to_string();
            let mut body_param = None;
            let mut requires_auth = false;
            let mut return_type = ReturnTypeVisitor::default();

            // Look for Json, JsonValidate, and JsonValidateWithMessage parameters
            for input in &func.sig.inputs {
                if let syn::FnArg::Typed(pat_type) = input {
                    // Check for body parameters (Json<T>, JsonValidate<T>, JsonValidateWithMessage<T>)
                    if let syn::Type::Path(type_path) = &*pat_type.ty {
                        if let Some(segment) = type_path.path.segments.last() {
                            let type_ident = segment.ident.to_string();

                            // Handle Json<T>, JsonValidate<T>, and JsonValidateWithMessage<T>
                            if matches!(
                                type_ident.as_str(),
                                "Json" | "JsonValidate" | "JsonValidateWithMessage"
                            ) {
                                // Extract the generic type parameter
                                if let syn::PathArguments::AngleBracketed(generics) =
                                    &segment.arguments
                                {
                                    if let Some(syn::GenericArgument::Type(syn::Type::Path(
                                        param_type,
                                    ))) = generics.args.first()
                                    {
                                        if let Some(param_segment) = param_type.path.segments.last()
                                        {
                                            body_param = Some(param_segment.ident.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Check for authentication (auth: JWT)
                    if let Pat::Ident(pat_ident) = &*pat_type.pat {
                        if pat_ident.ident == "auth" {
                            if let syn::Type::Path(type_path) = &*pat_type.ty {
                                if let Some(segment) = type_path.path.segments.last() {
                                    if segment.ident == "JWT" {
                                        requires_auth = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // First try to extract return type from function body analysis
            return_type = extract_return_type_from_body(func);

            handler_info.insert(
                handler_name,
                HandlerInfo {
                    body_param,
                    requires_auth,
                    return_type,
                },
            );
        }
    }

    Ok(handler_info)
}

/// Extract return type by analyzing the function body to find format::json calls
fn extract_return_type_from_body(func: &syn::ItemFn) -> ReturnTypeVisitor {
    let mut visitor = ReturnTypeVisitor::default();
    visitor.visit_item_fn(func);
    visitor
}
