#![feature(box_patterns)]

use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    fold::{self, Fold},
    parse_macro_input, parse_quote,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{Comma, Semi},
    AttributeArgs, Expr, FnArg, GenericParam, ItemFn, Lit, NestedMeta, Pat, PatType, Stmt,
};

struct FunctionLogVisitor {
    name: Ident,
    show_input: bool,
    ignore_input_args: Vec<String>,
    show_return: bool,
    /// support async function by `#[async_trait]`
    async_trait: bool,
    /// we don't want add log before returning in sub block or sub closure/async block
    current_block_count: i32,
    current_closure_or_async_block_count: i32,
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

#[proc_macro_attribute]
pub fn log_function(attr: TokenStream, input: TokenStream) -> TokenStream {
    // ItemFn seems OK for impl function perhaps for they both have sig and block.
    let fn_decl = parse_macro_input!(input as ItemFn);
    let fn_ident = fn_decl.sig.ident.clone();
    let fn_args = fn_decl.sig.inputs.clone();
    let fn_sig = &fn_decl.sig;
    let fn_stmts = &fn_decl.block.stmts;

    let fn_async_trait = fn_decl.sig.generics.params.iter().any(|p| match p {
        GenericParam::Lifetime(x) => x.lifetime.ident == "async_trait",
        _ => false,
    });

    let args = parse_macro_input!(attr as AttributeArgs);

    let mut show_input = false;
    let mut show_return = false;
    let mut ignore_input_args = vec![];

    for arg in args {
        match arg {
            NestedMeta::Lit(Lit::Str(x)) if x.token().to_string() == "\"show-input\"" => {
                show_input = true;
            }
            NestedMeta::Lit(Lit::Str(x)) if x.token().to_string() == "\"show-return\"" => {
                show_return = true;
            }
            NestedMeta::Lit(Lit::Str(x)) if x.token().to_string().starts_with("\"except") => {
                ignore_input_args = x
                    .token()
                    .to_string()
                    .trim_end_matches('\"')
                    .split_whitespace()
                    .skip(1)
                    .filter_map(|word| word.parse().ok())
                    .collect();
            }
            _ => {
                return macro_error!("unknown logging options", arg.span());
            }
        }
    }

    let mut visitor = FunctionLogVisitor {
        name: fn_ident.clone(),
        show_input,
        ignore_input_args,
        show_return,
        async_trait: fn_async_trait,
        current_block_count: 0,
        current_closure_or_async_block_count: 0,
        has_return_stmt: false,
    };

    let args_text = visitor.generate_args_text(fn_args);

    // Use a syntax tree traversal to transform the function body.
    let stmts: Punctuated<Stmt, Semi> = fn_stmts
        .iter()
        .map(|stmt| visitor.fold_stmt(stmt.to_owned()))
        .collect();

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

impl FunctionLogVisitor {
    fn generate_args_text(&self, fn_args: Punctuated<FnArg, Comma>) -> TokenStream2 {
        let args = quote! {
            let mut __args: Vec<String> = vec![];
        };

        fn_args
            .iter()
            .filter_map(|arg| match arg {
                FnArg::Typed(PatType {
                    attrs: _,
                    pat: box Pat::Ident(p),
                    colon_token: _,
                    ty: _,
                }) => {
                    let ident = &p.ident;
                    let arg_text = if self.show_input
                        && !self.ignore_input_args.contains(&ident.to_string())
                    {
                        quote! {
                            __args.push(format!("{}: {:?}", stringify!(#ident), #ident));
                        }
                    } else {
                        quote! {
                            __args.push(format!("{}: ignored", stringify!(#ident)));
                        }
                    };
                    Some(arg_text)
                }
                _ => None,
            })
            .fold(args, |mut args, arg| {
                args.extend(arg);
                args
            })
    }

    fn generate_log(&self) -> TokenStream2 {
        let fn_ident = &self.name;

        let return_text = if self.show_return {
            quote! {
                &format!("{:?}", __res)
            }
        } else {
            quote! {
                &format!("{:?}", "ignored")
            }
        };
        quote! {
            let ___args: Vec<&str> = __args.iter().map(|arg| arg as &str).collect();
            let __log = LogModel{
                fn_name: stringify!(#fn_ident),
                fn_args: &___args,
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

    fn insert_log_and_fold_expr_stmt(&mut self, e: Expr) -> Stmt {
        if !self.async_trait && self.current_block_count == 0 {
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
                if self.current_closure_or_async_block_count == 0 {
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
                } else {
                    fold::fold_expr(self, Expr::Return(e))
                }
            }
            Expr::Try(e) => {
                let expr_try = self.handle_expr_try(e);
                parse_quote!(
                    #expr_try
                )
            }
            // clone __args before move block
            Expr::Async(e) => {
                if e.capture.is_some() {
                    self.current_closure_or_async_block_count += 1;
                    let expr = fold::fold_expr_async(self, e);
                    self.current_closure_or_async_block_count -= 1;
                    parse_quote!({
                        let __args = __args.clone();
                        #expr
                    })
                } else {
                    fold::fold_expr(self, Expr::Async(e))
                }
            }
            // clone __args before move block
            Expr::Closure(e) => {
                if e.capture.is_some() {
                    self.current_closure_or_async_block_count += 1;
                    let expr = fold::fold_expr_closure(self, e);
                    self.current_closure_or_async_block_count -= 1;
                    parse_quote!({
                        let __args = __args.clone();
                        #expr
                    })
                } else {
                    fold::fold_expr(self, Expr::Closure(e))
                }
            }
            _ => fold::fold_expr(self, e),
        }
    }

    fn fold_stmt(&mut self, s: Stmt) -> Stmt {
        match s {
            Stmt::Expr(e) => match e {
                // ignore log on Box::pin in async_trait attribute macro
                Expr::Call(c) => match *c.func.clone() {
                    Expr::Path(p) => {
                        let first = p.path.segments.first();
                        let last = p.path.segments.last();
                        match (self.async_trait, first, last) {
                            (true, Some(f), Some(l)) if f.ident == "Box" && l.ident == "pin" => {
                                fold::fold_stmt(self, Stmt::Expr(Expr::Call(c)))
                            }
                            _ => self.insert_log_and_fold_expr_stmt(Expr::Call(c)),
                        }
                    }
                    _ => self.insert_log_and_fold_expr_stmt(Expr::Call(c)),
                },
                // log on __ret in async_trait attribute macro
                Expr::Path(p) => {
                    let ident = p.path.get_ident();
                    if self.async_trait && ident.is_some() && *ident.unwrap() == "__ret" {
                        self.has_return_stmt = true;
                        let log = self.generate_log();
                        parse_quote!({
                            let __res = __ret;
                            #log
                            __res
                        })
                    } else {
                        self.insert_log_and_fold_expr_stmt(Expr::Path(p))
                    }
                }
                // These exprs should be common for return value.
                Expr::Array(_)
                | Expr::Await(_)
                | Expr::Binary(_)
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
                | Expr::Unary(_) => self.insert_log_and_fold_expr_stmt(e),
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
