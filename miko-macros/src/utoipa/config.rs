/// OpenAPI 配置相关的数据结构
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Expr, Type};

/// 参数配置
#[derive(Debug, Clone)]
pub struct ParamConfig {
    pub name: String,
    pub ty: Type,
    pub location: ParamLocation,
    pub description: Option<String>,
    /// required 字段保留用于记录语义,实际由 utoipa 从类型自动推断
    #[allow(dead_code)]
    pub required: bool,
    pub deprecated: bool,
    pub example: Option<Expr>,
}

/// 参数位置
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum ParamLocation {
    Path,
    Query,
    Header,
    Cookie,
}

impl ToTokens for ParamLocation {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let t = match self {
            ParamLocation::Path => quote::quote!(Path),
            ParamLocation::Query => quote::quote!(Query),
            ParamLocation::Header => quote::quote!(Header),
            ParamLocation::Cookie => quote::quote!(Cookie),
        };
        tokens.extend(t);
    }
}

/// 响应配置
#[derive(Debug, Clone)]
pub struct ResponseConfig {
    pub status: u16,
    pub description: String,
    pub body: Option<Type>,
    pub content_type: Option<String>,
}

/// 完整的 OpenAPI 配置
#[derive(Debug, Default)]
pub struct OpenApiConfig {
    /// 用户显式提供的 summary
    pub user_summary: Option<String>,
    /// 用户显式提供的 description
    pub user_description: Option<String>,
    /// 用户显式提供的 tags
    pub user_tags: Vec<String>,
    /// 用户显式提供的参数配置
    pub user_params: Vec<ParamConfig>,
    /// 用户显式提供的响应配置
    pub user_responses: Vec<ResponseConfig>,
    /// 用户显式提供的请求体配置
    pub user_request_body: Option<RequestBodyConfig>,
    /// 是否弃用
    pub deprecated: bool,

    // 自动推断的信息
    /// 从文档注释提取的 summary
    pub auto_summary: Option<String>,
    /// 从文档注释提取的 description
    pub auto_description: Option<String>,
    /// 从函数参数推断的参数配置
    pub auto_params: Vec<ParamConfig>,
    /// 从返回类型推断的成功响应
    pub auto_response: Option<ResponseConfig>,
    /// 从 #[body] 参数推断的请求体
    pub auto_request_body: Option<RequestBodyConfig>,
}

/// 请求体配置
#[derive(Debug, Clone)]
pub struct RequestBodyConfig {
    pub ty: Type,
    pub description: Option<String>,
    /// required 字段保留用于记录语义
    #[allow(dead_code)]
    pub required: bool,
    pub content_type: String,
}

impl OpenApiConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取最终的 summary（用户配置优先）
    pub fn final_summary(&self) -> Option<&str> {
        self.user_summary
            .as_deref()
            .or(self.auto_summary.as_deref())
    }

    /// 获取最终的 description（优先用户配置，否则用自动提取）
    pub fn final_description(&self) -> Option<&str> {
        self.user_description
            .as_deref()
            .or(self.auto_description.as_deref())
    }

    /// 获取最终的 tags
    pub fn final_tags(&self) -> &[String] {
        &self.user_tags
    }

    /// 合并参数：用户参数可以覆盖自动推断的参数
    pub fn final_params(&self) -> Vec<ParamConfig> {
        let mut params = self.auto_params.clone();

        // 用户定义的参数覆盖自动推断的
        for user_param in &self.user_params {
            if let Some(pos) = params.iter().position(|p| p.name == user_param.name) {
                params[pos] = user_param.clone();
            } else {
                params.push(user_param.clone());
            }
        }

        params
    }

    /// 合并响应：自动推断的 200 响应 + 用户定义的其他响应
    pub fn final_responses(&self) -> Vec<ResponseConfig> {
        let mut responses = Vec::new();

        // 添加自动推断的成功响应
        if let Some(ref auto_resp) = self.auto_response {
            responses.push(auto_resp.clone());
        }

        // 添加用户定义的响应
        responses.extend(self.user_responses.iter().cloned());

        responses
    }

    /// 获取最终的请求体配置（用户配置优先）
    pub fn final_request_body(&self) -> Option<&RequestBodyConfig> {
        self.user_request_body
            .as_ref()
            .or(self.auto_request_body.as_ref())
    }
}
