#![allow(non_snake_case)]
use std::convert::Infallible;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use hyper::{Request, Response};
use hyper::http::request::Parts;

use crate::handler::extractor::from_request::FRFut;
use crate::handler::{extractor::from_request::FromRequestParts, into_response::IntoResponse, extractor::from_request::FromRequest};
pub type RespBody = BoxBody<Bytes, Infallible>;
pub type Resp = Response<RespBody>;
pub type Req = Request<RespBody>;
pub trait Handler: Send + Sync + 'static {
  fn call(&self, req: Req) -> Pin<Box<dyn Future<Output = Resp> + Send>>;
}

impl<F, Fut, Res> Handler for F
where
  F: Fn(Req) -> Fut + Send + Sync + 'static,
  Fut: Future<Output = Res> + Send + 'static,
  Res: IntoResponse,
  {
    fn call(&self, req: Req) -> Pin<Box<dyn Future<Output = Resp> + Send>> {
        let fut = (self)(req);
        Box::pin(async move { fut.await.into_response() })
    }
  }

macro_rules! impl_fn_once_tuple {
  ($($name:ident),+) => {
      impl<F, R, $($name,)+> FnOnceTuple<($($name,)+)> for F
      where
          F: FnOnce($($name),+) -> R,
      {
          type Output = R;
          fn call(self, ($($name,)+): ($($name,)+)) -> R {
              self($($name),+)
          }
      }
  };
}
pub trait FnOnceTuple<Args> {
  type Output;
  fn call(self, args: Args) -> Self::Output;
}

impl<F, R> FnOnceTuple<()> for F
where
    F: FnOnce() -> R,
{
    type Output = R;
    fn call(self, (): ()) -> R {
        (self)()
    }
}

macro_rules! impl_fn_once_tuple_all {
    () => {
        impl_fn_once_tuple!(A);
        impl_fn_once_tuple!(A, B);
        impl_fn_once_tuple!(A, B, C);
        impl_fn_once_tuple!(A, B, C, D);
        impl_fn_once_tuple!(A, B, C, D, E);
        impl_fn_once_tuple!(A, B, C, D, E, AA);
        impl_fn_once_tuple!(A, B, C, D, E, AA, G);
        impl_fn_once_tuple!(A, B, C, D, E, AA, G, H);
        impl_fn_once_tuple!(A, B, C, D, E, AA, G, H, I);
        impl_fn_once_tuple!(A, B, C, D, E, AA, G, H, I, J);
        impl_fn_once_tuple!(A, B, C, D, E, AA, G, H, I, J, K);
        impl_fn_once_tuple!(A, B, C, D, E, AA, G, H, I, J, K, L);
        impl_fn_once_tuple!(A, B, C, D, E, AA, G, H, I, J, K, L, M);
        impl_fn_once_tuple!(A, B, C, D, E, AA, G, H, I, J, K, L, M, N);
        impl_fn_once_tuple!(A, B, C, D, E, AA, G, H, I, J, K, L, M, N, O);
        impl_fn_once_tuple!(A, B, C, D, E, AA, G, H, I, J, K, L, M, N, O, P);
    };
}

impl_fn_once_tuple_all!();

// macro_rules! impl_from_request_parts_tuple {
//     ($($name:ident),+) => {
//         impl<S, $($name,)+> FromRequestParts<S> for ($($name,)+)
//         where
//             S: Send + Sync + 'static,
//             $( $name: FromRequestParts<S> + Send + 'static, )+
//         {
//             fn from_request_parts<'a>(parts: &'a Parts, state: &'a S) -> FRFut<'a, Self> {
//                 Box::pin(async move {
//                     (
//                         $(
//                             $name::from_request_parts(parts, state).await,
//                         )+
//                     )
//                 })
//             }
//         }
//     };
// }
// macro_rules! impl_from_request_tuple {
//     ($($name:ident),+; $last:ident) => {
//         impl<S, $($name,)+ $last> FromRequest<S> for ($($name,)+ $last,)
//         where
//             S: Send + Sync,
//             $( $name: FromRequestParts<S> + Send, )+
//             $last: FromRequest<S> + Send,
//         {
//             fn from_request<'a>(req: Req, state: &'a S) -> FRFut<'a, Self> {
//                 Box::pin(async move {
//                     let (mut parts, body) = req.into_parts();
//                     $(
//                         let $name = $name::from_request_parts(&mut parts, state).await;
//                     )+
//                     let req = Req::from_parts(parts, body);
//                     let $last = $last::from_request(req, state).await;
//                     ($($name,)+ $last,)
//                 })
//             }
//         }
//     };
// }

