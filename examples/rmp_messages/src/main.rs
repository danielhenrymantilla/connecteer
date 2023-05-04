extern crate connecteer;
extern crate rmp_serde;
extern crate serde;

use connecteer::Connection;

fn main() {
    let mut connection = Connection::new(rmp_serde::Serializer::new, rmp_serde::Deserializer::new);

    let before = Something {
        foo: "Hello".to_string(),
        bar: 1024,
        baz: "World!".to_string(),
    };
    let val = connection.serialize(before.clone()).unwrap();
    connection.feed_bytes(&val);

    assert_eq!(before, connection.try_deserialize().unwrap());
}

#[derive(PartialEq, serde::Serialize, serde::Deserialize, Debug, Clone)]
struct Something {
    foo: String,
    bar: usize,
    baz: String,
}
