#![feature(
    type_alias_impl_trait,
    impl_trait_in_assoc_type,
    generators,
    generator_trait
)]
#![no_std]
use core::ops::{Generator, GeneratorState};

// #![warn(clippy::pedantic)]
//use bridge::{GeneratorExt, *};
use serde::{de::DeserializeOwned, Serialize};

fn fuck_mut<'b, T: ?Sized>(v: &mut T) -> &'b mut T {
    unsafe { &mut *(v as *mut T) }
}

pub mod bridge {

    pub trait Reborrow<'s>: Sized + 's {
        type Reborrower: Reborrower<'s, Out = Self> + 's;
        fn into_reborrower(self) -> Self::Reborrower;
    }

    pub trait Reborrower<'s>: Sized + 's {
        type Out;

        unsafe fn reborrow(&self) -> Self::Out;
    }

    macro_rules! impl_tuple_reborrow {
            ($($t:ident),*$(,)?) => {
                paste::paste! {
                impl<'s, $($t: 's,)*> Reborrower<'s> for ($(*mut $t,)*) {
                    type Out = ($(&'s mut $t,)*);
                    unsafe fn reborrow(&self) -> Self::Out {
                        let ($([<val $t:lower>],)*): &($(*mut $t,)*) = self;
                        {($(&mut (**[<val $t:lower >]),)*)}
                    }
                }

                impl<'s, $($t : 's,)*> Reborrow<'s> for ($(&'s mut $t,)*) {
                    type Reborrower = ($(*mut $t,)*);

