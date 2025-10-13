use std::collections::HashMap;
use syn::{Expr, ExprLit, Lit, LitStr, Meta, Token};
use syn::parse::ParseStream;

pub struct StrAttrMap {
    pub map: HashMap<String, String>,
    pub default: Option<String>,
}

impl StrAttrMap {
    pub fn from_parse_stream(input: ParseStream) -> Self {
        let mut _default = None;
        let mut map = HashMap::new();
        while !input.is_empty() {
            if input.peek(LitStr) {
                let s: LitStr = input.parse().unwrap();
                _default = Some(s.value());
            } else {
                let meta: Meta = input.parse().unwrap();
                match meta {
                    Meta::NameValue(nvmeta) => {
                        let ident = nvmeta.path.get_ident().unwrap();
                        if let Expr::Lit(ExprLit {lit: Lit::Str(str), ..}) = nvmeta.value {
                            map.insert(ident.to_string(), str.value());
                        }
                    }
                    Meta::Path(path) => {
                        let ident = path.get_ident().unwrap().to_string();
                        map.insert(ident.clone(), ident);
                    }
                    _ => {}
                }
            }
            if input.peek(Token![,]) {
                let _comma: Token![,] = input.parse().unwrap();
            }
        }
        Self {
            map,
            default: _default,
        }
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.map.get(key)
    }
    pub fn get_or_default(&self, key: &str) -> Option<String> {
        self.map.get(key).map(|s| s.to_string())
            .or(self.default.clone())
    }
}