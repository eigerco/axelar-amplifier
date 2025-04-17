use nom::branch::alt;
use nom::bytes::complete::{tag, take_while_m_n};
use nom::character::complete::{alpha1, alphanumeric1, char, digit1, multispace0};
use nom::combinator::{all_consuming, map_parser, map_res, recognize};
use nom::multi::{many0, separated_list1};
use nom::sequence::{delimited, pair, preceded, separated_pair, terminated};
use nom::{IResult, Parser};

/// Translate a Leo-like JSON string into a JSON string
pub fn into_json(i: &str) -> Result<String, String> {
    let (_, result) = all_consuming(alt((record, expression)))
        .parse(i)
        .map_err(|err| format!("{:?}", err))?;

    Ok(result)
}

fn record(i: &str) -> IResult<&str, String> {
    map_res(
        delimited(
            char('{'),
            separated_list1(char(','), field),
            preceded(multispace0, char('}')),
        ),
        |values| -> Result<String, ()> { Ok(format!("{{{}}}", values.join(","))) },
    )
    .parse(i)
}

fn field(i: &str) -> IResult<&str, String> {
    map_res(
        separated_pair(
            preceded(multispace0, identifier),
            tag(":"),
            preceded(multispace0, expression),
        ),
        |(name, value)| -> Result<String, ()> { Ok(format!("\"{name}\":{value}")) },
    )
    .parse(i)
}

/// From Leo's grammar:
///
/// uppercase-letter = %x41-5A ; A-Z
/// lowercase-letter = %x61-7A ; a-z
/// letter = uppercase-letter / lowercase-letter
/// identifier = letter *( letter / decimal-digit / "_" )
fn identifier(i: &str) -> IResult<&str, &str> {
    recognize(pair(alpha1, many0(alt((alphanumeric1, tag("_")))))).parse(i)
}

/// An "expression" in Leo's grammar is too broad, covering a large portion of the
/// language syntax. We only need to parse arrays of literals
/// or structs/records.```
///
/// From Leo's grammar:
///
/// array-expression = "[" expression 1*( "," expression ) [ "," ] "]"
fn array_expression(i: &str) -> IResult<&str, String> {
    map_res(
        delimited(
            tag("["),
            separated_list1(char(','), delimited(multispace0, expression, multispace0)),
            terminated(tag("]"), multispace0),
        ),
        |values| -> Result<String, ()> { Ok(format!("[{}]", values.join(","))) },
    )
    .parse(i)
}

fn expression(i: &str) -> IResult<&str, String> {
    alt((atomic_literal, array_expression, record)).parse(i)
}

/// From Leo's grammar:
///
/// atomic-literal = numeric-literal
///                / boolean-literal
///                / explicit-address-literal
///                / string-literal (not supported in the official Leo yet)
fn atomic_literal(i: &str) -> IResult<&str, String> {
    alt((numeric_literal, boolean_literal, explicit_address_literal)).parse(i)
}

/// From Leo's grammar:
///
/// decimal-digit = %x30-39 ; 0-9
/// decimal-numeral = 1*( decimal-digit *"_" ) (_ separator not supported)
/// numeral = binary-numeral (not yet supported)
///         / octal-numeral (not yet supported)
///         / decimal-numeral
///         / hexadecimal-numeral (not yet supported)
/// unsigned-literal = numeral ( %s"u8" / %s"u16" / %s"u32" / %s"u64" / %s"u128" )
/// signed-literal = numeral ( %s"i8" / %s"i16" / %s"i32" / %s"i64" / %s"i128" )
/// integer-literal = unsigned-literal / signed-literal
/// field-literal = decimal-numeral %s"field"
/// product-group-literal = decimal-numeral %s"group"
/// scalar-literal = decimal-numeral %s"scalar"
/// numeric-literal = integer-literal / field-literal / product-group-literal / scalar-literal
fn numeric_literal(i: &str) -> IResult<&str, String> {
    map_parser(
        terminated(
            digit1,
            alt((
                tag("u8"),
                tag("u16"),
                tag("u32"),
                tag("u64"),
                tag("u128"),
                tag("i8"),
                tag("i16"),
                tag("i32"),
                tag("i64"),
                tag("i128"),
                tag("field"),
                tag("group"),
                tag("scalar"),
            )),
        ),
        into,
    )
    .parse(i)
}

