#![no_std]
//#![warn(clippy::pedantic)]
use serde::{de::DeserializeOwned, Serialize};

mod sealed {
    pub struct PublicUncallable;

    pub trait Sealed<P> {}

    pub trait PublicUncallableSealed {}

    impl PublicUncallableSealed for PublicUncallable {}
}

pub trait PublicUncallable: sealed::PublicUncallableSealed {}

impl PublicUncallable for sealed::PublicUncallable {}

/// This is used when creating
pub type NextConnection<'a, Over, This> = dyn Connection<
        Over,
        Wrapped = <This as Middleware<Over>>::NextWrapped,
        SendError = <This as Middleware<Over>>::NextWrapError,
        ReceiveError = <This as Middleware<Over>>::NextUnwrapError,
        ReceiveInputError = <This as Middleware<Over>>::NextUnwrapInputError,
    > + 'a;

/// You can't implement this trait, you need to let the blanket implementation do its job by
/// implementing [`Middleware`](crate::Middleware) on your types
pub trait Connection<Payload: Serialize + DeserializeOwned>: sealed::Sealed<Payload> {
    type Wrapped;

    type SendError;
    type ReceiveError;
    type ReceiveInputError;
    fn send(
        &mut self,
        input: Option<Payload>,
        _: sealed::PublicUncallable,
    ) -> Result<Option<Self::Wrapped>, Self::SendError>;
    fn receive(
        &mut self,
        output: Result<Option<Self::Wrapped>, Self::ReceiveInputError>,
        _: sealed::PublicUncallable,
    ) -> Result<Option<Payload>, Self::ReceiveError>;
}

pub trait Middleware<Over: Serialize + DeserializeOwned> {
    type Message;
    type WrapError;
    type UnwrapError;

    type NextWrapError;
    type NextUnwrapError;
    type NextUnwrapInputError;

    type NextWrapped;

    fn unwrap<Uncallable: PublicUncallable>(
        &mut self,
        msg: Self::Message,
        pemit: Uncallable,
    ) -> Result<Option<Over>, Self::UnwrapError>;

    fn wrap<Uncallable: PublicUncallable>(
        &mut self,
        msg: Over,
        pemit: Uncallable,
    ) -> Result<Option<Self::Message>, Self::WrapError>;

    fn create_wrap_error<Uncallable: PublicUncallable>(
        &mut self,
        err: Self::NextWrapError,
        pemit: Uncallable,
    ) -> Self::WrapError;
    fn create_unwrap_error<Uncallable: PublicUncallable>(
        &mut self,
        err: Self::NextUnwrapError,
        pemit: Uncallable,
    ) -> Self::UnwrapError;

    fn create_unwrap_error_passthrough<Uncallable: PublicUncallable>(
        &mut self,
        err: Self::NextUnwrapInputError,
        pemit: Uncallable,
    ) -> Self::UnwrapError;

    fn get_next<Uncallable: PublicUncallable>(
        &mut self,
        pemit: Uncallable,
    ) -> &mut NextConnection<'_, Over, Self>;
}

impl<M: Middleware<P>, P: Serialize + DeserializeOwned> sealed::Sealed<P> for M {}

impl<M: Middleware<Over>, Over: Serialize + DeserializeOwned> Connection<Over> for M {
    type Wrapped = M::Message;

    type SendError = M::WrapError;
    type ReceiveError = M::UnwrapError;
    type ReceiveInputError = M::NextUnwrapInputError;

    fn send(
        &mut self,
        input: Option<Over>,
        _: sealed::PublicUncallable,
    ) -> Result<Option<Self::Wrapped>, M::WrapError> {
        input
            .map(|v| self.wrap::<sealed::PublicUncallable>(v, sealed::PublicUncallable))
            .transpose()
            .map(core::option::Option::flatten)
    }