                    fn into_reborrower(self) -> Self::Reborrower {

                        let ($([<val $t:lower>],)*): ($(&mut $t,)*) = self;
                        {($([<val $t:lower>] as *mut $t,)*)}
                    }
                }
                }
            };
        }

    #[allow(clippy::unused_unit)]
    mod impl_reborrow {
        use super::*;
        impl_tuple_reborrow!();
        impl_tuple_reborrow!(T1);
        impl_tuple_reborrow!(T1, T2);
        impl_tuple_reborrow!(T1, T2, T3);
        impl_tuple_reborrow!(T1, T2, T3, T4);
        impl_tuple_reborrow!(T1, T2, T3, T4, T5);
        impl_tuple_reborrow!(T1, T2, T3, T4, T5, T6);
        impl_tuple_reborrow!(T1, T2, T3, T4, T5, T6, T7);
        impl_tuple_reborrow!(T1, T2, T3, T4, T5, T6, T7, T8);
        impl_tuple_reborrow!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
        impl_tuple_reborrow!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
        impl_tuple_reborrow!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
        impl_tuple_reborrow!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
        impl_tuple_reborrow!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
        impl_tuple_reborrow!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
        impl_tuple_reborrow!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
        impl_tuple_reborrow!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);
    }

    macro_rules! impl_project {
            (<{$($bounds:tt)*}>: $s:ty => ($($name:tt: $out:ty),+$(,)?)) => {
                impl<$($bounds)*> $s {
                    fn project(mut self: ::core::pin::Pin<&'_ mut Self>) -> ($( ::core::pin::Pin<&'_ mut $out>,)+) {
                        paste::paste! {
                        let self_ptr = unsafe{self.as_mut().get_unchecked_mut()};
                        $(let [<ptr_ $name:lower>] = core::ptr::addr_of_mut!(self_ptr.$name);)+
                        (
                            $(unsafe {core::pin::Pin::new_unchecked(&mut *([<ptr_ $name:lower>]))},)+
                        )
                        }
                    }
                }
            };
        }

    use crate::Generator;
    use core::ops::GeneratorState;
    use core::pin::Pin;
    pub struct Map<Args, G, F>(G, F, core::marker::PhantomData<fn() -> Args>);
    impl_project!(<{Args, G, F}>: Map<Args, G, F> => (0: G, 1: F));

    impl<
            'a,
            Args: Reborrow<'a> + 'a,
            T,
            G: Generator<Args, Return = ()>,
            F: FnMut(Args, G::Yield) -> T + Unpin,
        > Generator<Args> for Map<Args, G, F>
    {
        type Yield = T;
        type Return = ();

        fn resume(self: Pin<&mut Self>, arg: Args) -> GeneratorState<Self::Yield, Self::Return> {
            let (g, mut f) = self.project();
            let f = &mut *f;
            let reborrower = arg.into_reborrower();
            match g.resume(unsafe { reborrower.reborrow() }) {
                GeneratorState::Yielded(v) => {
                    GeneratorState::Yielded(f(unsafe { reborrower.reborrow() }, v))
                }
                GeneratorState::Complete(()) => GeneratorState::Complete(()),
            }
        }
    }
    pub struct Filter<Args, G, F>(G, F, core::marker::PhantomData<fn() -> Args>);
    impl_project!(<{Args, G, F}>: Filter<Args, G, F> => (0: G, 1: F));

    impl<
            'a,
            Args: Reborrow<'a> + 'a,
            G: Generator<Args, Return = ()>,
            F: FnMut(Args, &G::Yield) -> bool + Unpin,
        > Generator<Args> for Filter<Args, G, F>
    {
        type Yield = G::Yield;
        type Return = ();

        fn resume(self: Pin<&mut Self>, arg: Args) -> GeneratorState<Self::Yield, Self::Return> {
            let (mut g, mut f) = self.project();
            let f = &mut *f;
            let reborrower = arg.into_reborrower();
            loop {
                match g.as_mut().resume(unsafe { reborrower.reborrow() }) {
                    GeneratorState::Yielded(v) if f(unsafe { reborrower.reborrow() }, &v) => {
                        return GeneratorState::Yielded(v)
                    }
                    GeneratorState::Yielded(_) => continue,
                    GeneratorState::Complete(()) => return GeneratorState::Complete(()),
                }
            }
        }
    }

    pub struct FilterMap<Args, G, F>(G, F, core::marker::PhantomData<fn() -> Args>);
    impl_project!(<{Args, G, F}>: FilterMap<Args, G, F> => (0: G, 1: F));

    impl<
            'a,
            Args: Reborrow<'a> + 'a,
            T,
            G: Generator<Args, Return = ()>,
            F: FnMut(Args, G::Yield) -> Option<T> + Unpin,
        > Generator<Args> for FilterMap<Args, G, F>
    {
        type Yield = T;
        type Return = ();

        fn resume(self: Pin<&mut Self>, arg: Args) -> GeneratorState<Self::Yield, Self::Return> {
            let (mut g, mut f) = self.project();
            let f = &mut *f;
            let reborrower = arg.into_reborrower();
            loop {
                match g.as_mut().resume(unsafe { reborrower.reborrow() }) {
                    GeneratorState::Yielded(v) => {
                        return GeneratorState::Yielded(
                            match f(unsafe { reborrower.reborrow() }, v) {
                                Some(v) => v,
                                None => continue,
                            },
                        )
                    }
                    GeneratorState::Complete(()) => return GeneratorState::Complete(()),
                }
            }
        }
    }

    pub struct Flatten<'a, Args: Reborrow<'a> + 'a, G: Generator<Args>>(
        G,
        Option<<G as Generator<Args>>::Yield>,
        core::marker::PhantomData<fn() -> &'a Args>,
    );
    impl_project!(<{'a, Args: Reborrow<'a> + 'a, G: Generator<Args>}>: Flatten<'a, Args, G> => (0: G, 1: Option<G::Yield>));

    impl<
            'a,
            Args: Reborrow<'a> + 'a,
            T: Generator<Args, Return = ()>,
            G: Generator<Args, Return = (), Yield = T>,
        > Generator<Args> for Flatten<'a, Args, G>
    {
        type Yield = T::Yield;
        type Return = ();

        fn resume(self: Pin<&mut Self>, arg: Args) -> GeneratorState<Self::Yield, Self::Return> {
            let (mut g, mut slot) = self.project();
            let r = arg.into_reborrower();
            if slot.as_mut().is_none() {
                slot.set(match g.as_mut().resume(unsafe { r.reborrow() }) {
                    GeneratorState::Yielded(v) => Some(v),
                    GeneratorState::Complete(()) => return GeneratorState::Complete(()),
                });
            }
            loop {
                match unsafe { slot.as_mut().as_pin_mut().unwrap_unchecked() }
                    .resume(unsafe { r.reborrow() })
                {
                    GeneratorState::Yielded(v) => return GeneratorState::Yielded(v),
                    GeneratorState::Complete(()) => {
                        slot.set(match g.as_mut().resume(unsafe { r.reborrow() }) {
                            GeneratorState::Yielded(v) => Some(v),
                            GeneratorState::Complete(()) => return GeneratorState::Complete(()),
                        })
                    }
                }
            }
        }
    }

    pub trait GeneratorExt<'a, Args: Reborrow<'a> + 'a>:
        Generator<Args, Return = ()> + Sized
    {
        fn map<T, F: FnMut(Args, Self::Yield) -> T>(self, f: F) -> Map<Args, Self, F> {
            Map(self, f, core::marker::PhantomData)
        }
        fn filter<F: FnMut(Args, &Self::Yield) -> bool>(self, f: F) -> Filter<Args, Self, F> {
            Filter(self, f, core::marker::PhantomData)
        }
        fn filter_map<T, F: FnMut(Args, Self::Yield) -> Option<T>>(
            self,
            f: F,
        ) -> FilterMap<Args, Self, F> {
            FilterMap(self, f, core::marker::PhantomData)
        }
        fn flatten(self) -> Flatten<'a, Args, Self>
        where
            Self::Yield: Generator<Args, Return = ()>,
        {
            Flatten(self, None, core::marker::PhantomData)
        }
    }
    impl<'a, Args: Reborrow<'a> + 'a, G: Generator<Args, Return = ()>> GeneratorExt<'a, Args> for G {}

    pub enum Both<L, R> {
        Left(L),
        Right(R),
    }

    impl<L, R> Both<L, R> {
        fn project(self: Pin<&mut Self>) -> Both<Pin<&mut L>, Pin<&mut R>> {
            match unsafe { self.get_unchecked_mut() } {
                Both::Right(ref mut r) => {
                    Both::Right(unsafe { Pin::new_unchecked(&mut *(r as *mut R)) })
                }
                Both::Left(ref mut l) => {
                    Both::Left(unsafe { Pin::new_unchecked(&mut *(l as *mut L)) })
                }
            }
        }
    }

    impl<Arg, L: Generator<Arg>, R: Generator<Arg, Yield = L::Yield, Return = L::Return>>
        Generator<Arg> for Both<L, R>
    {
        type Yield = L::Yield;
        type Return = L::Return;
        fn resume(self: Pin<&mut Self>, arg: Arg) -> GeneratorState<Self::Yield, Self::Return> {
            match self.project() {
                Both::Left(l) => l.resume(arg),
                Both::Right(r) => r.resume(arg),
            }
        }
    }
}

