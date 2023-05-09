#![no_std]
// #![warn(clippy::pedantic)]
use serde::{de::DeserializeOwned, Serialize};

mod sealed {
    pub struct PublicUncallable;

    pub trait Sealed<P> {}

    pub trait PublicUncallableSealed {}

    impl PublicUncallableSealed for PublicUncallable {}
}

/// This is the only way to actually pass a message through the whole middleware chain
pub struct Pipeline<Payload: Serialize + DeserializeOwned, C: Connection<Payload>> {
    con: C,
    _marker: core::marker::PhantomData<fn() -> Payload>,
}

impl<Payload: Serialize + DeserializeOwned, C: Connection<Payload>> Pipeline<Payload, C> {
    pub fn new(c: C) -> Self {
        Self {
            con: c,
            _marker: Default::default(),
        }
    }

    pub fn send(&mut self, message: Payload) -> Result<Option<C::Wrapped>, C::SendError> {
        self.con.send(Some(message), sealed::PublicUncallable)
    }

    pub fn receive(&mut self, message: C::Wrapped) -> Result<Option<Payload>, C::ReceiveError> {
        self.con
            .receive(Ok(Some(message)), sealed::PublicUncallable)
    }
}

/// This is a trait that prevent an "outsider" to call some methods on trait, while still allowing
/// you to implement those traits
pub trait PublicUncallable: sealed::PublicUncallableSealed {}

impl PublicUncallable for sealed::PublicUncallable {}

/// Use this when implementing the [`Middleware`](crate::Middleware) trait, since it sets the
/// appropriate generics and Associated types
pub type NextConnection<'a, Payload, This> = dyn Connection<
        <This as Middleware<Payload>>::Message,
        Wrapped = <This as Middleware<Payload>>::NextWrapped,
        SendError = <This as Middleware<Payload>>::NextWrapError,
        ReceiveError = <This as Middleware<Payload>>::NextUnwrapInputError,
        NextError = <This as Middleware<Payload>>::NextUnwrapError,
    > + 'a;

/// You can't implement this trait, you need to let the blanket implementation do its job by
/// implementing [`Middleware`](crate::Middleware) on your types
/// This type isn't used directly by the consumer, it is only used by this crate
pub trait Connection<Payload: Serialize + DeserializeOwned>: sealed::Sealed<Payload> {
    type Wrapped: Serialize + DeserializeOwned;

    type SendError;
    type ReceiveError;
    type NextError;
    fn send(
        &mut self,
        input: Option<Payload>,
        _: sealed::PublicUncallable,
    ) -> Result<Option<Self::Wrapped>, Self::SendError>;
    fn receive(
        &mut self,
        output: Result<Option<Self::Wrapped>, Self::NextError>,
        _: sealed::PublicUncallable,
    ) -> Result<Option<Payload>, Self::ReceiveError>;

    #[doc(hidden)]
    fn is_final(&self) -> bool {
        false
    }
}

pub trait Middleware<Payload: Serialize + DeserializeOwned> {
    /// This is the message type that is outputted by the middleware when sending messages (and
    /// inputted when receiving messages)
    type Message: Serialize + DeserializeOwned;
    /// The error type returned when wrapping an [`Message`](Self::Message)
    type WrapError;
    /// The error type returned when unwrapping an [`Message`](Self::Message) and also provide a
    /// way to "passthrough" errors made by middleware after them
    type UnwrapError;

    /// The error returned by the next Middleware down the chain
    ///
    /// usually it is `Next::SendError`
    type NextWrapError;
    /// The error returned by the next Middleware down the chain
    ///
    /// usually it is `Next::NextError`
    type NextUnwrapError;
    /// The error returned by the next Middleware down the chain
    ///
    /// usually it is `Next::ReceiveError`
    type NextUnwrapInputError;
    /// The [`Message`](Self::Message) type used by the next Middleware down the chain
    ///
    /// usually it is `Next::Wrapped`
    type NextWrapped: Serialize + DeserializeOwned;

    /// Transform an [`Message`](Self::Message) into a Unwrapped `Payload`
    fn unwrap<Uncallable: PublicUncallable>(
        &mut self,
        msg: Self::Message,
    ) -> Result<Option<Payload>, Self::UnwrapError>;

    /// Transform an `Payload` into a Wrapped [`Message`](Self::Message)
    fn wrap<Uncallable: PublicUncallable>(
        &mut self,
        msg: Payload,
    ) -> Result<Option<Self::Message>, Self::WrapError>;

