use connecteer_capabilities::*;

fn main() {
    test();
}

pub fn test() {
    let mut base =
        Pipeline::<String, _>::new(id::IdMiddleware::new(log::LoggingMiddleware::new(Base)));
    //let mut base = log::LoggingMiddleware::new(Base);
    for line in std::io::stdin().lines() {
        let line = line.unwrap();
        let _ = base.receive(Wrapper(Wrapper(id::MessageWithId::msg(line))));
        let _ = base
            .send("hello world!".to_string())
            .unwrap()
            .unwrap()
            .into_inner()
            .0
            .get();
    }
}

mod id {
    struct Ctx {
        current_id: usize,
    }

    pub struct IdMiddleware<
        Payload: serde::de::DeserializeOwned + serde::Serialize,
        Next: crate::Connection<MessageWithId<Payload>>,
    >(Next, Ctx, std::marker::PhantomData<fn() -> Payload>);

    impl<
            Payload: serde::de::DeserializeOwned + serde::Serialize,
            Next: crate::Connection<MessageWithId<Payload>>,
        > IdMiddleware<Payload, Next>
    {
        pub fn new(next: Next) -> Self {
            Self(next, Ctx { current_id: 0 }, std::marker::PhantomData)
        }
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct MessageWithId<M>(usize, M);

    impl<M> MessageWithId<M> {
        pub fn msg(m: M) -> Self {
            MessageWithId(0, m)
        }

        pub fn get(self) -> M {
            self.1
        }
    }

    impl<
            Payload: serde::de::DeserializeOwned + serde::Serialize,
            Next: crate::Connection<MessageWithId<Payload>>,
        > crate::Middleware<Payload> for IdMiddleware<Payload, Next>
    {
        type Message = MessageWithId<Payload>;

        type WrapError = Self::NextWrapError;
        type UnwrapError = Self::NextUnwrapInputError;

        type NextWrapError = Next::SendError;
        type NextUnwrapError = Next::NextError;
        type NextUnwrapInputError = Next::ReceiveError;
        type NextWrapped = Next::Wrapped;

        fn wrap<Uncallable: connecteer_capabilities::PublicUncallable>(
            &mut self,
            msg: Payload,
        ) -> Result<Option<Self::Message>, Self::WrapError> {
            let id = self.1.current_id;
            self.1.current_id += 1;
            Ok(Some(MessageWithId(id, msg)))
        }

        fn unwrap<Uncallable: connecteer_capabilities::PublicUncallable>(
            &mut self,
            msg: Self::Message,
        ) -> Result<Option<Payload>, Self::UnwrapError> {
            Ok(Some(msg.1))
        }

        fn get_next<Uncallable: connecteer_capabilities::PublicUncallable>(
            &mut self,
        ) -> &mut connecteer_capabilities::NextConnection<'_, Payload, Self> {
            &mut self.0
        }

        fn create_wrap_error<Uncallable: connecteer_capabilities::PublicUncallable>(
            &mut self,
            err: Self::NextWrapError,
        ) -> Self::WrapError {
            err
        }

        fn create_unwrap_error<Uncallable: connecteer_capabilities::PublicUncallable>(
            &mut self,
            _err: Self::NextUnwrapError,
        ) -> Self::UnwrapError {
            unreachable!()
        }

        fn create_unwrap_error_passthrough<
            Uncallable: connecteer_capabilities::PublicUncallable,
        >(
            &mut self,
            err: Self::NextUnwrapInputError,
        ) -> Self::UnwrapError {
            err
        }
    }
}

mod log {

    pub struct LoggingMiddleware<
        Payload: serde::de::DeserializeOwned + serde::Serialize,
        Next: crate::Connection<crate::Wrapper<Payload>>,
    >(pub Next, pub std::marker::PhantomData<fn() -> Payload>);

    impl<
            Over: serde::de::DeserializeOwned + serde::Serialize,
            Next: crate::Connection<crate::Wrapper<Over>>,
        > LoggingMiddleware<Over, Next>
    {
        pub fn new(next: Next) -> Self {
            Self(next, core::marker::PhantomData)
        }
    }
    impl<
            Over: serde::de::DeserializeOwned + serde::Serialize,
            Next: crate::Connection<crate::Wrapper<Over>>,
        > crate::Middleware<Over> for LoggingMiddleware<Over, Next>
    {
        type Message = crate::Wrapper<Over>;

        type WrapError = Self::NextWrapError;
        type UnwrapError = Self::NextUnwrapInputError;

        type NextWrapError = Next::SendError;
        type NextUnwrapError = Next::NextError;
        type NextUnwrapInputError = Next::ReceiveError;
        type NextWrapped = Next::Wrapped;

        fn wrap<Uncallable: crate::PublicUncallable>(
            &mut self,
            msg: Over,
        ) -> Result<Option<Self::Message>, Self::WrapError> {
            print!("\x1B[32m");
            ron::ser::to_writer_pretty(
                std::io::stdout().lock(),
                &msg,
                ron::ser::PrettyConfig::new().struct_names(true),
            )
            .unwrap();
            println!("\x1B[0m");
            Ok(Some(crate::Wrapper(msg)))
        }

        fn unwrap<Uncallable: crate::PublicUncallable>(
            &mut self,
            msg: Self::Message,
        ) -> Result<Option<Over>, Self::UnwrapError> {
            print!("\x1B[94m");
            ron::ser::to_writer_pretty(
                std::io::stdout().lock(),
                &msg.0,
                ron::ser::PrettyConfig::new().struct_names(true),
            )
            .unwrap();
            println!("\x1B[0m");
            Ok(Some(msg.0))
        }

        fn get_next<Uncallable: crate::PublicUncallable>(
            &mut self,
        ) -> &mut crate::NextConnection<'_, Over, Self> {
            &mut self.0
        }

        fn create_wrap_error<Uncallable: crate::PublicUncallable>(
            &mut self,
            err: Self::NextWrapError,
        ) -> Self::WrapError {
            err
        }

        fn create_unwrap_error<Uncallable: crate::PublicUncallable>(
            &mut self,
            _err: Self::NextUnwrapError,
        ) -> Self::UnwrapError {
            unreachable!()
        }

        fn create_unwrap_error_passthrough<Uncallable: crate::PublicUncallable>(
            &mut self,
            err: Self::NextUnwrapInputError,
        ) -> Self::UnwrapError {
            err
        }
    }
}
// */
