extern crate connecteer_translation;
extern crate serde;
extern crate serde_json;

use connecteer_translation::{embedded_io::adapters::ToStd, Connection};

fn main() {
    let mut connection = Connection::new_alloc(
        |v| serde_json::Serializer::new(ToStd::new(v)),
        |v| serde_json::Deserializer::from_reader(ToStd::new(v)),
    );

    let before = serde_json::json!({ /* Packet here */ });

    let val = connection.serialize(before.clone()).unwrap();
    connection.feed_bytes(&val);

    assert_eq!(before, connection.try_deserialize().unwrap());
}
