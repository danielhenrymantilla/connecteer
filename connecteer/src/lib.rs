mod buffer;

pub const DEFAULT_BUFFER_SIZE: usize = 4096;

pub struct Connection<ProtocolDe, ProtocolSer, ProtocolDeFactory, ProtocolSerFactory, Message> {
    buffer: buffer::RingBuffer,
    protocol_de_factory: ProtocolDeFactory,
    protocol_ser_factory: ProtocolSerFactory,
    message_marker: std::marker::PhantomData<fn() -> Message>,
    protocol_marker: std::marker::PhantomData<fn() -> (ProtocolSer, ProtocolDe)>,
}

impl<
        ProtocolDe,
        ProtocolSer,
        ProtocolDeFactory,
        ProtocolSerFactory,
        Message: serde::Serialize + serde::de::DeserializeOwned,
        DeserializerError,
        SerializerError,
    > Connection<ProtocolDe, ProtocolSer, ProtocolDeFactory, ProtocolSerFactory, Message>
where
    ProtocolDeFactory: Fn(SignalDrop<dyn std::io::Read>) -> ProtocolDe,
    ProtocolSerFactory: Fn(SignalDrop<dyn std::io::Write>) -> ProtocolSer,
    for<'r> &'r mut ProtocolSer: serde::Serializer<Error = SerializerError>,
    for<'r, 'de> &'r mut ProtocolDe: serde::Deserializer<'de, Error = DeserializerError>,
{
    pub fn new(ser_factory: ProtocolSerFactory, de_factory: ProtocolDeFactory) -> Self {
        Self::with_capacity(ser_factory, de_factory, DEFAULT_BUFFER_SIZE)
    }

    pub fn with_capacity(
        ser_factory: ProtocolSerFactory,
        de_factory: ProtocolDeFactory,
        capacity: usize,
    ) -> Self {
        Self {
            buffer: buffer::RingBuffer::new(capacity),
            protocol_de_factory: de_factory,
            protocol_ser_factory: ser_factory,
            message_marker: std::marker::PhantomData,
            protocol_marker: std::marker::PhantomData,
        }
    }

    pub fn feed_bytes(&mut self, bytes: &[u8]) {
        self.buffer.feed_bytes(bytes)
    }

    pub fn try_deserialize(&mut self) -> Result<Message, DeserializerError>
where {
        // this is because BufferRead as an non 'static lifetime otherwise and it doesn't work
        // Here there are runtime checks in place so that there isn't any memory corruption
        // possible as the process will be aborted if the value is leaked.
        let mut buf: buffer::BufferRead<'static> =
            unsafe { std::mem::transmute(self.buffer.as_read()) };
        SignalDrop::run_with_val(&mut buf as &mut dyn std::io::Read, |s| {
            let mut deserializer = (self.protocol_de_factory)(s);

            Message::deserialize(&mut deserializer)
        })
    }

    pub fn serialize(&mut self, value: Message) -> Result<Vec<u8>, SerializerError>
where {
        let mut buf = Vec::with_capacity(128);
        let res = SignalDrop::run_with_val::<Result<_, SerializerError>>(
            &mut buf as &mut dyn std::io::Write,
            |s| {
                let mut serializer = (self.protocol_ser_factory)(s);

                value.serialize(&mut serializer).map(|_| ())
            },
        );

        match res {
            Err(e) => {
                self.buffer.reset_read_bytes();
                Err(e)
            }
            Ok(_) => {
                self.buffer.discard_read_bytes();
                Ok(buf)
            }
        }
    }
}

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
    fn run_with_val<R>(val: &mut T, code: impl FnOnce(Self) -> R) -> R {
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

impl std::io::Read for SignalDrop<dyn std::io::Read> {
    #[inline(always)]
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        unsafe { self.0.as_mut().read(buf) }
    }

    #[inline(always)]
    fn read_vectored(&mut self, bufs: &mut [std::io::IoSliceMut<'_>]) -> std::io::Result<usize> {
        unsafe { self.0.as_mut().read_vectored(bufs) }
    }

    #[inline(always)]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        unsafe { self.0.as_mut().read_to_end(buf) }
    }

    #[inline(always)]
    fn read_to_string(&mut self, buf: &mut String) -> std::io::Result<usize> {
        unsafe { self.0.as_mut().read_to_string(buf) }
    }

    #[inline(always)]
    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        unsafe { self.0.as_mut().read_exact(buf) }
    }
}

impl std::io::Write for SignalDrop<dyn std::io::Write> {
    #[inline(always)]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        unsafe { self.0.as_mut().write(buf) }
    }

    #[inline(always)]
    fn flush(&mut self) -> std::io::Result<()> {
        unsafe { self.0.as_mut().flush() }
    }

    #[inline(always)]
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        unsafe { self.0.as_mut().write_all(buf) }
    }

    #[inline(always)]
    fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> std::io::Result<()> {
        unsafe { self.0.as_mut().write_fmt(fmt) }
    }
}