// macro_rules! impl_from_request_tuple_all {
//     () => {
//         impl_from_request_parts_tuple!(A);
//         impl_from_request_parts_tuple!(A, B);
//         impl_from_request_parts_tuple!(A, B, C);
//         impl_from_request_parts_tuple!(A, B, C, D);
//         impl_from_request_parts_tuple!(A, B, C, D, E);
//         impl_from_request_parts_tuple!(A, B, C, D, E, F);
//         impl_from_request_parts_tuple!(A, B, C, D, E, F, G);
//         impl_from_request_parts_tuple!(A, B, C, D, E, F, G, H);
//         impl_from_request_parts_tuple!(A, B, C, D, E, F, G, H, I);
//         impl_from_request_parts_tuple!(A, B, C, D, E, F, G, H, I, J);
//         impl_from_request_parts_tuple!(A, B, C, D, E, F, G, H, I, J, K);
//         impl_from_request_parts_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
//         impl_from_request_parts_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
//         impl_from_request_parts_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
//         impl_from_request_parts_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
//         impl_from_request_parts_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

//         impl_from_request_tuple!(A; B);
//         impl_from_request_tuple!(A, B; C);
//         impl_from_request_tuple!(A, B, C; D);
//         impl_from_request_tuple!(A, B, C, D; E);
//         impl_from_request_tuple!(A, B, C, D, E; F);
//         impl_from_request_tuple!(A, B, C, D, E, F; G);
//         impl_from_request_tuple!(A, B, C, D, E, F, G; H);
//         impl_from_request_tuple!(A, B, C, D, E, F, G, H; I);
//         impl_from_request_tuple!(A, B, C, D, E, F, G, H, I; J);
//         impl_from_request_tuple!(A, B, C, D, E, F, G, H, I, J; K);
//         impl_from_request_tuple!(A, B, C, D, E, F, G, H, I, J, K; L);
//         impl_from_request_tuple!(A, B, C, D, E, F, G, H, I, J, K, L; M);
//         impl_from_request_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M; N);
//         impl_from_request_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N; O);
//         impl_from_request_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O; P);
//         impl_from_request_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P; Q);
//     };
// }

// impl_from_request_tuple_all!();

pub struct TypedHandler<F, A, S> {
    pub f: F,
    pub state: Arc<S>,
    _marker: PhantomData<A>
}
impl<F, A, S> TypedHandler<F, A, S> {
    pub fn new(f: F, state: Arc<S>) -> Self {
        Self { f, state, _marker: PhantomData }
    }
}

impl<F, A, S, Fut, R> Handler for TypedHandler<F, A, S>
where
    F: FnOnceTuple<A, Output = Fut> + Clone + Send + Sync + 'static,
    A: FromRequest<S> + Send + 'static,
    Fut: Future<Output = R> + Send + 'static,
    R: IntoResponse,
    S: Send + Sync + 'static,
{
    fn call(&self, req: Req) -> Pin<Box<dyn Future<Output = Resp> + Send>> {
        let f = self.f.clone();
        let state = self.state.clone();
        Box::pin(async move {
            let args = A::from_request(req, &state).await;
            let resp = f.call(args).await;
            resp.into_response()
        })
    }
}

impl<S> FromRequest<S> for () 
where
    S: Send + Sync + 'static,
{
    fn from_request<'a>(_req: Req, _state: &'a S) -> FRFut<'a, Self> {
        Box::pin(async move { () })
    }
}
pub use __extract_kind::{PartsTag, ReqTag, IsParts, IsReq, Which};
mod __extract_kind {
    pub enum PartsTag {}
    pub enum ReqTag {}

