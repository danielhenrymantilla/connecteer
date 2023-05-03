extern crate connecteer;
extern crate serde;
extern crate serde_json;

use connecteer::{Connection, SignalDrop};
use serde::{Deserialize, Serialize};
use serde_json::{Deserializer, Serializer};

fn main() {
    let mut connection = Connection::new(
        serde_json::Serializer::new,
        serde_json::Deserializer::from_reader,
    );

    let val = connection
        .serialize(serde_json::json!({"hello": "world"}))
        .unwrap();
    connection.feed_bytes(&val);

    println!("{val:02X?}\n{}", String::from_utf8(val.clone()).unwrap());
    dbg!(connection.try_deserialize().unwrap());
}
