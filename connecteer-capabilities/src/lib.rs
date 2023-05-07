#![no_std]
//#![warn(clippy::pedantic)]
use serde::{de::DeserializeOwned, Serialize};

mod sealed {
    pub struct PublicUncallable;

    pub trait Sealed<P> {}
}

pub fn test() {
    let mut base = Base;

    let res = <dyn Connection<
        [u8; 5],
        core::convert::Infallible,
        core::convert::Infallible,
        (),
        Wrapped = identity::Wrapper<[u8; 5]>,
        WrappedSendError = _,
        WrappedReceiveError = _,
    >>::send(
        &mut base,
        Some(*b"hello" as [u8; 5]),
        sealed::PublicUncallable,
    )
    .unwrap()
    .unwrap();

    let mut base2 = Base;

    let res2 = <dyn Connection<
        [u8; 5],
        core::convert::Infallible,
        core::convert::Infallible,
        (),
        Wrapped = identity::Wrapper<[u8; 5]>,
        WrappedSendError = _,
        WrappedReceiveError = _,
    >>::send(
        &mut base2,
        Some(*b"hello" as [u8; 5]),
        sealed::PublicUncallable,
    )
    .unwrap()
    .unwrap();

    core::mem::drop((base, base2));

    let s = core::str::from_utf8(&*res).unwrap();

    assert_eq!(s, "hello");
}

/// You can't implement this trait, you need to let the blanket implementation do its job by
/// implementing [`Middleware`](crate::Middleware) on your types
pub trait Connection<
    Payload: Serialize + DeserializeOwned,
    SendError,
    ReceiveError,
    ReceiveInputError,
>: sealed::Sealed<Payload>
{
    type Wrapped;

    type WrappedSendError;
    type WrappedReceiveError;

    fn send(
        &mut self,
        input: Option<Payload>,
        _: sealed::PublicUncallable,
    ) -> Result<Option<Self::Wrapped>, Self::WrappedSendError>;
    fn receive(
        &mut self,
        output: Result<Option<Self::Wrapped>, ReceiveInputError>,
        _: sealed::PublicUncallable,
    ) -> Result<Option<Payload>, Self::WrappedReceiveError>;
}

pub trait Middleware<Over: Serialize + DeserializeOwned> {
    type Message<T>: Serialize + DeserializeOwned
    where
        T: Serialize + DeserializeOwned;

    type WrapError<Inner>;
    type UnwrapError<Inner, Passthrough>;

    fn unwrap<Err, Passthrough>(
        &mut self,
        msg: Self::Message<Over>,
    ) -> Result<Option<Over>, Self::UnwrapError<Err, Passthrough>>;

    fn wrap<Err>(&mut self, msg: Over)
        -> Result<Option<Self::Message<Over>>, Self::WrapError<Err>>;

    fn create_wrap_error<Err>(&mut self, err: Err) -> Self::WrapError<Err>;
    fn create_unwrap_error<Err, ReceiveInputError>(
        &mut self,
        err: Err,
    ) -> Self::UnwrapError<Err, ReceiveInputError>;

    fn create_unwrap_error_passthrough<Err, ReceiveInputError>(
        &mut self,
        err: ReceiveInputError,
    ) -> Self::UnwrapError<Err, ReceiveInputError>;

    fn get_next<SendError, ReceiveError, ReceiveInputError>(
        &mut self,
    ) -> &mut dyn Connection<
        Self::Message<Over>,
        SendError,
        ReceiveError,
        ReceiveInputError,
        Wrapped = Self::Message<Over>,
        WrappedSendError = Self::WrapError<SendError>,
        WrappedReceiveError = Self::UnwrapError<ReceiveError, ReceiveInputError>,
    >;
}

impl<M: Middleware<P>, P: Serialize + DeserializeOwned> sealed::Sealed<P> for M {}

