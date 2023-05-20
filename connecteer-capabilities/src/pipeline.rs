use crate::connection::Connection;
use crate::sealed::PublicUncallable;
use core::ops::{Generator, GeneratorState};
use serde::{de::DeserializeOwned, Serialize};

fn fuck_mut<'b, T: ?Sized>(v: &mut T) -> &'_ mut T {
    unsafe { &mut *(v as *mut T) }
}

/// This is the only way to actually pass a message through the whole middleware chain
pub struct Pipeline<Con: Connection<Payload>, Payload: Serialize + DeserializeOwned> {
    ctx: <Con as Connection<Payload>>::Ctx,
    con: Con,
    _marker: core::marker::PhantomData<fn() -> Payload>,
}

impl<Con: Connection<Payload>, Payload: Serialize + DeserializeOwned> Pipeline<Con, Payload> {
    pub fn new(c: Con, ctx: Con::Ctx) -> Self {
        Self {
            ctx,
            con: c,
            _marker: Default::default(),
        }
    }

    pub fn ctx(&self) -> &Con::Ctx {
        &self.ctx
    }

    pub fn ctx_mut(&mut self) -> &mut Con::Ctx {
        &mut self.ctx
    }

    pub fn send(
        &mut self,
        message: Payload,
    ) -> impl Generator<(), Yield = Result<Con::Wrapped, Con::SendError>, Return = ()> + '_ {
        let ret = Con::send(message, PublicUncallable);
        move || {
            let _pin = core::marker::PhantomPinned;
            let this = self;
            let mut ret = ret;
            while let GeneratorState::Yielded(v) =
                unsafe { core::pin::Pin::new_unchecked(fuck_mut(&mut ret)) }
                    .resume((fuck_mut(&mut this.con), fuck_mut(&mut this.ctx)))
            {
                yield v;
            }
        }
    }

    pub fn receive(
        &mut self,
        _message: Con::Wrapped,
    ) -> Result<Option<Payload>, Con::ReceiveError> {
        todo!()
        //self.con
        //    .receive(Ok(Some(message)), &mut self.ctx, sealed::PublicUncallable);
    }
}