mod sealed {
    pub struct PublicUncallable;

    pub trait Sealed<P> {}

    pub trait PublicUncallableSealed: Sized {
        fn spawn() -> Self;
    }

    impl PublicUncallableSealed for PublicUncallable {
        fn spawn() -> Self {
            Self
        }
    }

    pub trait NextConnectionSealed<U1, U2, U3> {}
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
        let ret = Con::send(message, sealed::PublicUncallable);
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

/// This is a trait that prevent an "outsider" to call some methods on trait, while still allowing
/// you to implement those traits
pub trait PublicUncallable: sealed::PublicUncallableSealed {}

impl PublicUncallable for sealed::PublicUncallable {}

/// You can't implement this trait, you need to let the blanket implementation do its job by
/// implementing [`Middleware`](crate::Middleware) on your types
/// This type isn't used directly by the consumer, it is only used by this crate
pub trait Connection<Payload: Serialize + DeserializeOwned>: sealed::Sealed<Payload> {
    type Wrapped: Serialize + DeserializeOwned;

    type Ctx;

    type SendError;
    type ReceiveError;
    type NextError;

    type SendGen<'s, 'c>: Generator<
            (&'s mut Self, &'c mut Self::Ctx),
            Yield = Result<Self::Wrapped, Self::SendError>,
            Return = (),
        > + 'static
    where
        Self: 's,
        Self::Ctx: 'c;
    type ReceiveGen<'s, 'c>: Generator<
            (&'s mut Self, &'c mut Self::Ctx),
            Yield = Result<Self::Wrapped, Self::ReceiveError>,
            Return = (),
        > + 'static
    where
        Self: 's,
        Self::Ctx: 'c;

