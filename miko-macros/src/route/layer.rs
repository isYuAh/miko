use syn::{Expr, parse::Parse, parse::ParseStream};

/// Layer 属性信息
#[derive(Debug, Clone)]
pub struct LayerAttr {
    /// Layer 表达式，可以是任意有效的 Rust 表达式
    /// 例如: TimeoutLayer::new(Duration::from_secs(30))
    /// 或: timeout_layer()
    pub layer_expr: Expr,
}

impl Parse for LayerAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let layer_expr: Expr = input.parse()?;
        Ok(LayerAttr { layer_expr })
    }
}

/// 从函数属性中提取所有 #[layer(...)] 标记
pub fn extract_layer_attrs(attrs: &[syn::Attribute]) -> Vec<LayerAttr> {
    let mut layers = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("layer")
            && let Ok(layer_attr) = attr.parse_args::<LayerAttr>()
        {
            layers.push(layer_attr);
        }
    }
    layers
}
