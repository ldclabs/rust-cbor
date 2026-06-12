//! Integer map keys for structs (COSE, RFC 9052) and serde field
//! attributes such as `alias` and `rename`.

use cbor::Value;
use serde::{Deserialize, Serialize};

// A COSE_Key-shaped structure (RFC 9052 §7): all map keys are integers.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct CoseKey {
    #[serde(rename = "1")]
    kty: u8,
    #[serde(rename = "3", alias = "alg")]
    alg: i8,
    #[serde(rename = "-1")]
    crv: u8,
    #[serde(rename = "-2")]
    x: serde_bytes::ByteBuf,
}

fn sample() -> CoseKey {
    CoseKey {
        kty: 2,  // EC2
        alg: -7, // ES256
        crv: 1,  // P-256
        x: serde_bytes::ByteBuf::from(vec![0x11, 0x22, 0x33, 0x44]),
    }
}

#[test]
fn cose_style_integer_keys() {
    // {1: 2, 3: -7, -1: 1, -2: h'11223344'}
    let bytes = cbor::to_vec(&sample()).unwrap();
    assert_eq!(hex::encode(&bytes), "a4010203262001214411223344");

    let back: CoseKey = cbor::from_slice(&bytes).unwrap();
    assert_eq!(back, sample());

    // The same bytes decode through a Value, and Value::serialized
    // produces integer keys too.
    let value: Value = cbor::from_slice(&bytes).unwrap();
    let keys: Vec<&Value> = value.as_map().unwrap().iter().map(|(k, _)| k).collect();
    assert_eq!(
        keys,
        [
            &Value::from(1),
            &Value::from(3),
            &Value::from(-1),
            &Value::from(-2)
        ]
    );
    assert_eq!(Value::serialized(&sample()).unwrap(), value);
    assert_eq!(value.deserialized::<CoseKey>().unwrap(), sample());

    // Canonical encoding sorts the integer keys like any other key.
    let canonical = cbor::to_canonical_vec(&sample()).unwrap();
    assert_eq!(hex::encode(&canonical), "a4010203262001214411223344");
}

#[test]
fn aliases_match_text_and_integer_keys() {
    // A plain string alias, independent of integer keys.
    #[derive(Debug, PartialEq, Deserialize)]
    struct Named {
        #[serde(rename = "name", alias = "title")]
        n: String,
    }

    let renamed = cbor::to_vec(&cbor::cbor!({ "name" => "x" }).unwrap()).unwrap();
    assert_eq!(cbor::from_slice::<Named>(&renamed).unwrap().n, "x");
    let aliased = cbor::to_vec(&cbor::cbor!({ "title" => "x" }).unwrap()).unwrap();
    assert_eq!(cbor::from_slice::<Named>(&aliased).unwrap().n, "x");

    // An integer-renamed field with a textual alias accepts both forms.
    let mixed = cbor::cbor!({
        1 => 2,
        "alg" => -7,
        -1 => 1,
        -2 => cbor::Value::Bytes(vec![0x11, 0x22, 0x33, 0x44]),
    })
    .unwrap();
    let bytes = cbor::to_vec(&mixed).unwrap();
    assert_eq!(cbor::from_slice::<CoseKey>(&bytes).unwrap(), sample());
    assert_eq!(mixed.deserialized::<CoseKey>().unwrap(), sample());
}

#[test]
fn unknown_integer_keys_are_ignored() {
    let extra = cbor::cbor!({
        1 => 2,
        3 => -7,
        -1 => 1,
        -2 => cbor::Value::Bytes(vec![0x11, 0x22, 0x33, 0x44]),
        99 => ["ignored", {"deep" => null}],
        -99 => "also ignored",
    })
    .unwrap();
    let bytes = cbor::to_vec(&extra).unwrap();
    assert_eq!(cbor::from_slice::<CoseKey>(&bytes).unwrap(), sample());
}

