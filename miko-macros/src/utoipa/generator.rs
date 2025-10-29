/// 生成 utoipa::path 宏调用
use crate::utoipa::config::OpenApiConfig;
use proc_macro2::TokenStream;
use quote::quote;

/// HTTP 方法
#[derive(Debug, Clone)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
    Trace,
}

impl HttpMethod {
    pub fn to_token(&self) -> TokenStream {
        match self {
            HttpMethod::Get => quote!(get),
            HttpMethod::Post => quote!(post),
            HttpMethod::Put => quote!(put),
            HttpMethod::Delete => quote!(delete),
            HttpMethod::Patch => quote!(patch),
            HttpMethod::Head => quote!(head),
            HttpMethod::Options => quote!(options),
            HttpMethod::Trace => quote!(trace),
        }
    }

    pub fn from_hyper_method(method: &hyper::Method) -> Self {
        match method.as_str() {
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "DELETE" => HttpMethod::Delete,
            "PATCH" => HttpMethod::Patch,
            "HEAD" => HttpMethod::Head,
            "OPTIONS" => HttpMethod::Options,
            "TRACE" => HttpMethod::Trace,
            _ => HttpMethod::Get,
        }
    }
}

/// 生成完整的 utoipa::path 宏属性
pub fn generate_utoipa_path_attr(
    method: &HttpMethod,
    path: &str,
    config: &OpenApiConfig,
) -> TokenStream {
    let method_token = method.to_token();

    // Summary
    let summary = config.final_summary().map(|s| quote!(summary = #s,));

    // Description
    let description = config
        .final_description()
        .map(|d| quote!(description = #d,));

    // Tags
    let tags = if !config.final_tags().is_empty() {
        let tag_list = config.final_tags();
        quote!(tag = #(#tag_list),*,)
    } else {
        quote!()
    };

    // Deprecated
    let deprecated = if config.deprecated {
        quote!(deprecated,)
    } else {
        quote!()
    };

    // Params
    let params = generate_params_tokens(config);

    // Request Body
    let request_body = generate_request_body_tokens(config);

    // Responses
    let responses = generate_responses_tokens(config);

    quote! {
        #[::miko::utoipa::path(
            #method_token,
            path = #path,
            #summary
            #description
            #tags
            #deprecated
            #params
            #request_body
            #responses
        )]
    }
}

/// 生成 params 部分
fn generate_params_tokens(config: &OpenApiConfig) -> TokenStream {
    let params = config.final_params();

    if params.is_empty() {
        return quote!();
    }

    let param_defs = params.iter().map(|p| {
        let name = &p.name;
        let ty = &p.ty; // 现在 ty 可能是 Option<T> 或 T
        let location = &p.location;

        let desc = p.description.as_ref().map(|d| quote!(description = #d,));

        // utoipa 会自动从类型推断 required 状态:
        // - Option<T> -> required=false, nullable=true
        // - T -> required=true
        // 我们不需要手动添加 nullable 或 required 属性

        let deprecated = if p.deprecated {
            quote!(deprecated,)
        } else {
            quote!()
        };
        let example = p.example.as_ref().map(|e| quote!(example = #e,));

        quote! {
            (#name = #ty, #location, #desc #deprecated #example)
        }
    });

    quote! {
        params(
            #(#param_defs),*
        ),
    }
}

/// 生成 request_body 部分
fn generate_request_body_tokens(config: &OpenApiConfig) -> TokenStream {
    if let Some(body) = config.final_request_body() {
        let ty = &body.ty;
        let content_type = &body.content_type;

        let desc = body.description.as_ref().map(|d| quote!(description = #d,));
        // utoipa 5 中 request_body 不支持 required 参数
        // let required = if body.required {
        //     quote!()  // 默认是 required，不需要指定
        // } else {
        //     quote!(required = false,)
        // };

        quote! {
            request_body(
                content = #ty,
                #desc
                content_type = #content_type
            ),
        }
    } else {
        quote!()
    }
}

/// 生成 responses 部分
fn generate_responses_tokens(config: &OpenApiConfig) -> TokenStream {
    let responses = config.final_responses();

    if responses.is_empty() {
        return quote!();
    }

    let resp_defs = responses.iter().map(|r| {
        let status = r.status;
        let desc = &r.description;

        if let Some(ref body) = r.body {
            if let Some(ref content_type) = r.content_type {
                quote! {
                    (status = #status, description = #desc, body = #body, content_type = #content_type)
                }
            } else {
                quote! {
                    (status = #status, description = #desc, body = #body)
                }
            }
        } else {
            quote! {
                (status = #status, description = #desc)
            }
        }
    });

    quote! {
        responses(
            #(#resp_defs),*
        ),
    }
}