    /// This function allows the system to bubble-up an error when wrapping a [`Message`](Self::Message)
    fn create_wrap_error<Uncallable: PublicUncallable>(
        &mut self,
        err: Self::NextWrapError,
    ) -> Self::WrapError;

    /// This function allows the system to create an error when unwrapping a [`Message`](Self::Message)
    fn create_unwrap_error<Uncallable: PublicUncallable>(
        &mut self,
        err: Self::NextUnwrapError,
    ) -> Self::UnwrapError;

    /// This function allows the system to bubble-up an error
    fn create_unwrap_error_passthrough<Uncallable: PublicUncallable>(
        &mut self,
        err: Self::NextUnwrapInputError,
    ) -> Self::UnwrapError;

    fn get_next<Uncallable: PublicUncallable>(&mut self) -> &mut NextConnection<'_, Payload, Self>;
}

impl<M: Middleware<P>, P: Serialize + DeserializeOwned> sealed::Sealed<P> for M {}

impl<M: Middleware<Payload>, Payload: Serialize + DeserializeOwned> Connection<Payload> for M {
    type Wrapped = M::NextWrapped;

    type SendError = M::WrapError;
    type ReceiveError = M::UnwrapError;
    type NextError = M::NextUnwrapInputError;

    fn send(
        &mut self,
        input: Option<Payload>,
        _: sealed::PublicUncallable,
    ) -> Result<Option<Self::Wrapped>, M::WrapError> {
        let wrapped = {
            let o: Option<_> = input
                .map(|v| self.wrap::<sealed::PublicUncallable>(v))
                .transpose()
                .map(core::option::Option::flatten)?;
            o
        };
        let next = self.get_next::<sealed::PublicUncallable>();
        if next.is_final() {
            unsafe {
                core::mem::transmute::<
                    _,
                    &mut dyn crate::Connection<
                        Wrapper<Payload>,
                        Wrapped = Payload,
                        SendError = core::convert::Infallible,
                        ReceiveError = core::convert::Infallible,
                        NextError = core::convert::Infallible,
                    >,
                >(next)
            }
            .send(
                unsafe {
                    let orig = wrapped.map(Wrapper);
                    let copy = core::mem::transmute_copy(&orig);
                    core::mem::forget(orig);
                    copy
                },
                sealed::PublicUncallable,
            )
            .map(|o| {
                o.map(|v| {
                    let copy: Self::Wrapped = unsafe { core::mem::transmute_copy(&v) };
                    core::mem::forget(v);
                    copy
                })
            })
            .map_err(|e| {
                let copy: M::WrapError = unsafe { core::mem::transmute_copy(&e) };
                copy
            })
        } else {
            next.send(wrapped, sealed::PublicUncallable)
                .map_err(|v| self.create_wrap_error::<sealed::PublicUncallable>(v))
        }
    }

    fn receive(
        &mut self,
        output: Result<Option<Self::Wrapped>, Self::NextError>,
        _: sealed::PublicUncallable,
    ) -> Result<Option<Payload>, Self::ReceiveError> {
        output
            .transpose()
            .map(|o| {
                o.map(|o| {
                    let next = self.get_next::<sealed::PublicUncallable>();
                    let is_final = next.is_final();

                    let v: Result<Option<_>, _> = if is_final {
                        let copy: Payload = unsafe { core::mem::transmute_copy(&o) };
                        core::mem::forget(o);
                        unsafe {
                            core::mem::transmute::<
                                _,
                                &mut dyn crate::Connection<
                                    Wrapper<Payload>,
                                    Wrapped = Payload,
                                    SendError = core::convert::Infallible,
                                    ReceiveError = core::convert::Infallible,
                                    NextError = core::convert::Infallible,
                                >,
                            >(next)
                        }
                        .receive(Ok(Some(copy)), sealed::PublicUncallable)
                        .map(|o| {
                            o.map(|v| {
                                let copy: M::Message = unsafe { core::mem::transmute_copy(&v) };
                                core::mem::forget(v);
                                copy
                            })
                        })
                        .map_err(|e| {
                            let copy: M::UnwrapError = unsafe { core::mem::transmute_copy(&e) };
                            copy
                        })
                    } else {
                        next.receive(Ok(Some(o)), sealed::PublicUncallable)
                            .map_err(|e| {
                                self.create_unwrap_error_passthrough::<sealed::PublicUncallable>(e)
                            })
                    };
                    v
                })
            })
            .map(|e| {
                e.map_err(|e| self.create_unwrap_error_passthrough::<sealed::PublicUncallable>(e))
            })
            .map(|e| match e {
                Ok(Ok(v)) => Ok(v),
                Ok(Err(e)) | Err(e) => Err(e),
            })
            .transpose()?
            .flatten()
            .map(|v| self.unwrap::<sealed::PublicUncallable>(v))
            .transpose()
            .map(core::option::Option::flatten)
    }
}

