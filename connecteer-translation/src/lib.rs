pub use embedded_io;

pub mod buffer;
pub mod io;

pub const DEFAULT_BUFFER_SIZE: usize = 4096;

pub struct Connection<
    ProtocolDe,
    ProtocolSer,
    ProtocolDeFactory,
    ProtocolSerFactory,
    InBuffer,
    OutBufferFactory,
    OutBuffer,
    WriteError,
    Message,
> {
    buffer: InBuffer,
    buffer_factory: OutBufferFactory,
    protocol_de_factory: ProtocolDeFactory,
    protocol_ser_factory: ProtocolSerFactory,
    message_marker: std::marker::PhantomData<fn() -> Message>,
    buffer_marker: std::marker::PhantomData<fn() -> (OutBuffer, WriteError)>,
    protocol_marker: std::marker::PhantomData<fn() -> (ProtocolSer, ProtocolDe)>,
}

impl<
        ProtocolDe,
        ProtocolSer,
        ProtocolDeFactory,
        ProtocolSerFactory,
        InBuffer,
        OutBufferFactory,
        OutBuffer,
        WriteError,
        Message: serde::Serialize + serde::de::DeserializeOwned,
        DeserializerError,
        SerializerError,
    >
    Connection<
        ProtocolDe,
        ProtocolSer,
        ProtocolDeFactory,
        ProtocolSerFactory,
        InBuffer,
        OutBufferFactory,
        OutBuffer,
        WriteError,
        Message,
    >
where
    ProtocolDeFactory: FnMut(
        io::SignalDrop<dyn embedded_io::blocking::Read<Error = core::convert::Infallible>>,
    ) -> ProtocolDe,
    ProtocolSerFactory:
        FnMut(io::SignalDrop<dyn embedded_io::blocking::Write<Error = WriteError>>) -> ProtocolSer,
    InBuffer: buffer::Buffer + 'static,
    OutBufferFactory: FnMut() -> OutBuffer,
    OutBuffer: embedded_io::blocking::Write<Error = WriteError> + 'static,
    for<'r> &'r mut ProtocolSer: serde::Serializer<Error = SerializerError>,
    for<'r, 'de> &'r mut ProtocolDe: serde::Deserializer<'de, Error = DeserializerError>,
{
    pub fn new(
        ser_factory: ProtocolSerFactory,
        de_factory: ProtocolDeFactory,
        buffer_factory: OutBufferFactory,
        inner_buffer: InBuffer,
    ) -> Self {
        Self {
            buffer: inner_buffer,
            protocol_de_factory: de_factory,
            protocol_ser_factory: ser_factory,
            buffer_factory,
            buffer_marker: core::marker::PhantomData,
            message_marker: core::marker::PhantomData,
            protocol_marker: core::marker::PhantomData,
        }
    }

    pub fn feed_bytes(&mut self, bytes: &[u8]) -> usize {
        self.buffer.feed_bytes(bytes)
    }

    pub fn try_deserialize(&mut self) -> Result<Message, DeserializerError>
where {
        // this is because BufferRead as an non 'static lifetime otherwise and it doesn't work
        // Here there are runtime checks in place so that there isn't any memory corruption
        // possible as the process will be aborted if the value is leaked.
        let mut buf: InBuffer::Reader<'static> =
            unsafe { std::mem::transmute(self.buffer.get_read()) };
        io::SignalDrop::<dyn embedded_io::blocking::Read<Error = core::convert::Infallible>>::run_with_val(
            &mut buf,
            |s| {
                let mut deserializer = (self.protocol_de_factory)(s);

                Message::deserialize(&mut deserializer)
            },
        )
    }

    pub fn serialize(&mut self, value: Message) -> Result<OutBuffer, SerializerError>
where {
        let mut buf = (self.buffer_factory)();
        let res =
            io::SignalDrop::<dyn embedded_io::blocking::Write<Error = WriteError>>::run_with_val::<
                Result<_, SerializerError>,
            >(&mut buf, |s| {
                let mut serializer = (self.protocol_ser_factory)(s);

                value.serialize(&mut serializer).map(|_| ())
            });

        match res {
            Err(e) => {
                self.buffer.keep_read_bytes();
                Err(e)
            }
            Ok(_) => {
                self.buffer.discard_read_bytes();
                Ok(buf)
            }
        }
    }
}

#[cfg(feature = "alloc")]
impl<
        ProtocolDe,
        ProtocolSer,
        ProtocolDeFactory,
        ProtocolSerFactory,
        Message: serde::Serialize + serde::de::DeserializeOwned,
        DeserializerError,
        SerializerError,
    >
    Connection<
        ProtocolDe,
        ProtocolSer,
        ProtocolDeFactory,
        ProtocolSerFactory,
        buffer::RingBuffer,
        fn() -> Vec<u8>,
        Vec<u8>,
        core::convert::Infallible,
        Message,
    >
where
    ProtocolDeFactory: FnMut(
        io::SignalDrop<dyn embedded_io::blocking::Read<Error = core::convert::Infallible>>,
    ) -> ProtocolDe,
    ProtocolSerFactory: FnMut(
        io::SignalDrop<dyn embedded_io::blocking::Write<Error = core::convert::Infallible>>,
    ) -> ProtocolSer,
    for<'r> &'r mut ProtocolSer: serde::Serializer<Error = SerializerError>,
    for<'r, 'de> &'r mut ProtocolDe: serde::Deserializer<'de, Error = DeserializerError>,
{
    pub fn new_alloc(ser_factory: ProtocolSerFactory, de_factory: ProtocolDeFactory) -> Self {
        Connection::new(
            ser_factory,
            de_factory,
            || std::vec::Vec::<u8>::with_capacity(128),
            buffer::RingBuffer::new(512),
        )
    }
}
