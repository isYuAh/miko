use crate::extractor::body::deal_with_body_attr;
use crate::extractor::path::deal_with_path_attr;
use crate::route::{RouteAttr, build_register_expr};
use crate::toolkit::exactors::build_struct_from_query;
use crate::toolkit::rout_arg::{FnArgResult, IntoFnArgs, RouteFnArg, build_dep_injector, is_arc, build_config_value_injector};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{ItemFn, Stmt, parse_quote};

pub fn route_handler(args: RouteAttr, mut fn_item: ItemFn) -> TokenStream {
    let fn_name = fn_item.sig.ident.clone();
    // 自动返回值
    let sig = &mut fn_item.sig;
    if matches!(sig.output, syn::ReturnType::Default) {
        (*sig).output = parse_quote!(-> impl ::miko::handler::into_response::IntoResponse)
    }
    let mut inject_segs: Vec<Stmt> = Vec::new();
    let rfa = RouteFnArg::from_punctuated(&mut sig.inputs);
    //处理路由
    let path_inputs = rfa.gen_fn_args(deal_with_path_attr);
    //处理body
    let body_inputs = rfa.gen_fn_args(deal_with_body_attr);
    let plain_inputs = rfa.gen_fn_args(|rfa| {
        if rfa.mark.is_empty() {
            FnArgResult::Keep
        } else {
            FnArgResult::Remove
        }
    });
    //处理dep
    let mut dep_stmts = Vec::new();
    build_dep_injector(&rfa, &mut dep_stmts);
    #[cfg(feature = "auto")]
    let dep_stmts = if dep_stmts.is_empty() {
        dep_stmts
    } else {
        dep_stmts.insert(
            0,
            quote! {
                let __dep_container = ::miko::dep::get_global_dc().await;
            },
        );
        dep_stmts
    };
    // 处理config_value
    let mut config_value_stmts = Vec::new();
    build_config_value_injector(&rfa, &mut config_value_stmts);
    // 清空参数
    sig.inputs.clear();
    // 获取无修饰参数
    // 组装path
    sig.inputs.extend(path_inputs);
    // 构建 Query 结构体和解构提取器
    let q_struct_ident = Ident::new(
        &format!("__{}_QueryStruct", fn_name.to_string()),
        Span::call_site(),
    );
    // 重组Query
    let (q_struct, q_struct_exactor) = build_struct_from_query(&rfa, q_struct_ident);
    if q_struct.is_some() {
        sig.inputs.push(q_struct_exactor.unwrap());
    }
    // 组装plain_inputs
    sig.inputs.extend(plain_inputs);
    // 最后组装body
    sig.inputs.extend(body_inputs);
    // 展开
    let user_stmts = &fn_item.block.stmts.clone();
    let inventory_collect: Option<proc_macro2::TokenStream> = if cfg!(feature = "auto") {
        Some(build_register_expr(&args, &fn_name.clone()))
    } else {
        None
    };
    quote! {
      #q_struct

      #sig {
        #(#inject_segs)*
        #(#dep_stmts)*
        #(#config_value_stmts)*
        #(#user_stmts)*
      }

      #inventory_collect

    }
    .into()
}