#[test]
fn only_canonical_decimals_become_integer_keys() {
    #[derive(Serialize)]
    struct Oddballs {
        #[serde(rename = "0")]
        zero: u8, // integer key 0
        #[serde(rename = "18446744073709551615")]
        umax: u8, // integer key u64::MAX
        #[serde(rename = "-9223372036854775808")]
        imin: u8, // integer key i64::MIN
        #[serde(rename = "01")]
        zero_padded: u8, // text: leading zero
        #[serde(rename = "-0")]
        negative_zero: u8, // text: not canonical
        #[serde(rename = "+1")]
        plus: u8, // text: explicit sign
        #[serde(rename = "1x")]
        suffixed: u8, // text: not a number
        #[serde(rename = "-")]
        dash: u8, // text: no digits
        #[serde(rename = "18446744073709551616")]
        too_big: u8, // text: beyond u64
        #[serde(rename = "-9223372036854775809")]
        too_small: u8, // text: beyond i64
        #[serde(rename = "")]
        empty: u8, // text: empty name
    }

    let value = Value::serialized(&Oddballs {
        zero: 0,
        umax: 1,
        imin: 2,
        zero_padded: 3,
        negative_zero: 4,
        plus: 5,
        suffixed: 6,
        dash: 7,
        too_big: 8,
        too_small: 9,
        empty: 10,
    })
    .unwrap();

    let keys: Vec<&Value> = value.as_map().unwrap().iter().map(|(k, _)| k).collect();
    assert_eq!(
        keys,
        [
            &Value::from(0u64),
            &Value::from(u64::MAX),
            &Value::from(i64::MIN),
            &Value::from("01"),
            &Value::from("-0"),
            &Value::from("+1"),
            &Value::from("1x"),
            &Value::from("-"),
            &Value::from("18446744073709551616"),
            &Value::from("-9223372036854775809"),
            &Value::from(""),
        ]
    );

    // The streaming serializer agrees with the Value serializer.
    let direct = cbor::to_vec(&Oddballs {
        zero: 0,
        umax: 1,
        imin: 2,
        zero_padded: 3,
        negative_zero: 4,
        plus: 5,
        suffixed: 6,
        dash: 7,
        too_big: 8,
        too_small: 9,
        empty: 10,
    })
    .unwrap();
    assert_eq!(direct, cbor::to_vec(&value).unwrap());
}

#[test]
fn struct_variants_use_integer_keys_too() {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    enum Message {
        Signed {
            #[serde(rename = "1")]
            payload: u8,
        },
    }

    let bytes = cbor::to_vec(&Message::Signed { payload: 7 }).unwrap();
    // {"Signed": {1: 7}}
    assert_eq!(hex::encode(&bytes), "a1665369676e6564a10107");
    assert_eq!(
        cbor::from_slice::<Message>(&bytes).unwrap(),
        Message::Signed { payload: 7 }
    );
    assert_eq!(
        Value::serialized(&Message::Signed { payload: 7 })
            .unwrap()
            .deserialized::<Message>()
            .unwrap(),
        Message::Signed { payload: 7 }
    );
}

#[test]
fn non_identifier_keys_are_rejected() {
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct F {
        a: u8,
    }

    // A float key cannot name a field, on either path.
    let msg = cbor::from_slice::<F>(&hex::decode("a1f93c0001").unwrap())
        .unwrap_err()
        .to_string();
    assert!(msg.contains("str, bytes or an integer"), "{msg}");

    let value = Value::Map(vec![(Value::Float(1.0), Value::from(1))]);
    let msg = value.deserialized::<F>().unwrap_err().to_string();
    assert!(msg.contains("str or integer"), "{msg}");
}

#[test]
fn tagged_integer_keys_still_match() {
    // A tag wrapped around an integer key is transparent, like elsewhere.
    #[derive(Debug, PartialEq, Deserialize)]
    struct K {
        #[serde(rename = "1")]
        a: u8,
    }

    let bytes = hex::decode("a1c10107").unwrap(); // {1(1): 7}
    assert_eq!(cbor::from_slice::<K>(&bytes).unwrap(), K { a: 7 });

    let value = Value::Map(vec![(
        Value::Tag(9, Box::new(Value::from(1))),
        Value::from(7),
    )]);
    assert_eq!(value.deserialized::<K>().unwrap(), K { a: 7 });
}

#[test]
fn integer_key_write_failures_propagate() {
    struct Limited(usize);

    impl std::io::Write for Limited {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            if self.0 == 0 {
                return Err(std::io::Error::other("limit"));
            }
            let n = self.0.min(data.len());
            self.0 -= n;
            Ok(n)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[derive(Serialize)]
    struct Pos {
        #[serde(rename = "1")]
        a: u8,
    }

    #[derive(Serialize)]
    struct Neg {
        #[serde(rename = "-1")]
        a: u8,
    }

    // The map header fits, the integer key does not.
    assert!(matches!(
        cbor::to_writer(&Pos { a: 1 }, Limited(1)),
        Err(cbor::ser::Error::Io(..))
    ));
    assert!(matches!(
        cbor::to_writer(&Neg { a: 1 }, Limited(1)),
        Err(cbor::ser::Error::Io(..))
    ));
}