/// From Leo's grammar:
///
/// boolean-literal = %s"true" / %s"false"
fn boolean_literal(i: &str) -> IResult<&str, String> {
    map_parser(alt((tag("true"), tag("false"))), quote).parse(i)
}

/// From Leo's grammar:
///
/// explicit-address-literal = %s"aleo1" 58( lowercase-letter / decimal-digit )
fn explicit_address_literal(i: &str) -> IResult<&str, String> {
    map_parser(
        recognize(pair(
            tag("aleo1"),
            take_while_m_n(58, 58, is_ascii_lowercase_or_ascii_digit),
        )),
        quote,
    )
    .parse(i)
}

fn is_ascii_lowercase_or_ascii_digit(i: char) -> bool {
    i.is_ascii_lowercase() || i.is_ascii_digit()
}

fn into(i: &str) -> IResult<&str, String> {
    Ok((i, i.into()))
}

fn quote(i: &str) -> IResult<&str, String> {
    Ok((i, format!("\"{i}\"")))
}

#[cfg(test)]
mod test {
    use serde::Deserialize;

    use super::{
        array_expression, boolean_literal, explicit_address_literal, identifier, into_json,
        is_ascii_lowercase_or_ascii_digit, numeric_literal, record,
    };

    #[test]
    fn test_parse_into_json() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Outer {
            array_of_records: Vec<Inner1>,
            record_of_records: Inner2,
        }

        #[derive(Deserialize, PartialEq, Debug)]
        struct Inner1 {
            foo: usize,
            bar: usize,
        }

        #[derive(Deserialize, PartialEq, Debug)]
        struct Inner2 {
            foo: Inner3,
            bar: Inner4,
        }

        #[derive(Deserialize, PartialEq, Debug)]
        struct Inner3 {
            baz: usize,
        }

        #[derive(Deserialize, PartialEq, Debug)]
        struct Inner4 {
            baz: String,
        }

        let i = "{\n  array_of_records: [\n    {foo: 1u8, bar: 2field},\n    {foo: 1u16, bar: 2group},\n    {foo: 1u32, bar: 2scalar}\n  ],\n    record_of_records: { foo: { baz: 1u32 }, bar: { baz: aleo1lv7mvjhq9zkfc9fn0vt4nv2eh3k90t33gpt6r73n4yf8rae3gc8s526ddx } }\n}";