    fn send<'s, 'c>(input: Payload, _: sealed::PublicUncallable) -> Self::SendGen<'s, 'c>
    where
        Self: 's,
        Self::Ctx: 'c;
    fn receive<'s, 'c>(
        output: Result<Payload, Self::NextError>,
        _: sealed::PublicUncallable,
    ) -> Self::ReceiveGen<'s, 'c>
    where
        Self::Ctx: 'c,
        Self: 's;
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

    type Ctx;
    type Next: Connection<Self::Message>;

    type WrapGen<'s, 'c>: Generator<
            (&'s mut Self, &'c mut Self::Ctx),
            Yield = Result<Self::Message, Self::WrapError>,
            Return = (),
        > + 'static
    where
        Self::Ctx: 'c,
        Self: 's;
    type UnwrapGen<'s, 'c>: Generator<
            (&'s mut Self, &'c mut Self::Ctx),
            Yield = Result<Payload, Self::UnwrapError>,
            Return = (),
        > + 'static
    where
        Self: 's,
        Self::Ctx: 'c;
    /// Transform an [`Message`](Self::Message) into a Unwrapped `Payload`
    fn wrap<'a, 'b, Uncallable: PublicUncallable>(msg: Payload) -> Self::WrapGen<'a, 'b>;

    /// Transform an `Payload` into a Wrapped [`Message`](Self::Message)
    fn unwrap<'a, 'b, Uncallable: PublicUncallable>(msg: Self::Message) -> Self::UnwrapGen<'a, 'b>;

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
    sealed::Sealed<Payload> for M
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

    type SendGen<'s, 'c> =
        impl Generator<(&'s mut Self,&'c mut Self::Ctx), Yield = Result<Self::Wrapped, Self::SendError>, Return = ()> + 'static where Self::Ctx: 'c, Self: 's;
    type ReceiveGen<'s, 'c>=
        impl Generator<(&'s mut Self,&'c mut Self::Ctx), Yield = Result<Self::Wrapped, Self::ReceiveError>, Return = ()> + 'static where Self::Ctx: 'c, Self: 's;

