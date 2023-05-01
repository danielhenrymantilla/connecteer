mod buffer;

pub const DEFAULT_BUFFER_SIZE: usize = 4096;

pub struct Connection<
    ProtocolDe: 'static,
    ProtocolSer: 'static,
    ProtocolDeFactory: FnMut(&mut dyn std::io::Read) -> ProtocolDe,
    ProtocolSerFactory: FnMut(&mut dyn std::io::Write) -> ProtocolSer,
    Message: serde::Serialize + serde::de::DeserializeOwned + 'static,
> where
    for<'a, 'de> &'a mut ProtocolDe: serde::Deserializer<'de>,
    for<'a> &'a mut ProtocolSer: serde::Serializer,
    //for<'a, 'ser> <&'a mut ProtocolSer as serde::Serializer>::Ok: 'ser,
    // for<'a, 'de> <&'a mut ProtocolDe as serde::Deserializer<'de>>::Error: 'de,
    // for<'a, 'ser> <&'a mut ProtocolSer as serde::Serializer>::Error: 'ser,
{
    buffer: buffer::RingBuffer,
    // This is to satisfy the use of the generic on the type itself;
    protocol_de_factory: ProtocolDeFactory,
    protocol_ser_factory: ProtocolSerFactory,
    message: std::marker::PhantomData<fn() -> Message>,
}

impl<
        ProtocolDe: 'static,
        ProtocolSer: 'static,
        ProtocolDeFactory: FnMut(&mut dyn std::io::Read) -> ProtocolDe,
        ProtocolSerFactory: FnMut(&mut dyn std::io::Write) -> ProtocolSer,
        Message: serde::Serialize + serde::de::DeserializeOwned,
    > Connection<ProtocolDe, ProtocolSer, ProtocolDeFactory, ProtocolSerFactory, Message>
where
    for<'a, 'de> &'a mut ProtocolDe: serde::Deserializer<'de> + 'static,
    for<'a> &'a mut ProtocolSer: serde::Serializer + 'static,
    for<'a, 'de> <&'a mut ProtocolDe as serde::Deserializer<'de>>::Error: 'static,
    for<'a, 'ser> <&'a mut ProtocolSer as serde::Serializer>::Error: 'static,
    //for<'a, 'ser> <&'a mut ProtocolSer as serde::Serializer>::Ok: 'static,
{
    //pub fn new(ser_factory: ProtocolSerFactory, de_factory: ProtocolDeFactory) -> Self {
    //    Self::with_capacity(ser_factory, de_factory, DEFAULT_BUFFER_SIZE)
    //}

    pub fn with_capacity(
        ser_factory: ProtocolSerFactory,
        de_factory: ProtocolDeFactory,
        capacity: usize,
    ) -> Self {
        Self {
            buffer: buffer::RingBuffer::new(capacity),
            protocol_de_factory: de_factory,
            protocol_ser_factory: ser_factory,
            message: std::marker::PhantomData,
        }
    }

    pub fn feed_bytes(&mut self, bytes: &[u8]) {
        self.buffer.feed_bytes(bytes)
    }

    pub fn try_deserialize(
        &mut self,
    ) -> Result<Message, <&mut ProtocolDe as serde::Deserializer>::Error> {
        let mut deserializer = (self.protocol_de_factory)(&mut self.buffer.as_read());

        Message::deserialize(&mut deserializer)
    }

    pub fn serialize(
        &mut self,
        value: Message,
    ) -> Result<Vec<u8>, <&mut ProtocolSer as serde::Serializer>::Error> {
        let mut buf = Vec::with_capacity(128);
        let mut serializer = (self.protocol_ser_factory)(&mut buf);

        let res = value.serialize(&mut serializer);

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
