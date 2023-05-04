extern crate connecteer;
extern crate serde;
extern crate serde_json;

use connecteer::Connection;

fn main() {
    let mut connection = Connection::new(
        serde_json::Serializer::new,
        serde_json::Deserializer::from_reader,
    );

    let before = serde_json::json!({ /* Packet here */ });

    let val = connection.serialize(before.clone()).unwrap();
    connection.feed_bytes(&val);

    assert_eq!(before, connection.try_deserialize().unwrap());
}
