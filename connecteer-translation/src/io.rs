/// This type is meant to represent a type that Needs to be dropped, otherwise the process will
/// abort by triggering two panics
///
/// You may not leak this object as it will be caught
pub struct SignalDrop<T: ?Sized>(std::ptr::NonNull<T>, *mut bool);

impl<T: ?Sized> SignalDrop<T> {
    // this function will panic if the value given as an argument isn't dropped when the closure
    // returns
    //
    // This is designed to be an aborting panic (double panic, so there will be no way to recover);
    pub(crate) fn run_with_val<R>(val: &mut T, code: impl FnOnce(Self) -> R) -> R {
        let mut signal = false;

        let ret = code(Self(std::ptr::NonNull::from(val), &mut signal));

        if !signal {
            struct PanicOnDrop;
            impl Drop for PanicOnDrop {
                fn drop(&mut self) {
                    panic!("This is a panic to abort the process")
                }
            }

            let _p = PanicOnDrop;
            panic!("A SignalDrop as been leaked that shouldn't have. Please check anywhere you were given an `SignalDrop` as a parameter in a closure, and be sure that nothing leaked it");
        }

        ret
    }
}

impl<T: ?Sized> Drop for SignalDrop<T> {
    fn drop(&mut self) {
        // This is safe since the pointer points to a variable that is still valid due to the
        // contract "signed" when creating this Wrapper.
        //
        // Check SignalDropped::new() to see more info on said contract
        unsafe { self.1.write(true) }
    }
}

impl embedded_io::Io
    for SignalDrop<dyn embedded_io::blocking::Read<Error = core::convert::Infallible>>
{
    type Error = core::convert::Infallible;
}

impl embedded_io::blocking::Read
    for SignalDrop<dyn embedded_io::blocking::Read<Error = core::convert::Infallible>>
{
    #[inline(always)]
    fn read(&mut self, buf: &mut [u8]) -> core::result::Result<usize, Self::Error> {
        unsafe { self.0.as_mut().read(buf) }
    }

    #[inline(always)]
    fn read_exact(
        &mut self,
        buf: &mut [u8],
    ) -> core::result::Result<(), embedded_io::blocking::ReadExactError<Self::Error>> {
        unsafe { self.0.as_mut().read_exact(buf) }
    }
}

impl<WriteError: embedded_io::Error> embedded_io::Io
    for SignalDrop<dyn embedded_io::blocking::Write<Error = WriteError>>
{
    type Error = WriteError;
}

impl<WriteError: embedded_io::Error> embedded_io::blocking::Write
    for SignalDrop<dyn embedded_io::blocking::Write<Error = WriteError>>
{
    #[inline(always)]
    fn write(&mut self, buf: &[u8]) -> core::result::Result<usize, WriteError> {
        unsafe { self.0.as_mut().write(buf) }
    }

    #[inline(always)]
    fn flush(&mut self) -> core::result::Result<(), WriteError> {
        unsafe { self.0.as_mut().flush() }
    }

    #[inline(always)]
    fn write_all(&mut self, buf: &[u8]) -> core::result::Result<(), WriteError> {
        unsafe { self.0.as_mut().write_all(buf) }
    }

    #[inline(always)]
    fn write_fmt(
        &mut self,
        fmt: core::fmt::Arguments<'_>,
    ) -> core::result::Result<(), embedded_io::blocking::WriteFmtError<WriteError>> {
        unsafe { self.0.as_mut().write_fmt(fmt) }
    }
}
