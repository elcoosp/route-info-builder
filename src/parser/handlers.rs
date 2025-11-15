use super::HandlerInfo;
use std::collections::HashMap;
use syn::{Pat, ReturnType, visit::Visit};

#[derive(Default)]
struct ReturnTypeVisitor {
    found_type: Option<String>,
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
                    // Look for patterns like "SwitchResponse::from"
                    if path.segments.len() >= 2 {
                        if let Some(segment) = path.segments.iter().nth(path.segments.len() - 2) {
                            self.found_type = Some(segment.ident.to_string());
                        }
                    } else if let Some(segment) = path.segments.last() {
                        // Direct constructor call: Type(value)
                        self.found_type = Some(segment.ident.to_string());
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
            let mut return_type = None;

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

            // If body analysis didn't work, fall back to signature analysis
            if return_type.is_none() {
                if let ReturnType::Type(_, return_ty) = &func.sig.output {
                    return_type = extract_return_type(return_ty);
                }
            }

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

fn extract_return_type(return_ty: &syn::Type) -> Option<String> {
    // Check guarded patterns first
    if let syn::Type::Path(type_path) = return_ty {
        if is_result_type(type_path) {
            return extract_type_from_generic(type_path, 0); // First type parameter is Ok type
        }
        if is_json_type(type_path) {
            return extract_type_from_generic(type_path, 0); // The type inside Json
        }
    }

    match return_ty {
        // Direct type like -> SwitchResponse
        syn::Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                Some(segment.ident.to_string())
            } else {
                None
            }
        }
        // impl IntoResponse or other complex types - we can't easily determine
        syn::Type::ImplTrait(_) => None,
        _ => None,
    }
}

/// Extract return type by analyzing the function body to find format::json calls
fn extract_return_type_from_body(func: &syn::ItemFn) -> Option<String> {
    let mut visitor = ReturnTypeVisitor::default();
    visitor.visit_item_fn(func);
    visitor.found_type
}

/// Check if a type path is Result<T, E>
fn is_result_type(type_path: &syn::TypePath) -> bool {
    if let Some(segment) = type_path.path.segments.last() {
        segment.ident == "Result"
    } else {
        false
    }
}

/// Check if a type path is Json<T>
fn is_json_type(type_path: &syn::TypePath) -> bool {
    if let Some(segment) = type_path.path.segments.last() {
        segment.ident == "Json"
    } else {
        false
    }
}

/// Extract type from generic parameters at the given index
fn extract_type_from_generic(type_path: &syn::TypePath, index: usize) -> Option<String> {
    if let Some(segment) = type_path.path.segments.last() {
        if let syn::PathArguments::AngleBracketed(generics) = &segment.arguments {
            if let Some(syn::GenericArgument::Type(syn::Type::Path(param_type))) =
                generics.args.iter().nth(index)
            {
                if let Some(param_segment) = param_type.path.segments.last() {
                    return Some(param_segment.ident.to_string());
                }
            }
        }
    }
    None
}
