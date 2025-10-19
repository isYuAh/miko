use crate::toolkit::rout_arg::is_arc;
use proc_macro2::Ident;
use proc_macro2::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{FnArg, ImplItem, ImplItemFn, Pat, PatIdent};

/// 在 `impl` 项目列表中查找名为 `new` 的构造函数并返回其引用（如果存在）。
///
/// 用于在宏中检测并提取异步构造函数以进行依赖注入分析。
pub fn get_constructor(items: &Vec<ImplItem>) -> Option<&ImplItemFn> {
    for item in items {
        if let ImplItem::Fn(method) = item {
            if method.sig.ident == "new" {
                return Some(method);
            }
        }
    }
    None
}

/// 从构造函数参数列表中生成依赖注入的获取语句并收集参数标识符。
///
/// 要求构造函数参数为 `Arc<T>` 形式；该函数会为第一个依赖注入语句插入读取全局容器的代码片段，
/// 并为每个参数追加 `let <ident> = container.get::<T>().await.clone();` 之类的语句，同时收集参数名到 `arg_idents`。
pub fn inject_deps(
    args: &Punctuated<FnArg, Comma>,
    depend_get_stmts: &mut Vec<TokenStream>,
    arg_idents: &mut Vec<Ident>,
) {
    for arg in args {
        if let FnArg::Typed(pat) = arg {
            let arg_ident = match &*pat.pat {
                Pat::Ident(PatIdent { ident, .. }) => ident.clone(),
                _ => {
                    panic!("service method new argument must be Typed")
                }
            };
            let (is_arc, inner) = is_arc(&*pat.ty);
            if !is_arc {
                panic!("service method new argument must be Arc<T>")
            }
            let pat_ident = &inner.unwrap();
            if depend_get_stmts.is_empty() {
                depend_get_stmts.push(quote! {
                    let container = ::miko::dependency_container::CONTAINER.get().unwrap().read().await;
                })
            }
            depend_get_stmts.push(quote! {
                let #arg_ident = container.get::<#pat_ident>().await.clone();
            });
            arg_idents.push(arg_ident);
        }
    }
}
