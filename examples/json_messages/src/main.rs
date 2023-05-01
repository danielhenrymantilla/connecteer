extern crate connecteer;
extern crate serde;
extern crate serde_json;

use connecteer::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{Deserializer, Serializer};

fn main() {
    let connection = Connection::new(
        serde_json::Serializer::new,
        serde_json::Deserializer::from_reader,
    );
}
