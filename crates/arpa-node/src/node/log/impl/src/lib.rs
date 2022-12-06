use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    fold::{self, Fold},
    parse_macro_input, parse_quote,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{Comma, Semi},
    AttributeArgs, Expr, FnArg, ItemFn, Lit, NestedMeta, Pat, Stmt,
};

struct FunctionLogVisitor {
    name: Ident,
    ignore_return: bool,
    /// We don't want add log before expr stmt in sub block
    current_block_count: u32,
    /// deal with no explicitly return stmt for () as return type
    has_return_stmt: bool,
}

macro_rules! macro_error {
    ($msg:literal) => {
        quote::quote! {
            compile_error!($msg);
        }
        .into()
    };

    ($msg:literal, $span:expr) => {
        quote::quote_spanned! { $span =>
            compile_error!($msg);
        }
        .into()
    };
}

/// This attribute macro will
/// 1. Automatically log the name, input and return value of current function at debug level
///  before it returns by trying to recognize return stmt and inserting a `debug!` stmt.
/// 2. Set the name of current function as a key in `mdc` at the beginning of the function and
///  remove it before the function returns.
///
/// Notes:
/// 1. Input and return type need to implement `Debug`.
/// 2. When dealing with async function, using `#![feature(async_fn_in_trait)]` is recommended.
/// However there is an option as `log_function("ignore-return")` to ignore printing return value
///  so that it can be compatible with `#[async_trait]`(mainly for conflicting order of attribute expansion).
#[proc_macro_attribute]
pub fn log_function(attr: TokenStream, input: TokenStream) -> TokenStream {
    // ItemFn seems OK for impl function perhaps for they both have sig and block.
    let fn_decl = parse_macro_input!(input as ItemFn);
    let fn_ident = fn_decl.sig.ident.clone();
    let fn_args = fn_decl.sig.inputs.clone();
    let fn_sig = &fn_decl.sig;
    let fn_stmts = &fn_decl.block.stmts;

    let args = parse_macro_input!(attr as AttributeArgs);
    if args.len() > 1 {
        return macro_error!("Only one argument is allowed", args[1].span());
    }

    let mut ignore_return = false;
    if let Some(arg) = args.get(0) {
        match arg {
            NestedMeta::Lit(x) => match x {
                Lit::Str(x) => {
                    if format!("{}", x.token()) == "\"ignore-return\"" {
                        ignore_return = true;
                    }
                }
                _ => {
                    return macro_error!("expected string literal for logging message", x.span());
                }
            },
            _ => {
                return macro_error!("expected string literal for logging message", arg.span());
            }
        }
    }

    let mut visitor = FunctionLogVisitor {
        name: fn_ident.clone(),
        ignore_return,
        current_block_count: 0,
        has_return_stmt: false,
    };

    let args_text = generate_args_text(fn_args);

    // Use a syntax tree traversal to transform the function body.
    let mut stmts: Punctuated<Stmt, Semi> = Punctuated::new();
    for stmt in fn_stmts {
        let stmt = visitor.fold_stmt(stmt.to_owned());
        stmts.push(stmt);
    }

    let post_code = if visitor.has_return_stmt {
        TokenStream2::new()
    } else {
        let log = visitor.generate_log();
        quote! {
            let __res = "nothing";
            #log
            return;
        }
    };

    quote! {
        #fn_sig {
            log_mdc::insert("fn_name", stringify!(#fn_ident));
            #args_text
            #stmts
            #post_code
        }
    }
    .into()
}

fn generate_args_text(fn_args: Punctuated<FnArg, Comma>) -> TokenStream2 {
    let mut args = quote! {
        let mut __args: Vec<&str> = vec![];
    };
    for arg in fn_args {
        if let FnArg::Typed(a) = arg {
            let ident = match *a.pat {
                Pat::Ident(ref p) => &p.ident,
                _ => unreachable!(),
            };
            let arg_text = quote! {
                let __arg = format!("{}: {:?}", stringify!(#ident), #ident);
                __args.push(&__arg);
            };
            args.extend(arg_text);
        }
    }
    args
}

impl FunctionLogVisitor {
    fn generate_log(&self) -> TokenStream2 {
        let fn_ident = &self.name;

        let return_text = if self.ignore_return {
            quote! {
                &format!("{:?}", "ignored")
            }
        } else {
            quote! {
                &format!("{:?}", __res)
            }
        };
        quote! {
            let __log = LogModel{
                fn_name: stringify!(#fn_ident),
                fn_args: &__args,
                fn_return: #return_text,
            };
            debug!(target: stringify!(#fn_ident), "{:?}", __log);
            log_mdc::remove("fn_name");
        }
    }

    fn handle_expr_try(&mut self, e: syn::ExprTry) -> TokenStream2 {
        let expr = fold::fold_expr(self, *e.expr);
        let log = self.generate_log();
        quote!(
            match #expr {
                Ok(v) => v,
                Err(e) => {
                    let __res = Err(e.into());
                    #log
                    return __res;
                }
            }
        )
    }
}

impl Fold for FunctionLogVisitor {
    fn fold_block(&mut self, i: syn::Block) -> syn::Block {
        self.current_block_count += 1;
        let res = fold::fold_block(self, i);
        self.current_block_count -= 1;
        res
    }

    fn fold_expr(&mut self, e: Expr) -> Expr {
        match e {
            Expr::Block(_) => {
                self.current_block_count += 1;
                let res = fold::fold_expr(self, e);
                self.current_block_count -= 1;
                res
            }
            Expr::Return(e) => {
                self.has_return_stmt = true;
                let log = self.generate_log();
                if let Some(v) = e.expr {
                    let expr = fold::fold_expr(self, *v);
                    parse_quote!({
                        let __res = #expr;
                        #log
                        return __res;
                    })
                } else {
                    parse_quote!({
                        let __res = "nothing";
                        #log
                        return;
                    })
                }
            }
            Expr::Try(e) => {
                let expr_try = self.handle_expr_try(e);
                parse_quote!(
                    #expr_try
                )
            }
            _ => fold::fold_expr(self, e),
        }
    }

    fn fold_stmt(&mut self, s: Stmt) -> Stmt {
        match s {
            Stmt::Expr(e) => match e {
                // These exprs should be common for return value.
                Expr::Array(_)
                | Expr::Await(_)
                | Expr::Binary(_)
                | Expr::Call(_)
                | Expr::Closure(_)
                | Expr::Cast(_)
                | Expr::Field(_)
                | Expr::Index(_)
                | Expr::If(_)
                | Expr::Lit(_)
                | Expr::Macro(_)
                | Expr::MethodCall(_)
                | Expr::Match(_)
                | Expr::Paren(_)
                | Expr::Range(_)
                | Expr::Return(_)
                | Expr::Reference(_)
                | Expr::Repeat(_)
                | Expr::Struct(_)
                | Expr::Tuple(_)
                | Expr::Unary(_) => {
                    if self.current_block_count == 0 {
                        self.has_return_stmt = true;
                        let log = self.generate_log();
                        let expr = fold::fold_expr(self, e);
                        parse_quote!({
                            let __res = #expr;
                            #log
                            __res
                        })
                    } else {
                        fold::fold_stmt(self, Stmt::Expr(e))
                    }
                }
                Expr::Block(_) => {
                    self.current_block_count += 1;
                    let res = fold::fold_stmt(self, Stmt::Expr(e));
                    self.current_block_count -= 1;
                    res
                }
                _ => fold::fold_stmt(self, Stmt::Expr(e)),
            },
            Stmt::Semi(e, semi) => match e {
                Expr::Try(e) => {
                    let expr_try = self.handle_expr_try(e);
                    parse_quote!(
                        #expr_try;
                    )
                }
                _ => fold::fold_stmt(self, Stmt::Semi(e, semi)),
            },
            _ => fold::fold_stmt(self, s),
        }
    }
}
