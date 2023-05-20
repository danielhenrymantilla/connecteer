// TODO: split this into another crate

use core::ops::{Generator, GeneratorState};
use core::pin::Pin;

pub unsafe trait Reborrow<'t>: Sized + 't {
    type Reborrower: Reborrower<'t, Out = Self> + 't
    where
        Self: 't;
    fn into_reborrower(self) -> Self::Reborrower;
}

pub unsafe trait Reborrower<'t>: Sized {
    type Out: 't;

    unsafe fn reborrow(&self) -> Self::Out;
}

macro_rules! impl_tuple_reborrow {
            ($($t:ident),*$(,)?) => {
                paste::paste! {
                unsafe impl<'t, $($t: 't,)*> Reborrower<'t> for ($(*mut $t,)*) {
                    type Out = ($(&'t mut $t,)*);
                    unsafe fn reborrow(& self) -> Self::Out {
                        let ($([<val $t:lower>],)*): &($(*mut $t,)*) = self;
                        {($(&mut (**[<val $t:lower >]),)*)}
                    }
                }

                unsafe impl<'t, $($t: 't,)*> Reborrow<'t> for ($(&'t mut $t,)*){
                    type Reborrower = ($(*mut $t,)*);

                    fn into_reborrower(self) -> Self::Reborrower{

                        let ($([<val $t:lower>],)*): ($(&mut $t,)*) = self;
                        {($(([<val $t:lower>]) as *mut $t,)*)}
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

pub struct Map<Args, G, F>(G, F, core::marker::PhantomData<fn() -> Args>);
impl_project!(<{Args, G, F}>: Map<Args, G, F> => (0: G, 1: F));

impl<
        's,
        Args: Reborrow<'s>,
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
        's,
        Args: Reborrow<'s>,
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
        's,
        Args: Reborrow<'s>,
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
                    return GeneratorState::Yielded(match f(unsafe { reborrower.reborrow() }, v) {
                        Some(v) => v,
                        None => continue,
                    })
                }
                GeneratorState::Complete(()) => return GeneratorState::Complete(()),
            }
        }
    }
}

pub struct Flatten<'s, Args: Reborrow<'s>, G: Generator<Args>>(
    G,
    Option<<G as Generator<Args>>::Yield>,
    core::marker::PhantomData<fn() -> &'s Args>,
);
impl_project!(<{'s,  Args: Reborrow<'s>, G: Generator<Args>}>: Flatten<'s, Args, G> => (0: G, 1: Option<G::Yield>));

impl<
        's,
        Args: Reborrow<'s>,
        T: Generator<Args, Return = ()>,
        G: Generator<Args, Return = (), Yield = T>,
    > Generator<Args> for Flatten<'s, Args, G>
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

pub trait GeneratorExt<'s, Args: Reborrow<'s>>: Generator<Args, Return = ()> + Sized {
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
    fn flatten(self) -> Flatten<'s, Args, Self>
    where
        Self::Yield: Generator<Args, Return = ()>,
    {
        Flatten(self, None, core::marker::PhantomData)
    }
}
impl<'s, Args: Reborrow<'s>, G: Generator<Args, Return = ()>> GeneratorExt<'s, Args> for G {}

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
            Both::Left(ref mut l) => Both::Left(unsafe { Pin::new_unchecked(&mut *(l as *mut L)) }),
        }
    }
}

impl<Arg, L: Generator<Arg>, R: Generator<Arg, Yield = L::Yield, Return = L::Return>> Generator<Arg>
    for Both<L, R>
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
