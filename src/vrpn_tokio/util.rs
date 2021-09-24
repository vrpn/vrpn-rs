// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use std::task::Poll;

use futures::{Future, Stream};
use tokio::prelude::*;

/// Pull as many items from the stream as possible until an error, end of stream, or NotReady.
pub fn drain_stream<T: Stream>(stream: &mut T) -> Poll<(), T::Error> {
    drain_poll_fn(|| stream.poll())
}

/// Pull as many items from the poll function as possible until an error, end of stream, or NotReady.
pub fn drain_poll_fn<F, T, E>(mut func: F) -> Poll<(), E>
where
    F: FnMut() -> Poll<Option<T>, E>,
{
    loop {
        match ready!(func()) {
            Some(_) => {}
            None => {
                return Ok(Poll::Ready(()));
            }
        }
    }
}

pub trait StreamExtras: Stream + Sized {
    //fn drain(self) -> Drain<Self>;
    fn drain(self) -> Drain<Self> {
        Drain::new(self)
    }
}
impl<S> StreamExtras for S where S: Stream + Sized {}

#[derive(Debug)]
pub struct Drain<S>
where
    S: Stream + Sized,
{
    inner: Option<S>,
}

impl<S> Drain<S>
where
    S: Stream + Sized,
{
    pub fn new(stream: S) -> Drain<S> {
        Drain {
            inner: Some(stream),
        }
    }
}
impl<S> Future for Drain<S>
where
    S: Stream + Sized,
{
    fn poll(&mut self) -> Poll<Result<(), S::Error>> {
        let inner = self.inner.take();
        if let Some(mut stream) = inner {
            loop {
                match stream.poll()? {
                    Async::Ready(Some(_)) => {}
                    Async::Ready(None) => {
                        return Ok(Async::Ready(()));
                    }
                    Async::NotReady => {
                        break;
                    }
                }
            }
            self.inner = Some(stream);
        }
        Ok(Async::NotReady)
    }

    type Output = Result<(), S::Error>;
}

/// Evaluates the expression (`returning Poll<Option<_>, _>`) in a loop,
/// discarding values, until either `Ok(Async::Ready(None))` is returned
/// (indicating end of stream, and making the whole expression evalute to `Async::Ready(())`),
/// `Ok(Async::NotReady)` is returned (making the whole expression evalute to `Async::NotReady`),
/// or an error is returned (causing a return statement to be executed).
#[macro_export]
macro_rules! try_drain {
    ($e: expr) => {
        loop {
            match $e {
                Ok(Async::Ready(Some(_))) => {}
                Ok(Async::Ready(None)) => {
                    break Async::Ready(());
                }
                Ok(Async::NotReady) => {
                    break Async::NotReady;
                }
                Err(e) => return Err(From::from(e)),
            }
        }
    };
}

/// Evaluates the expression (returning `Poll<Option<_>, _>`) in a loop,
/// discarding values, until either `Ok(Async::Ready(None))` is returned
/// (indicating end of stream, and returning `Ok(Async::Ready(Default::default()))`),
/// `Ok(Async::NotReady)` is returned (making the whole expression evalute to `Async::NotReady`),
/// or an error is returned (causing a return statement to be executed).
///
/// This is useful for things
#[macro_export]
macro_rules! try_drain_return_on_ready {
    ($e: expr) => {
        loop {
            match $e {
                Ok(Async::Ready(Some(_))) => {}
                Ok(Async::Ready(None)) => {
                    return Ok(Async::Ready(Default::default()));
                }
                Ok(Async::NotReady) => {
                    break Async::NotReady;
                }
                Err(e) => return Err(From::from(e)),
            }
        }
    };
}