    #[allow(unreachable_code)]
    fn send<'a, 'b>(input: Payload, _: crate::sealed::PublicUncallable) -> Self::SendGen<'a, 'b>
    where
        Self: 'a,
        Self::Ctx: 'b,
    {
        move |(mut s, mut ctx): (&'a mut Self, &'b mut Self::Ctx)| {
            let mut gen_ptr = M::wrap::<sealed::PublicUncallable>(input);
            let _pin = core::marker::PhantomPinned;
            loop {
                match unsafe { core::pin::Pin::new_unchecked(fuck_mut(&mut gen_ptr)) }
                    .resume((fuck_mut(s), fuck_mut(ctx)))
                {
                    GeneratorState::Yielded(Ok(v)) => {
                        let mut ret = <M::Next>::send(v, sealed::PublicUncallable);
                        let next = fuck_mut(s.get_next::<sealed::PublicUncallable>());
                        while let GeneratorState::Yielded(v) =
                            unsafe { core::pin::Pin::new_unchecked(fuck_mut(&mut ret)) }.resume((
                                fuck_mut(next),
                                M::get_next_ctx::<sealed::PublicUncallable>(fuck_mut(ctx)),
                            ))
                        {
                            let y =
                                v.map_err(|e| s.create_wrap_error::<sealed::PublicUncallable>(e));
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
    fn receive<'a, 'b>(
        output: Result<Payload, Self::NextError>,
        _: crate::sealed::PublicUncallable,
    ) -> Self::ReceiveGen<'a, 'b>
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
    mod anon {
        pub struct SingleGen<T>(pub Option<T>);
        impl<R, T> crate::Generator<R> for SingleGen<T> {
            type Yield = T;
            type Return = ();

            fn resume(
                self: core::pin::Pin<&mut Self>,
                _: R,
            ) -> core::ops::GeneratorState<Self::Yield, Self::Return> {
                let opt = unsafe { self.map_unchecked_mut(|s| &mut s.0).get_unchecked_mut() };
                match opt.take() {
                    Some(v) => crate::GeneratorState::Yielded(v),
                    None => crate::GeneratorState::Complete(()),
                }
            }
        }
    }
    pub struct Identity;

    impl<Payload> crate::sealed::Sealed<Wrapper<Payload>> for Identity {}

    impl<Payload: crate::Serialize + crate::DeserializeOwned + 'static>
        crate::Connection<Wrapper<Payload>> for Identity
    {
        type Ctx = ();
        type Wrapped = Payload;
        type NextError = core::convert::Infallible;
        type SendError = core::convert::Infallible;
        type ReceiveError = core::convert::Infallible;

        type SendGen<'s, 'c> = anon::SingleGen<Result<Payload, Self::SendError>>;
        type ReceiveGen<'s, 'c> = anon::SingleGen<Result<Payload, Self::ReceiveError>>;
        fn send<'a, 'b>(
            input: Wrapper<Payload>,
            _: crate::sealed::PublicUncallable,
        ) -> Self::SendGen<'a, 'b>
        where
            Self: 'a,
            Self::Ctx: 'b,
        {
            anon::SingleGen(Some(Ok(input.into_inner())))
        }

        fn receive<'a, 'b>(
            output: Result<Wrapper<Payload>, Self::NextError>,
            _: crate::sealed::PublicUncallable,
        ) -> Self::ReceiveGen<'a, 'b>
        where
            Self::Ctx: 'b,
            Self: 'a,
        {
            anon::SingleGen(Some(Ok(output.unwrap().into_inner())))
        }
    }
    /// This is the "Base" of all [`Middleware`](crate::Middleware) chain.
    /// This is the only way to have a middleware that doesn't ask for an `Next` middleware.
    pub struct Base;

    impl<Payload: serde::de::DeserializeOwned + serde::Serialize + 'static>
        crate::Middleware<Payload> for Base
    {
        type Message = Wrapper<Payload>;

        type WrapError = core::convert::Infallible;
        type UnwrapError = core::convert::Infallible;

        type Ctx = ();
        type Next = Identity;

        type WrapGen<'s, 'c> = anon::SingleGen<Result<Self::Message, Self::WrapError>>;
        type UnwrapGen<'s, 'c> = anon::SingleGen<Result<Payload, Self::UnwrapError>>;

        fn wrap<'a, 'b, Uncallable: crate::PublicUncallable>(
            msg: Payload,
        ) -> Self::WrapGen<'a, 'b> {
            anon::SingleGen(Some(Ok(Wrapper(msg))))
        }

        fn unwrap<'a, 'b, Uncallable: crate::PublicUncallable>(
            msg: Self::Message,
        ) -> Self::UnwrapGen<'a, 'b> {
            anon::SingleGen(Some(Ok(msg.into_inner())))
        }

        fn create_wrap_error<Uncallable: crate::PublicUncallable>(
            &mut self,
            err: core::convert::Infallible,
        ) -> Self::WrapError {
            err
        }

        fn create_unwrap_error<Uncallable: crate::PublicUncallable>(
            &mut self,
            err: core::convert::Infallible,
        ) -> Self::UnwrapError {
            err
        }

        fn create_unwrap_error_passthrough<Uncallable: crate::PublicUncallable>(
            &mut self,
            err: core::convert::Infallible,
        ) -> Self::UnwrapError {
            err
        }

        fn get_next_ctx<Uncallable: crate::PublicUncallable>(
            _: &mut Self::Ctx,
        ) -> &mut <Self::Next as crate::Connection<Self::Message>>::Ctx {
            unsafe { core::ptr::NonNull::<()>::dangling().as_mut() }
        }
        fn get_next<Uncallable: crate::PublicUncallable>(&mut self) -> &mut Self::Next {
            unsafe { core::ptr::NonNull::<Identity>::dangling().as_mut() }
        }
    }
}
/*
pub use identity::{Base, Wrapper};

mod identity {
    use super::bridge::IntoGenerator;
    use super::Generator;
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

    impl<P> crate::sealed::Sealed<Wrapper<P>, ()> for Identity {}

    struct Identity;
    impl<Payload: crate::Serialize + crate::DeserializeOwned>
        crate::Connection<Wrapper<Payload>, ()> for Identity
    {
        type Wrapped = Payload;

        type ReceiveError = core::convert::Infallible;
        type NextError = core::convert::Infallible;
        type SendError = core::convert::Infallible;

        fn send(
            &mut self,
            input: impl Generator<(), Yield = Wrapper<Payload>, Return = ()>,
            _: &mut (),
            _: crate::sealed::PublicUncallable,
        ) -> Result<Option<Payload>, Self::SendError> {
            Ok(input.map(Wrapper::into_inner))
        }

        fn receive(
            &mut self,
            output: impl Generator<(), Yield = Result<Payload, Self::ReceiveError>, Return = ()>,
            _: &mut (),
            _: crate::sealed::PublicUncallable,
        ) -> impl Generator<(), Yield = Result<Wrapper<Payload>, Self::NextError>, Return = ()>
        {
            Ok(unsafe { output.map(Wrapper) })
        }

        fn is_final(&self) -> bool {
            true
        }
    }

    /// This is the "Base" of all [`Middleware`](crate::Middleware) chain.
    /// This is the only way to have a middleware that doesn't ask for an `Next` middleware.
    pub struct Base;

    impl<M: serde::de::DeserializeOwned + serde::Serialize, Ctx> crate::Middleware<M, Ctx> for Base {
        type Message = Wrapper<M>;

        type WrapError = core::convert::Infallible;
        type UnwrapError = core::convert::Infallible;

        type NextUnwrapError = core::convert::Infallible;
        type NextWrapError = core::convert::Infallible;
        type NextUnwrapInputError = core::convert::Infallible;

        type NextWrapped = Wrapper<M>;

        type Next = impl crate::Connection<Self::NextWrapped, Ctx>;

        fn wrap<Uncallable: crate::PublicUncallable>(
            &mut self,
            msg: M,
            _: &mut Ctx,
        ) -> Result<Option<Self::Message>, Self::WrapError> {
            Ok(Some(Wrapper(msg)))
        }

        fn unwrap<Uncallable: crate::PublicUncallable>(
            &mut self,
            msg: Self::Message,
            _: &mut Ctx,
        ) -> Result<Option<M>, Self::UnwrapError> {
            Ok(Some(msg.0))
        }

        fn get_next<Uncallable: crate::PublicUncallable>(&mut self) -> &mut Self::Next {
            unsafe { core::ptr::NonNull::<Identity>::dangling().as_mut() }
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
*/
