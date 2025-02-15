use crate::connection::Connection;
use core::ops::{Generator, GeneratorState};
use serde::{de::DeserializeOwned, Serialize};

fn fuck_mut<'b, T: ?Sized>(v: &mut T) -> &'_ mut T {
    unsafe { &mut *(v as *mut T) }
}

/// This is a trait that prevent an "outsider" to call some methods on trait, while still allowing
/// you to implement those traits
pub trait PublicUncallable: crate::sealed::PublicUncallableSealed {}

impl PublicUncallable for crate::sealed::PublicUncallable {}

pub trait Middleware<Payload: Serialize + DeserializeOwned> {
    /// This is the message type that is outputted by the middleware when sending messages (and
    /// inputted when receiving messages)
    type Message: Serialize + DeserializeOwned;
    /// The error type returned when wrapping an [`Message`](Self::Message)
    type WrapError;
    /// The error type returned when unwrapping an [`Message`](Self::Message) and also provide a
    /// way to "passthrough" errors made by middleware after them
    type UnwrapError;

    type Ctx;
    type Next: Connection<Self::Message>;

    type WrapGen<'s, 'c, 'g>: Generator<
            (&'s mut Self, &'c mut Self::Ctx),
            Yield = Result<Self::Message, Self::WrapError>,
            Return = (),
        > + 'g
    where
        Self::Ctx: 'c,
        Self: 's;
    type UnwrapGen<'s, 'c, 'g>: Generator<
            (&'s mut Self, &'c mut Self::Ctx),
            Yield = Result<Payload, Self::UnwrapError>,
            Return = (),
        > + 'g
    where
        Self: 's,
        Self::Ctx: 'c;
    /// Transform an [`Message`](Self::Message) into a Unwrapped `Payload`
    fn wrap<'a, 'b, 'g, Uncallable: PublicUncallable>(msg: Payload) -> Self::WrapGen<'a, 'b, 'g>;

    /// Transform an `Payload` into a Wrapped [`Message`](Self::Message)
    fn unwrap<'a, 'b, 'g, Uncallable: PublicUncallable>(
        msg: Self::Message,
    ) -> Self::UnwrapGen<'a, 'b, 'g>;

    /// This function allows the system to bubble-up an error when wrapping a [`Message`](Self::Message)
    fn create_wrap_error<Uncallable: PublicUncallable>(
        &mut self,
        err: <Self::Next as Connection<Self::Message>>::SendError,
    ) -> Self::WrapError;

    /// This function allows the system to create an error when unwrapping a [`Message`](Self::Message)
    fn create_unwrap_error<Uncallable: PublicUncallable>(
        &mut self,
        err: <Self::Next as Connection<Self::Message>>::ReceiveError,
    ) -> Self::UnwrapError;

    /// This function allows the system to bubble-up an error
    fn create_unwrap_error_passthrough<Uncallable: PublicUncallable>(
        &mut self,
        err: <Self::Next as Connection<Self::Message>>::NextError,
    ) -> Self::UnwrapError;

    fn get_next<Uncallable: PublicUncallable>(&mut self) -> &mut Self::Next;

    fn get_next_ctx<Uncallable: PublicUncallable>(
        c: &mut Self::Ctx,
    ) -> &mut <Self::Next as Connection<Self::Message>>::Ctx;
}

impl<M: Middleware<Payload>, Payload: Serialize + DeserializeOwned + 'static>
    crate::sealed::Sealed<Payload> for M
{
}