    fn receive(
        &mut self,
        output: Result<Option<Self::Wrapped>, Self::ReceiveInputError>,
        _: sealed::PublicUncallable,
    ) -> Result<Option<Over>, Self::ReceiveError> {
        output
            .transpose()
            .map(|o| {
                o.map_err(|e| {
                    self.create_unwrap_error_passthrough::<sealed::PublicUncallable>(
                        e,
                        sealed::PublicUncallable,
                    )
                })
                .map(|v| self.unwrap::<sealed::PublicUncallable>(v, sealed::PublicUncallable))
            })
            .map(|e| match e {
                Ok(Ok(v)) => Ok(v),
                Ok(Err(e)) | Err(e) => Err(e),
            })
            .transpose()
            .map(core::option::Option::flatten)
    }
}

pub use identity::WrappingMiddleware as Base;

#[repr(transparent)]
pub struct IdentityWrapper<T>(pub T);

impl<T> IdentityWrapper<T> {
    #[inline]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: serde::Serialize> serde::Serialize for IdentityWrapper<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for IdentityWrapper<T> {
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

mod identity {

    #[repr(transparent)]
    pub struct Wrapper<T>(T);

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
    impl<P: crate::Serialize + crate::DeserializeOwned> crate::Connection<Wrapper<P>> for Identity {
        type Wrapped = P;

        type ReceiveError = core::convert::Infallible;
        type ReceiveInputError = core::convert::Infallible;
        type SendError = core::convert::Infallible;

        fn send(
            &mut self,
            input: Option<Wrapper<P>>,
            _: crate::sealed::PublicUncallable,
        ) -> Result<Option<P>, Self::SendError> {
            Ok(input.map(Wrapper::into_inner))
        }

        fn receive(
            &mut self,
            output: Result<Option<P>, Self::ReceiveInputError>,
            _: crate::sealed::PublicUncallable,
        ) -> Result<core::option::Option<Wrapper<P>>, Self::ReceiveInputError> {
            Ok(unsafe { output.unwrap_unchecked().map(Wrapper) })
        }
    }

    pub struct WrappingMiddleware;

    impl<M: serde::de::DeserializeOwned + serde::Serialize> crate::Middleware<M>
        for WrappingMiddleware
    {
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
            _permit: Uncallable,
        ) -> Result<Option<Self::Message>, Self::WrapError> {
            Ok(Some(Wrapper(msg)))
        }

        fn unwrap<Uncallable: crate::PublicUncallable>(
            &mut self,
            msg: Self::Message,
            _permit: Uncallable,
        ) -> Result<Option<M>, Self::UnwrapError> {
            Ok(Some(msg.0))
        }

        fn get_next<Uncallable: crate::PublicUncallable>(
            &mut self,
            _permit: Uncallable,
        ) -> &mut dyn crate::Connection<
            M,
            Wrapped = Wrapper<M>,
            SendError = core::convert::Infallible,
            ReceiveError = core::convert::Infallible,
            ReceiveInputError = core::convert::Infallible,
        > {
            unsafe {
                core::mem::transmute(core::ptr::NonNull::<Identity>::dangling().as_mut()
                    as &mut dyn crate::Connection<
                        Wrapper<M>,
                        Wrapped = M,
                        SendError = core::convert::Infallible,
                        ReceiveError = core::convert::Infallible,
                        ReceiveInputError = core::convert::Infallible,
                    >)
            }
        }

        fn create_wrap_error<Uncallable: crate::PublicUncallable>(
            &mut self,
            err: Self::NextWrapError,
            _permit: Uncallable,
        ) -> Self::WrapError {
            err
        }

        fn create_unwrap_error<Uncallable: crate::PublicUncallable>(
            &mut self,
            err: Self::NextUnwrapError,
            _permit: Uncallable,
        ) -> Self::UnwrapError {
            err
        }

        fn create_unwrap_error_passthrough<Uncallable: crate::PublicUncallable>(
            &mut self,
            err: Self::NextUnwrapInputError,
            _permit: Uncallable,
        ) -> Self::UnwrapError {
            err
        }
    }
}