        let actual: Outer = serde_json::from_str(&into_json(i).unwrap()).unwrap();
        let expected = Outer {
            array_of_records: vec![
                Inner1 { foo: 1, bar: 2 },
                Inner1 { foo: 1, bar: 2 },
                Inner1 { foo: 1, bar: 2 },
            ],
            record_of_records: Inner2 {
                foo: Inner3 { baz: 1 },
                bar: Inner4 {
                    baz: "aleo1lv7mvjhq9zkfc9fn0vt4nv2eh3k90t33gpt6r73n4yf8rae3gc8s526ddx".into(),
                },
            },
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_complete_parse_of_nested_structures() {
        let case = "{\n  array_of_records: [\n    {foo: 1u8, bar: 2field},\n    {foo: 1u16, bar: 2group},\n    {foo: 1u32, bar: 2scalar}\n  ],\n    record_of_records: { foo: { baz: 1u32 }, bar: { baz: aleo1lv7mvjhq9zkfc9fn0vt4nv2eh3k90t33gpt6r73n4yf8rae3gc8s526ddx } }\n}";
        let (rest, consumed) = record(case).unwrap();
        assert_eq!(rest, "");
        let expected = "{\"array_of_records\":[{\"foo\":1,\"bar\":2},{\"foo\":1,\"bar\":2},{\"foo\":1,\"bar\":2}],\"record_of_records\":{\"foo\":{\"baz\":1},\"bar\":{\"baz\":\"aleo1lv7mvjhq9zkfc9fn0vt4nv2eh3k90t33gpt6r73n4yf8rae3gc8s526ddx\"}}}";
        assert_eq!(consumed, expected);
    }

    #[test]
    fn test_complete_parse_of_flat_structure() {
        let case = "{\n  caller: aleo1lv7mvjhq9zkfc9fn0vt4nv2eh3k90t33gpt6r73n4yf8rae3gc8s526ddx,\n    signer: aleo1rtxa7fxfsznuulgcfc77prmwvw7g4y2y7r7xl4xltcygjpn34yzsh2dmln,\n    destination_address: [\n    101u8,\n    116u8,\n    104u8,\n    101u8,\n    114u8,\n    101u8,\n    117u8,\n    109u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8\n  ],\n    destination_chain: [\n    101u8,\n    116u8,\n    104u8,\n    101u8,\n    114u8,\n    101u8,\n    117u8,\n    109u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8\n  ]\n}";
        let (rest, consumed) = record(case).unwrap();
        assert_eq!(rest, "");
        let expected = "{\"caller\":\"aleo1lv7mvjhq9zkfc9fn0vt4nv2eh3k90t33gpt6r73n4yf8rae3gc8s526ddx\",\"signer\":\"aleo1rtxa7fxfsznuulgcfc77prmwvw7g4y2y7r7xl4xltcygjpn34yzsh2dmln\",\"destination_address\":[101,116,104,101,114,101,117,109,0,0,0,0,0,0,0,0,0,0,0,0],\"destination_chain\":[101,116,104,101,114,101,117,109,0,0,0,0,0,0,0,0,0,0,0,0]}";
        assert_eq!(consumed, expected);
    }

    #[test]
    fn test_valid_identifiers() {
        let cases = ["caller", "caller_", "c_aller", "c1aller", "Caller"];
        for case in cases {
            let (rest, consumed) = identifier(case).unwrap();
            assert_eq!(rest, "");
            assert_eq!(consumed, case);
        }
    }

    #[test]
    fn test_invalid_identifiers() {
        let case = "_caller";
        let actual_err = numeric_literal(case).unwrap_err();
        let expected_err = nom::Err::Error(nom::error::Error {
            input: case,
            code: nom::error::ErrorKind::Digit,
        });
        assert_eq!(actual_err, expected_err);

        let case = "1caller";
        let actual_err = numeric_literal(case).unwrap_err();
        let expected_err = nom::Err::Error(nom::error::Error {
            input: "caller",
            code: nom::error::ErrorKind::Tag,
        });
        assert_eq!(actual_err, expected_err);
    }

    #[test]
    fn test_valid_boolean_literal_expression() {
        let case = "[\n    true,\n    false,\n    true,    false\n  ]";
        let (rest, consumed) = array_expression(case).unwrap();
        assert_eq!(rest, "");
        assert_eq!(consumed, "[\"true\",\"false\",\"true\",\"false\"]");
    }

    #[test]
    fn test_valid_address_literal_expression() {
        let case = "[\n    aleo1n6c5ugxk6tp09vkrjegcpcprssdfcf7283agcdtt8gu9qex2c5xs9c28ay,\n    aleo1dsrv0z6wu9mgzl5l7wh62rwmrd4yt3zva7n4ayhvg02luvtqkqgq5tw209,\n    aleo12tf856xd9we5ay090zkep0s3q5e8srzwqr37ds0ppvv5kkzad5fqvwndmx,    aleo1anfvarnm27e2s5j6mzx3kzakx5eryc69re96x6grzkm9nkapkgpq4vyy5t\n  ]";
        let (rest, consumed) = array_expression(case).unwrap();
        assert_eq!(rest, "");
        assert_eq!(
            consumed,
            "[\"aleo1n6c5ugxk6tp09vkrjegcpcprssdfcf7283agcdtt8gu9qex2c5xs9c28ay\",\"aleo1dsrv0z6wu9mgzl5l7wh62rwmrd4yt3zva7n4ayhvg02luvtqkqgq5tw209\",\"aleo12tf856xd9we5ay090zkep0s3q5e8srzwqr37ds0ppvv5kkzad5fqvwndmx\",\"aleo1anfvarnm27e2s5j6mzx3kzakx5eryc69re96x6grzkm9nkapkgpq4vyy5t\"]"
        );
    }

    #[test]
    fn test_valid_array_numeric_literal_expressions() {
        let cases = [
            "[\n    102u8,\n    116u8,\n    109u8,    0u8\n  ]",
            "[\n    102u16,\n    116u16,\n    109u16,    0u16\n  ]",
            "[\n    102u32,\n    116u32,\n    109u32,    0u32\n  ]",
            "[\n    102u64,\n    116u64,\n    109u64,    0u64\n  ]",
            "[\n    102u128,\n    116u128,\n    109u128,    0u128\n  ]",
            "[\n    102i8,\n    116i8,\n    109i8,    0i8\n  ]",
            "[\n    102i16,\n    116i16,\n    109i16,    0i16\n  ]",
            "[\n    102i32,\n    116i32,\n    109i32,    0i32\n  ]",
            "[\n    102i64,\n    116i64,\n    109i64,    0i64\n  ]",
            "[\n    102i128,\n    116i128,\n    109i128,    0i128\n  ]",
            "[\n    102field,\n    116field,\n    109field,    0field\n  ]",
            "[\n    102scalar,\n    116scalar,\n    109scalar,    0scalar\n  ]",
            "[\n    102group,\n    116group,\n    109group,    0group\n  ]",
            // every element of a leo array is of the same type
            // so although these can never be hit, the parser supports it
            "[\n    102u8,\n    116u16,\n    109u32,    0u64\n  ]",
            "[\n    102u128,\n    116field,\n    109scalar,    0group\n  ]",
            "[\n    102i8,\n    116i16,\n    109i32,    0i64\n  ]",
            "[\n    102i128,\n    116field,\n    109scalar,    0group\n  ]",
        ];
        for case in cases {
            let (rest, consumed) = array_expression(case).unwrap();
            assert_eq!(rest, "");
            assert_eq!(consumed, "[102,116,109,0]");
        }
    }

    #[test]
    fn test_failing_array_expression() {
        let case = "[\n  72u8,\n  101u8,\n  108u8,\n  108u8,\n  111u8,\n  44u8,\n  32u8,\n  87u8,\n  111u8,\n  114u8,\n  108u8,\n  100u8,\n  33u8\n]";
        let result = into_json(case).unwrap();
        assert_eq!(result, "[72,101,108,108,111,44,32,87,111,114,108,100,33]");
    }

    #[test]
    fn test_invalid_array_expressions() {
        let case = "[\n    102u256,\n    116u8,\n    109u8,    0u8\n  ]";
        let actual_err = array_expression(case).unwrap_err();
        let expected_err = nom::Err::Error(nom::error::Error {
            input: "102u256,\n    116u8,\n    109u8,    0u8\n  ]",
            code: nom::error::ErrorKind::Char,
        });
        assert_eq!(actual_err, expected_err);

        let cases = ["[\n    \n  ]", "[]"];
        for case in cases {
            let actual_err = array_expression(case).unwrap_err();
            let expected_err = nom::Err::Error(nom::error::Error {
                input: "]",
                code: nom::error::ErrorKind::Char,
            });
            assert_eq!(actual_err, expected_err);
        }
    }

    #[test]
    fn test_valid_numeric_literals() {
        let cases = [
            "1u8", "2u16", "3u32", "4u64", "4u128", "5i8", "6i16", "7i32", "8i64", "9i128",
            "1field", "2group", "3scalar",
        ];

        for case in cases {
            let (rest, consumed) = numeric_literal(case).unwrap();
            assert_eq!(rest, "");
            assert_eq!(consumed, &case[0..1]);
        }
    }

    #[test]
    fn test_invalid_numeric_literals() {
        let case = "1u7";
        let actual_err = numeric_literal(case).unwrap_err();
        let expected_err = nom::Err::Error(nom::error::Error {
            input: "u7",
            code: nom::error::ErrorKind::Tag,
        });
        assert_eq!(actual_err, expected_err);

        let case = "au16";
        let actual_err = numeric_literal(case).unwrap_err();
        let expected_err = nom::Err::Error(nom::error::Error {
            input: case,
            code: nom::error::ErrorKind::Digit,
        });
        assert_eq!(actual_err, expected_err);

        let case = "3j32";
        let actual_err = numeric_literal(case).unwrap_err();
        let expected_err = nom::Err::Error(nom::error::Error {
            input: "j32",
            code: nom::error::ErrorKind::Tag,
        });
        assert_eq!(actual_err, expected_err);

        let case = "10feld";
        let actual_err = numeric_literal(case).unwrap_err();
        let expected_err = nom::Err::Error(nom::error::Error {
            input: "feld",
            code: nom::error::ErrorKind::Tag,
        });
        assert_eq!(actual_err, expected_err);

        let case = "11goup";
        let actual_err = numeric_literal(case).unwrap_err();
        let expected_err = nom::Err::Error(nom::error::Error {
            input: "goup",
            code: nom::error::ErrorKind::Tag,
        });
        assert_eq!(actual_err, expected_err);

        let case = "12scala";
        let actual_err = numeric_literal(case).unwrap_err();
        let expected_err = nom::Err::Error(nom::error::Error {
            input: "scala",
            code: nom::error::ErrorKind::Tag,
        });
        assert_eq!(actual_err, expected_err);
    }

    #[test]
    fn test_valid_boolean_literals() {
        let cases = ["true", "false"];

        for case in cases {
            let (rest, consumed) = boolean_literal(case).unwrap();
            assert_eq!(rest, "");
            assert_eq!(consumed, format!("\"{case}\""));
        }
    }

    #[test]
    fn test_invalid_boolean_literals() {
        let cases = [
            "True",
            "False",
            "TRUE",
            "FALSE",
            "truE",
            "falsE",
            "sth_else_entirely",
        ];

        for case in cases {
            let actual_err = boolean_literal(case).unwrap_err();
            let expected_err = nom::Err::Error(nom::error::Error {
                input: case,
                code: nom::error::ErrorKind::Tag,
            });
            assert_eq!(actual_err, expected_err);
        }
    }

    #[test]
    fn test_valid_explicit_address() {
        let address = "aleo17m3l8a4hmf3wypzkf5lsausfdwq9etzyujd0vmqh35ledn2sgvqqzqkqal";
        let (rest, consumed) = explicit_address_literal(address).unwrap();

        assert_eq!(rest, "");
        assert_eq!(consumed, format!("\"{address}\""));
    }

    #[test]
    fn test_invalid_too_short_explicit_address() {
        let short_address = "aleo17m3l8a4hmf3wypzkf5lsausfdwq9etzyujd0vmqh35ledn2sgvqqzqkqa";
        let actual_err = explicit_address_literal(short_address).unwrap_err();
        let expected_err = nom::Err::Error(nom::error::Error {
            input: &short_address[5..],
            code: nom::error::ErrorKind::TakeWhileMN,
        });

        assert_eq!(actual_err, expected_err);
    }

    #[test]
    fn test_invalid_too_long_explicit_address() {
        let long_address = "aleo17m3l8a4hmf3wypzkf5lsausfdwq9etzyujd0vmqh35ledn2sgvqqzqkqall";
        let expected = format!("\"{}\"", &long_address[..long_address.len() - 1]);
        let (rest, consumed) = explicit_address_literal(long_address).unwrap();
        assert_eq!(rest, "l");
        assert_eq!(consumed, expected);
    }

    #[test]
    fn test_invalid_disallowed_chars_explicit_address() {
        let disallowed_chars_address =
            "aleo17m3l8a4hmf3wypZKF5LSAUSFDWQ9ETzyujd0vmqh35ledn2sgvqqzqkqa";
        let actual_err = explicit_address_literal(disallowed_chars_address).unwrap_err();
        let expected_err = nom::Err::Error(nom::error::Error {
            input: &disallowed_chars_address[5..],
            code: nom::error::ErrorKind::TakeWhileMN,
        });

        assert_eq!(actual_err, expected_err);
    }

    #[test]
    fn test_is_ascii_lowercase_or_ascii_digit() {
        assert!(is_ascii_lowercase_or_ascii_digit('i'));
        assert!(is_ascii_lowercase_or_ascii_digit('2'));
        assert!(!is_ascii_lowercase_or_ascii_digit('J'));
        assert!(!is_ascii_lowercase_or_ascii_digit('.'));
        assert!(!is_ascii_lowercase_or_ascii_digit('$'));
        assert!(!is_ascii_lowercase_or_ascii_digit('\t'));
    }

    #[test]
    fn test_call_contract() {
        let call_contract = "{\n  caller: aleo1d4vxwgfempkqy9qmxlxc6a2qu4vptyaxyllpwlqhwwj22tjn3u9s405jsz,\n  sender: aleo10fmsqwh059uqm74x6t6zgj93wfxtep0avevcxz0n4w9uawymkv9s7whsau,\n  destination_chain: [\n    129497940983541880690546129557858025472u128,\n    0u128\n  ],\n  destination_address: [\n    72059799150973012437864881636927078400u128,\n    0u128,\n    0u128,\n    0u128\n  ]\n}";
        let json = into_json(call_contract);

        assert!(json.is_ok());
    }
}