impl<
        M: Middleware<Over>,
        Over: Serialize + DeserializeOwned,
        SendError,
        ReceiveError,
        ReceiveInputError,
    > Connection<Over, SendError, ReceiveError, ReceiveInputError> for M
{
    type Wrapped = M::Message<Over>;

    type WrappedSendError = M::WrapError<SendError>;
    type WrappedReceiveError = M::UnwrapError<ReceiveError, ReceiveInputError>;

    fn send(
        &mut self,
        input: Option<Over>,
        _: sealed::PublicUncallable,
    ) -> Result<Option<Self::Wrapped>, Self::WrappedSendError> {
        input
            .map(|v| self.wrap(v))
            .transpose()
            .map(core::option::Option::flatten)
    }

    fn receive(
        &mut self,
        output: Result<Option<Self::Wrapped>, ReceiveInputError>,
        _: sealed::PublicUncallable,
    ) -> Result<Option<Over>, Self::WrappedReceiveError> {
        output
            .transpose()
            .map(|o| {
                o.map_err(|e| {
                    self.create_unwrap_error_passthrough::<ReceiveError, ReceiveInputError>(e)
                })
                .map(|v| self.unwrap::<ReceiveError, ReceiveInputError>(v))
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
mod identity {
    use core::ops::{Deref, DerefMut};

    #[repr(transparent)]
    pub struct Wrapper<T>(T);

    impl<T> Deref for Wrapper<T> {
        type Target = T;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<T> DerefMut for Wrapper<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl<T> AsRef<T> for Wrapper<T> {
        fn as_ref(&self) -> &T {
            &self.0
        }
    }

    impl<T> AsMut<T> for Wrapper<T> {
        fn as_mut(&mut self) -> &mut T {
            &mut self.0
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
    impl<
            P: crate::Serialize + crate::DeserializeOwned,
            SendError,
            ReceiveInputError,
            ReceiveError,
        > crate::Connection<Wrapper<P>, SendError, ReceiveError, ReceiveInputError> for Identity
    {
        type Wrapped = Wrapper<P>;
        type WrappedSendError = SendError;
        type WrappedReceiveError = ReceiveInputError;
        fn send(
            &mut self,
            input: Option<Wrapper<P>>,
            _: crate::sealed::PublicUncallable,
        ) -> Result<Option<Self::Wrapped>, Self::WrappedSendError> {
            Ok(input)
        }

        fn receive(
            &mut self,
            output: Result<Option<Self::Wrapped>, ReceiveInputError>,
            _: crate::sealed::PublicUncallable,
        ) -> Result<Option<Wrapper<P>>, Self::WrappedReceiveError> {
            output
        }
    }

    pub struct WrappingMiddleware;

    impl<M: serde::de::DeserializeOwned + serde::Serialize> crate::Middleware<M>
        for WrappingMiddleware
    {
        type Message<T> = Wrapper<T> where T: serde::Serialize + serde::de::DeserializeOwned;

        type WrapError<Inner> = Inner;
        type UnwrapError<Inner, Passthrough> = Passthrough;

        fn wrap<Err>(&mut self, msg: M) -> Result<Option<Self::Message<M>>, Self::WrapError<Err>> {
            Ok(Some(Wrapper(msg)))
        }

        fn unwrap<Err, Passthrough>(
            &mut self,
            msg: Self::Message<M>,
        ) -> Result<Option<M>, Self::UnwrapError<Err, Passthrough>> {
            Ok(Some(msg.0))
        }

        fn get_next<SendError, ReceiveError, ReceiveInputError>(
            &mut self,
        ) -> &mut dyn crate::Connection<
            Wrapper<M>,
            SendError,
            ReceiveError,
            ReceiveInputError,
            Wrapped = Self::Message<M>,
            WrappedSendError = Self::WrapError<SendError>,
            WrappedReceiveError = Self::UnwrapError<ReceiveError, ReceiveInputError>,
        > {
            (unsafe { core::ptr::NonNull::<Identity>::dangling().as_mut() })
                as &mut dyn crate::Connection<
                    Wrapper<M>,
                    SendError,
                    ReceiveError,
                    ReceiveInputError,
                    Wrapped = Self::Message<M>,
                    WrappedSendError = Self::WrapError<SendError>,
                    WrappedReceiveError = Self::UnwrapError<ReceiveError, ReceiveInputError>,
                >
        }

        fn create_wrap_error<Err>(&mut self, err: Err) -> Self::WrapError<Err> {
            err
        }

        fn create_unwrap_error<Err, ReceiveInputError>(
            &mut self,
            _err: Err,
        ) -> Self::UnwrapError<Err, ReceiveInputError> {
            unreachable!()
        }

        fn create_unwrap_error_passthrough<Err, ReceiveInputError>(
            &mut self,
            err: ReceiveInputError,
        ) -> Self::UnwrapError<Err, ReceiveInputError> {
            err
        }
    }
}
