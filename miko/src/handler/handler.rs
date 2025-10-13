#![allow(non_snake_case)]
use crate::handler::extractor::from_request::{FRFut, FRPFut};
use crate::handler::{extractor::from_request::FromRequest, extractor::from_request::FromRequestParts, into_response::IntoResponse};
use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use hyper::http::request::Parts;
use hyper::{Request, Response};
use std::convert::Infallible;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::util::BoxCloneService;
use tower::Service;
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
        let fut = self(req);
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
        self()
    }
}

impl<F, A> FnOnceTuple<A> for Arc<F>
where
    F: FnOnceTuple<A> + Clone,
{
    type Output = <F as FnOnceTuple<A>>::Output;

    fn call(self, args: A) -> Self::Output {
        let f: F = (&*self).clone();
        f.call(args)
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

pub struct TypedHandler<F, A, S, M> {
    pub f: F,
    pub state: Arc<S>,
    _marker: PhantomData<(A, M)>
}
impl<F, A, S, M> TypedHandler<F, A, S, M> {
    pub fn new(f: F, state: Arc<S>) -> Self {
        Self { f, state, _marker: PhantomData }
    }
}

impl<F, A, S, Fut, R, M> Handler for TypedHandler<F, A, S, M>
where
    F: FnOnceTuple<A, Output = Fut> + Clone + Send + Sync + 'static,
    A: FromRequest<S, M> + Send + 'static,
    Fut: Future<Output = R> + Send + 'static,
    R: IntoResponse,
    S: Send + Sync + 'static,
    M: Send + Sync + 'static,
{
    fn call(&self, req: Req) -> Pin<Box<dyn Future<Output = Resp> + Send>> {
        let f = self.f.clone();
        let state = self.state.clone();
        Box::pin(async move {
            let args = A::from_request(req, state.clone()).await;
            match args {
                Ok(args) => {
                    let resp = f.call(args).await;
                    resp.into_response()
                }
                Err(err) => {
                    err.into_response()
                }
            }
        })
    }
}

impl<S> FromRequestParts<S> for () 
where
    S: Send + Sync + 'static,
{
    fn from_request_parts(_req: &mut Parts, _state: Arc<S>) -> FRFut<Self> {
        Box::pin(async move { Ok(()) })
    }
}

use crate::handler::router::HttpSvc;
pub use __extract_kind::{PartsTag, ReqTag};

mod __extract_kind {
    pub enum PartsTag {}
    pub enum ReqTag {}
}

macro_rules! impl_from_request_parts_tuple {
    ($($name:ident),+) => {
        impl<S, $($name,)+> FromRequestParts<S> for ($($name,)+)
        where
            S: Send + Sync + 'static,
            $( $name: FromRequestParts<S> + Send + 'static, )+
        {
            fn from_request_parts<'a>(parts: &'a mut Parts, state: Arc<S>) -> FRPFut<'a, Self> {
                Box::pin(async move {
                    Ok((
                        $(
                            $name::from_request_parts(parts, state.clone()).await?,
                        )+
                    ))
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
            $( $name: FromRequestParts<S> + Send + 'static, )+
            $last: FromRequest<S> + Send + 'static,
        {
            fn from_request(req: Req, state: Arc<S>) -> FRFut<Self> {
                Box::pin(async move {
                    let (mut parts, body) = req.into_parts();
                    $(
                        let $name = $name::from_request_parts(&mut parts, state.clone()).await?;
                    )+
                    let req = Req::from_parts(parts, body);
                    let $last = $last::from_request(req, state).await?;
                    Ok(($($name,)+ $last,))
                })
            }
        }
    };
}

impl <S, A> FromRequest<S> for (A,) where
    S: Send + Sync + 'static,
    A: FromRequest<S> + Send + 'static,
{
    fn from_request(req: Req, state: Arc<S>) -> FRFut<Self> {
        Box::pin(async move {
            let (mut parts, body) = req.into_parts();
            let req = Req::from_parts(parts, body);
            let a = A::from_request(req, state).await?;
            Ok((a,))
        })
    }
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

pub type DynHandler = Arc<dyn Handler + Send + Sync>;

#[derive(Clone)]
pub struct HandlerSvc {
    inner: DynHandler
}
impl HandlerSvc {
    pub fn new(inner: DynHandler) -> Self {
        Self { inner }
    }
}

impl Service<Req> for HandlerSvc {
    type Response = Resp;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, req: Req) -> Self::Future {
        let h = self.inner.clone();
        Box::pin(async move {
            Ok(h.call(req).await)
        })
    }
}
pub fn handler_to_svc(h: DynHandler) -> HttpSvc<Req> {
    let svc = HandlerSvc::new(h);
    BoxCloneService::new(svc)
}