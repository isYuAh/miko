use syn::{Item, ItemFn, ItemMod, LitStr, parse::Parse};

use crate::StrAttrMap;

#[derive(Clone)]
pub enum TransformOp {
    /// 添加路径前缀
    Prefix(String),
    /// 添加 layer
    Layer(String),
}

/// Prefix 属性解析
#[derive(Debug)]
pub struct PrefixAttr {
    pub path: String,
}

impl Parse for PrefixAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let path_lit: LitStr = input.parse()?;
        Ok(PrefixAttr {
            path: path_lit.value(),
        })
    }
}

/// Layer 属性解析
#[derive(Debug)]
pub struct ModLayerAttr {
    pub expr: String,
}

impl Parse for ModLayerAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let expr_tokens: proc_macro2::TokenStream = input.parse()?;
        Ok(ModLayerAttr {
            expr: expr_tokens.to_string(),
        })
    }
}

pub fn apply_transform_to_module(mod_item: &mut ItemMod, op: TransformOp) {
    if let Some((_, items)) = &mut mod_item.content {
        for item in items.iter_mut() {
            match item {
                Item::Fn(f) => {
                    apply_transform_to_fn(f, &op);
                }
                Item::Mod(m) => {
                    apply_transform_to_submodule(m, &op);
                }
                _ => {}
            }
        }
    }
}
static ROUTE_MACROS: &[&str] = &[
    "get", "post", "put", "delete", "patch", "head", "options", "route",
];
fn apply_transform_to_fn(func: &mut ItemFn, op: &TransformOp) {
    match op {
        TransformOp::Prefix(prefix) => {
            for attr in &mut func.attrs {
                if let Some(ident) = attr.path().get_ident() {
                    let attr_name = ident.to_string();
                    if ROUTE_MACROS.contains(&attr_name.as_str())
                        && let Ok(mut attr_map) = attr.parse_args::<StrAttrMap>()
                    {
                        let original_path = attr_map
                            .get_or_default("path")
                            .or_else(|| attr_map.default.clone())
                            .unwrap_or_default();
                        let new_path = if original_path.is_empty() {
                            prefix.clone()
                        } else {
                            format!("{}{}", prefix, original_path)
                        };
                        if attr_map.map.contains_key("path") {
                            attr_map.map.insert("path".to_string(), new_path);
                        } else if attr_map.default.is_some() {
                            attr_map.default = Some(new_path);
                        }

                        let new_tokens = attr_map.to_token_stream();
                        let path = attr.path();
                        *attr = syn::parse_quote! {
                            #[#path(#new_tokens)]
                        };
                    }
                }
            }
        }
        TransformOp::Layer(layer_expr) => {
            let layer_tokens: proc_macro2::TokenStream = layer_expr.parse().unwrap_or_default();
            func.attrs.push(syn::parse_quote! {
                #[layer(#layer_tokens)]
            });
        }
    }
}

fn apply_transform_to_submodule(mod_item: &mut ItemMod, op: &TransformOp) {
    match op {
        TransformOp::Prefix(prefix) => {
            for attr in &mut mod_item.attrs {
                if attr.path().is_ident("prefix") {
                    let prefix_attr = attr.parse_args::<PrefixAttr>().unwrap();
                    let combined_prefix = format!("{}{}", prefix, prefix_attr.path);
                    *attr = syn::parse_quote! {
                        #[prefix(#combined_prefix)]
                    };
                    return;
                }
            }
            mod_item.attrs.push(syn::parse_quote! {
                #[prefix(#prefix)]
            });
        }
        TransformOp::Layer(layer_expr) => {
            let layer_tokens: proc_macro2::TokenStream = layer_expr.parse().unwrap_or_default();
            mod_item.attrs.push(syn::parse_quote! {
                #[layer(#layer_tokens)]
            });
        }
    }
}
