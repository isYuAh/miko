use std::str::FromStr;

use hyper::Method;

pub trait IntoMethods {
    fn into_methods(self) -> Vec<Method>;
}

impl IntoMethods for &str {
    fn into_methods(self) -> Vec<Method> {
        self.split(',')
            .map(|m| Method::from_str(m.trim()).unwrap())
            .collect::<Vec<_>>()
    }
}

impl IntoMethods for Vec<Method> {
    fn into_methods(self) -> Vec<Method> {
        self
    }
}

impl IntoMethods for Method {
    fn into_methods(self) -> Vec<Method> {
        vec![self]
    }
}

impl IntoMethods for &[Method] {
    fn into_methods(self) -> Vec<Method> {
        self.to_vec()
    }
}
