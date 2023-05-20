pub struct PublicUncallable;

pub trait Sealed<P> {}

pub trait PublicUncallableSealed {}

impl PublicUncallableSealed for PublicUncallable {}
