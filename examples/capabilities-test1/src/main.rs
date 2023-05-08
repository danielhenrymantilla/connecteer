use connecteer_capabilities::*;

fn main() {
    test();
}

pub fn test() {
    let mut base = LoggingMiddleware(Base, core::marker::PhantomData);

    let _ = base
        .send(Some(*b"hello" as [u8; 5]), unsafe {
            std::mem::transmute(())
        })
        .unwrap()
        .unwrap()
        .into_inner();
    let _ = base
        .send(Some(*b"hello" as [u8; 5]), unsafe {
            std::mem::transmute(())
        })
        .unwrap()
        .unwrap()
        .into_inner();
    let _ = base
        .send(Some(*b"hello" as [u8; 5]), unsafe {
            std::mem::transmute(())
        })
        .unwrap()
        .unwrap()
        .into_inner();
    let _ = base
        .send(Some(*b"hello" as [u8; 5]), unsafe {
            std::mem::transmute(())
        })
        .unwrap()
        .unwrap()
        .into_inner();
}

pub struct LoggingMiddleware<
    Payload: serde::de::DeserializeOwned + serde::Serialize,
    Next: Connection<Payload>,
>(Next, std::marker::PhantomData<fn() -> Payload>);

impl<Over: serde::de::DeserializeOwned + serde::Serialize, Next: Connection<Over>>
    crate::Middleware<Over> for LoggingMiddleware<Over, Next>
{
    type Message = IdentityWrapper<Over>;

    type WrapError = Self::NextWrapError;
    type UnwrapError = Self::NextUnwrapInputError;

    type NextWrapError = Next::SendError;
    type NextUnwrapError = Next::ReceiveError;
    type NextUnwrapInputError = Next::ReceiveInputError;
    type NextWrapped = Next::Wrapped;

    fn wrap<Uncallable: crate::PublicUncallable>(
        &mut self,
        msg: Over,
        _permit: Uncallable,
    ) -> Result<Option<Self::Message>, Self::WrapError> {
        println!("Wrapped message !");
        Ok(Some(IdentityWrapper(msg)))
    }

    fn unwrap<Uncallable: crate::PublicUncallable>(
        &mut self,
        msg: Self::Message,
        _permit: Uncallable,
    ) -> Result<Option<Over>, Self::UnwrapError> {
        println!("Unwrapped message !");
        Ok(Some(msg.0))
    }

    fn get_next<Uncallable: PublicUncallable>(
        &mut self,
        _pemit: Uncallable,
    ) -> &mut NextConnection<'_, Over, Self> {
        &mut self.0
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
        _err: Self::NextUnwrapError,
        _permit: Uncallable,
    ) -> Self::UnwrapError {
        unreachable!()
    }

    fn create_unwrap_error_passthrough<Uncallable: crate::PublicUncallable>(
        &mut self,
        err: Self::NextUnwrapInputError,
        _permit: Uncallable,
    ) -> Self::UnwrapError {
        err
    }
}
