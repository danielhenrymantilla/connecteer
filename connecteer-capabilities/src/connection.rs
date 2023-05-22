use crate::sealed;
use core::ops::Generator;
use serde::{de::DeserializeOwned, Serialize};

/// You can't implement this trait, you need to let the blanket implementation do its job by
/// implementing [`Middleware`](crate::Middleware) on your types
/// This type isn't used directly by the consumer, it is only used by this crate
pub trait Connection<Payload: Serialize + DeserializeOwned>: sealed::Sealed<Payload> {
    type Wrapped: Serialize + DeserializeOwned;

    type Ctx;

    type SendError;
    type ReceiveError;
    type NextError;

    type SendGen<'s, 'c, 'g>: for<'ss, 'cc> Generator<
            (&'ss mut Self, &'cc mut Self::Ctx),
            Yield = Result<Self::Wrapped, Self::SendError>,
            Return = (),
        > + 'g
    where
        Self: 's,
        Self::Ctx: 'c;
    type ReceiveGen<'s, 'c, 'g>: Generator<
            (&'s mut Self, &'c mut Self::Ctx),
            Yield = Result<Self::Wrapped, Self::ReceiveError>,
            Return = (),
        > + 'g
    where
        Self: 's,
        Self::Ctx: 'c;

    fn send<'s, 'c, 'g>(input: Payload, _: sealed::PublicUncallable) -> Self::SendGen<'s, 'c, 'g>
    where
        Self: 's,
        Self::Ctx: 'c;
    fn receive<'s, 'c, 'g>(
        output: Result<Payload, Self::NextError>,
        _: sealed::PublicUncallable,
    ) -> Self::ReceiveGen<'s, 'c, 'g>
    where
        Self::Ctx: 'c,
        Self: 's;
}