    pub(crate) mod sealed {
        pub trait Sealed {}
        impl<T> Sealed for T {}
    }

    pub trait Which: sealed::Sealed {
        type Tag;
    }

    pub trait IsParts: Which<Tag = PartsTag> {}
    pub trait IsReq:   Which<Tag = ReqTag>   {}

    impl<T> IsParts for T where T: Which<Tag = PartsTag> {}
    impl<T> IsReq   for T where T: Which<Tag = ReqTag>   {}
}

macro_rules! impl_from_request_parts_tuple {
    ($($name:ident),+) => {
        impl<S, $($name,)+> FromRequestParts<S> for ($($name,)+)
        where
            S: Send + Sync + 'static,
            $( $name: FromRequestParts<S> + __extract_kind::IsParts + Send + 'static, )+
        {
            fn from_request_parts<'a>(parts: &'a Parts, state: &'a S) -> FRFut<'a, Self> {
                Box::pin(async move {
                    (
                        $(
                            $name::from_request_parts(parts, state).await,
                        )+
                    )
                })
            }
        }
    };
}

macro_rules! impl_from_request_tuple {
    ($($name:ident),+; $last:ident) => {
        impl<S, $($name,)+ $last> FromRequest<S> for ($($name,)+ $last,)
        where
            S: Send + Sync + 'static,
            $( $name: FromRequestParts<S> + __extract_kind::IsParts + Send + 'static, )+
            $last: FromRequest<S> + __extract_kind::IsReq + Send + 'static,
        {
            fn from_request<'a>(req: Req, state: &'a S) -> FRFut<'a, Self> {
                Box::pin(async move {
                    let (mut parts, body) = req.into_parts();
                    $(
                        let $name = $name::from_request_parts(&mut parts, state).await;
                    )+
                    let req = Req::from_parts(parts, body);
                    let $last = $last::from_request(req, state).await;
                    ($($name,)+ $last,)
                })
            }
        }
    };
}

macro_rules! impl_from_request_tuple_all {
    () => {
        impl_from_request_parts_tuple!(A);
        impl_from_request_parts_tuple!(A, B);
        impl_from_request_parts_tuple!(A, B, C);
        impl_from_request_parts_tuple!(A, B, C, D);
        impl_from_request_parts_tuple!(A, B, C, D, E);
        impl_from_request_parts_tuple!(A, B, C, D, E, F);
        impl_from_request_parts_tuple!(A, B, C, D, E, F, G);
        impl_from_request_parts_tuple!(A, B, C, D, E, F, G, H);
        impl_from_request_parts_tuple!(A, B, C, D, E, F, G, H, I);
        impl_from_request_parts_tuple!(A, B, C, D, E, F, G, H, I, J);
        impl_from_request_parts_tuple!(A, B, C, D, E, F, G, H, I, J, K);
        impl_from_request_parts_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
        impl_from_request_parts_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
        impl_from_request_parts_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
        impl_from_request_parts_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
        impl_from_request_parts_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

        impl_from_request_tuple!(A; B);
        impl_from_request_tuple!(A, B; C);
        impl_from_request_tuple!(A, B, C; D);
        impl_from_request_tuple!(A, B, C, D; E);
        impl_from_request_tuple!(A, B, C, D, E; F);
        impl_from_request_tuple!(A, B, C, D, E, F; G);
        impl_from_request_tuple!(A, B, C, D, E, F, G; H);
        impl_from_request_tuple!(A, B, C, D, E, F, G, H; I);
        impl_from_request_tuple!(A, B, C, D, E, F, G, H, I; J);
        impl_from_request_tuple!(A, B, C, D, E, F, G, H, I, J; K);
        impl_from_request_tuple!(A, B, C, D, E, F, G, H, I, J, K; L);
        impl_from_request_tuple!(A, B, C, D, E, F, G, H, I, J, K, L; M);
        impl_from_request_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M; N);
        impl_from_request_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N; O);
        impl_from_request_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O; P);
        impl_from_request_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P; Q);
    };
}

impl_from_request_tuple_all!();