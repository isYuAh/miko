use bytes::Bytes;
use futures::Stream;
use http_body::{Body, Frame, SizeHint};
use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct FallibleStreamBody<S> {
    stream: S,
    size_hint: SizeHint,
}

impl<S> FallibleStreamBody<S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            size_hint: SizeHint::default(),
        }
    }

    pub fn with_size_hint(stream: S, len: u64) -> Self {
        let mut hint = SizeHint::default();
        hint.set_exact(len);
        Self {
            stream,
            size_hint: hint,
        }
    }
}

impl<S, E> Body for FallibleStreamBody<S>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin,
{
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match Pin::new(&mut self.stream).poll_next(cx) {
            Poll::Ready(Some(Ok(data))) => Poll::Ready(Some(Ok(Frame::data(data)))),
            Poll::Ready(Some(Err(_e))) => {
                //ERR
                Poll::Ready(None)
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }

    fn size_hint(&self) -> SizeHint {
        self.size_hint.clone()
    }
}
