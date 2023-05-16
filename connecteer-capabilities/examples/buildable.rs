#![feature(generator_trait)]
use std::ops::Generator;

use connecteer_capabilities::*;
fn main() {
    let mut pipeline: Pipeline<Base, ()> = Pipeline::new(Base, ());

    let mut gen = core::pin::pin!(pipeline.send(()));
    while let core::ops::GeneratorState::Yielded(_) = gen.as_mut().resume(()) {}
}