impl<M: Middleware<Payload> + 'static, Payload: Serialize + DeserializeOwned + 'static>
    Connection<Payload> for M
{
    type Ctx = M::Ctx;
    type Wrapped = <M::Next as Connection<M::Message>>::Wrapped;

    type SendError = M::WrapError;
    type ReceiveError = M::UnwrapError;
    type NextError = <M::Next as Connection<M::Message>>::ReceiveError;

    type SendGen<'s, 'c, 'g> =
        impl Generator<(&'s mut Self,&'c mut Self::Ctx), Yield = Result<Self::Wrapped, Self::SendError>, Return = ()> + 'g where Self::Ctx: 'c, Self: 's;
    type ReceiveGen<'s, 'c, 'g>=
        impl Generator<(&'s mut Self,&'c mut Self::Ctx), Yield = Result<Self::Wrapped, Self::ReceiveError>, Return = ()> + 'g where Self::Ctx: 'c, Self: 's;

    #[allow(unreachable_code)]
    fn send<'a, 'b, 'g>(
        input: Payload,
        _: crate::sealed::PublicUncallable,
    ) -> Self::SendGen<'a, 'b, 'g>
    where
        Self: 'a,
        Self::Ctx: 'b,
    {
        move |(mut s, mut ctx): (&'a mut Self, &'b mut Self::Ctx)| {
            let mut gen_ptr = M::wrap::<crate::sealed::PublicUncallable>(input);
            let _pin = core::marker::PhantomPinned;
            loop {
                match unsafe { core::pin::Pin::new_unchecked(fuck_mut(&mut gen_ptr)) }
                    .resume((fuck_mut(s), fuck_mut(ctx)))
                {
                    GeneratorState::Yielded(Ok(v)) => {
                        let mut ret = <M::Next>::send(v, crate::sealed::PublicUncallable);
                        let next = fuck_mut(s.get_next::<crate::sealed::PublicUncallable>());
                        while let GeneratorState::Yielded(v) =
                            unsafe { core::pin::Pin::new_unchecked(fuck_mut(&mut ret)) }.resume((
                                fuck_mut(next),
                                M::get_next_ctx::<crate::sealed::PublicUncallable>(fuck_mut(ctx)),
                            ))
                        {
                            let y = v.map_err(|e| {
                                s.create_wrap_error::<crate::sealed::PublicUncallable>(e)
                            });
                            let tmp = yield y;
                            s = tmp.0;
                            ctx = tmp.1;
                        }
                        continue;
                    }
                    GeneratorState::Yielded(Err(e)) => {
                        let tmp = yield Err(e);
                        s = tmp.0;
                        ctx = tmp.1;
                    }
                    GeneratorState::Complete(()) => return,
                };
            }
        }
    }

    #[allow(unreachable_code, dead_code, unused)]
    fn receive<'a, 'b, 'g>(
        output: Result<Payload, Self::NextError>,
        _: crate::sealed::PublicUncallable,
    ) -> Self::ReceiveGen<'a, 'b, 'g>
    where
        Self::Ctx: 'b,
        Self: 'a,
    {
        |(s, ctx)| {
            yield todo!();
        }
        /*
            output
                .map(|o| {
                    o.map(|o| {
                        let next = self.get_next::<sealed::PublicUncallable>();
                        let is_final = next.is_final();

                        let v: Result<Option<_>, _> = if is_final {
                            let copy: Payload = unsafe { core::mem::transmute_copy(&o) };
                            core::mem::forget(o);
                            /*unsafe {
                                core::mem::transmute::<
                                    _,
                                    &mut dyn crate::Connection<
                                        Wrapper<Payload>,
                                        (),
                                        Wrapped = Payload,
                                        SendError = core::convert::Infallible,
                                        ReceiveError = core::convert::Infallible,
                                        NextError = core::convert::Infallible,
                                    >,
                                >(next)
                            }*/
                            next.receive(Ok(Some(copy)), &mut (), sealed::PublicUncallable)
                                .map(|o| {
                                    o.map(|v| {
                                        debug_assert_eq!(
                                            core::mem::size_of_val(&v),
                                            core::mem::size_of::<Self::Wrapped>()
                                        );
                                        debug_assert_eq!(
                                            core::mem::align_of_val(&v),
                                            core::mem::align_of::<Self::Wrapped>()
                                        );
                                        let copy: M::Message = unsafe { core::mem::transmute_copy(&v) };
                                        core::mem::forget(v);
                                        copy
                                    })
                                })
                                .map_err(|e| match e {})
                        } else {
                            next.receive(Ok(Some(o)), ctx, sealed::PublicUncallable)
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
                .map(|v| self.unwrap::<sealed::PublicUncallable>(v, ctx))
                .transpose()
                .map(core::option::Option::flatten)
        */
    }
}