pub use identity::{Base, Wrapper};

mod identity {

    /// This type can be used when implementing an Middleware that doesn't modify the message, but only act with side effects
    /// (for example logging message, or filtering)
    #[repr(transparent)]
    pub struct Wrapper<T>(pub T);

    impl<T> Wrapper<T> {
        #[inline]
        pub fn into_inner(self) -> T {
            self.0
        }
    }

    impl<T: serde::Serialize> serde::Serialize for Wrapper<T> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            self.0.serialize(serializer)
        }
    }

    impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for Wrapper<T> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            T::deserialize(deserializer).map(Self)
        }
        fn deserialize_in_place<D>(deserializer: D, place: &mut Self) -> Result<(), D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            <T as serde::Deserialize>::deserialize_in_place(deserializer, &mut place.0)
        }
    }

    impl<P> crate::sealed::Sealed<Wrapper<P>> for Identity {}

    struct Identity;
    impl<Payload: crate::Serialize + crate::DeserializeOwned> crate::Connection<Wrapper<Payload>>
        for Identity
    {
        type Wrapped = Payload;

        type ReceiveError = core::convert::Infallible;
        type NextError = core::convert::Infallible;
        type SendError = core::convert::Infallible;

        fn send(
            &mut self,
            input: Option<Wrapper<Payload>>,
            _: crate::sealed::PublicUncallable,
        ) -> Result<Option<Payload>, Self::SendError> {
            Ok(input.map(Wrapper::into_inner))
        }

        fn receive(
            &mut self,
            output: Result<Option<Payload>, Self::NextError>,
            _: crate::sealed::PublicUncallable,
        ) -> Result<core::option::Option<Wrapper<Payload>>, Self::NextError> {
            Ok(unsafe { output.unwrap_unchecked().map(Wrapper) })
        }

        fn is_final(&self) -> bool {
            true
        }
    }

    /// This is the "Base" of all [`Middleware`](crate::Middleware) chain.
    /// This is the only way to have a middleware that doesn't ask for an `Next` middleware.
    pub struct Base;

    impl<M: serde::de::DeserializeOwned + serde::Serialize> crate::Middleware<M> for Base {
        type Message = Wrapper<M>;

        type WrapError = core::convert::Infallible;
        type UnwrapError = core::convert::Infallible;

        type NextUnwrapError = core::convert::Infallible;
        type NextWrapError = core::convert::Infallible;
        type NextUnwrapInputError = core::convert::Infallible;

        type NextWrapped = Wrapper<M>;

        fn wrap<Uncallable: crate::PublicUncallable>(
            &mut self,
            msg: M,
        ) -> Result<Option<Self::Message>, Self::WrapError> {
            Ok(Some(Wrapper(msg)))
        }

        fn unwrap<Uncallable: crate::PublicUncallable>(
            &mut self,
            msg: Self::Message,
        ) -> Result<Option<M>, Self::UnwrapError> {
            Ok(Some(msg.0))
        }

        fn get_next<Uncallable: crate::PublicUncallable>(
            &mut self,
        ) -> &mut dyn crate::Connection<
            Wrapper<M>,
            Wrapped = Wrapper<M>,
            SendError = core::convert::Infallible,
            ReceiveError = core::convert::Infallible,
            NextError = core::convert::Infallible,
        > {
            unsafe {
                core::mem::transmute(core::ptr::NonNull::<Identity>::dangling().as_mut()
                    as &mut dyn crate::Connection<
                        Wrapper<M>,
                        Wrapped = M,
                        SendError = core::convert::Infallible,
                        ReceiveError = core::convert::Infallible,
                        NextError = core::convert::Infallible,
                    >)
            }
        }

        fn create_wrap_error<Uncallable: crate::PublicUncallable>(
            &mut self,
            err: Self::NextWrapError,
        ) -> Self::WrapError {
            err
        }

        fn create_unwrap_error<Uncallable: crate::PublicUncallable>(
            &mut self,
            err: Self::NextUnwrapError,
        ) -> Self::UnwrapError {
            err
        }

        fn create_unwrap_error_passthrough<Uncallable: crate::PublicUncallable>(
            &mut self,
            err: Self::NextUnwrapInputError,
        ) -> Self::UnwrapError {
            err
        }
    }
}
