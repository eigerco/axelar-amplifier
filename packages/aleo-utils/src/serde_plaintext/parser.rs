use nom::branch::alt;
use nom::bytes::complete::{tag, take_while1};
use nom::character::complete::{alpha1, alphanumeric1, digit1, multispace0};
use nom::combinator::{map, opt, recognize};
use nom::multi::many0;
use nom::{IResult, Parser};

#[derive(Debug, PartialEq)]
pub enum NumericSuffix {
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
}

pub fn parse_numeric_literal(i: &str) -> IResult<&str, (&str, NumericSuffix)> {
    alt((parse_unsigned_literal, parse_signed_literal)).parse(i)
}

pub fn parse_unsigned_literal(i: &str) -> IResult<&str, (&str, NumericSuffix)> {
    use NumericSuffix::*;
    (
        digit1,
        alt((
            map(tag("u8"), |_| U8),
            map(tag("u16"), |_| U16),
            map(tag("u32"), |_| U32),
            map(tag("u64"), |_| U64),
            map(tag("u128"), |_| U128),
        )),
    )
        .parse(i)
}

pub fn parse_signed_literal(i: &str) -> IResult<&str, (&str, NumericSuffix)> {
    use NumericSuffix::*;
    (
        recognize((opt(tag("-")), digit1)),
        alt((
            map(tag("i8"), |_| I8),
            map(tag("i16"), |_| I16),
            map(tag("i32"), |_| I32),
            map(tag("i64"), |_| I64),
            map(tag("i128"), |_| I128),
        )),
    )
        .parse(i)
}

pub fn parse_bool(i: &str) -> IResult<&str, &str> {
    alt((tag("false"), tag("true"))).parse(i)
}

pub fn parse_aleo_literal(i: &str) -> IResult<&str, &str> {
    alt((parse_suffixed_aleo_literal, parse_prefixed_aleo_literal)).parse(i)
}

pub fn parse_suffixed_aleo_literal(i: &str) -> IResult<&str, &str> {
    recognize((digit1, alt((tag("field"), tag("group"), tag("scalar"))))).parse(i)
}

pub fn parse_prefixed_aleo_literal(i: &str) -> IResult<&str, &str> {
    recognize((alt((tag("aleo1"), tag("sign1"))), take_while1(is_bech32))).parse(i)
}

fn is_bech32(i: char) -> bool {
    "023456789acdefghjklmnpqrstuvwxyz".contains(i)
}

pub fn parse_identifier(i: &str) -> IResult<&str, &str> {
    recognize((alpha1, many0(alt((alphanumeric1, tag("_")))))).parse(i)
}

pub fn parse_whitespace(i: &str) -> IResult<&str, &str> {
    multispace0.parse(i)
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use arbtest::arbtest;

    use crate::serde_plaintext::parser::{
        parse_aleo_literal, parse_bool, parse_identifier, parse_numeric_literal, parse_whitespace,
        NumericSuffix,
    };

    #[test]
    fn test_parse_numeric_literal_with_overflow() {
        // this tests documents it clearly that the parser is not concerned
        // with semantics. Overflow errors should be handled downstream, after
        // parsing
        let cases = [
            format!("{}u8", (u8::MAX as u16) + 1),
            format!("{}u16", (u16::MAX as u32) + 1),
            format!("{}u32", (u32::MAX as u64) + 1),
            format!("{}u64", (u64::MAX as u128) + 1),
        ];

        for case in cases {
            let result = parse_numeric_literal(&case);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_parse_numeric_literal() {
        use NumericSuffix::*;

        let cases = [
            ("0u8", "0", U8),
            ("1u16", "1", U16),
            ("2u32", "2", U32),
            ("3u64", "3", U64),
            ("4u128", "4", U128),
            ("5i8", "5", I8),
            ("6i16", "6", I16),
            ("7i32", "7", I32),
            ("8i64", "8", I64),
            ("9i128", "9", I128),
            ("-5i8", "-5", I8),
            ("-6i16", "-6", I16),
            ("-7i32", "-7", I32),
            ("-8i64", "-8", I64),
            ("-9i128", "-9", I128),
        ];

        for (case, expected_digits, expected_suffix) in cases {
            let (rest, (actual_digits, actual_suffix)) = parse_numeric_literal(case).unwrap();
            assert_eq!(rest, "");
            assert_eq!(actual_digits, expected_digits);
            assert_eq!(actual_suffix, expected_suffix);
        }
    }

    #[test]
    fn test_parse_bool() {
        let cases = ["true", "false"];

        for case in cases {
            let (rest, consumed) = parse_bool(case).unwrap();
            assert_eq!(rest, "");
            assert_eq!(consumed, case.to_string());
        }
    }

    #[test]
    fn test_parse_aleo_literal() {
        let cases = [
            "1field", 
            "2group", 
            "3scalar",
            "aleo17m3l8a4hmf3wypzkf5lsausfdwq9etzyujd0vmqh35ledn2sgvqqzqkqal",
            "sign1u0yl73wpa4jhs7ujdhegglwd6y564c2mz5r5d0kje23c409xdvqz3dx3lzc8cc5ccp3c8fgdgdxl5ckkk20etts3yaunk63z67mg5ppxmnhj3sqkxnxf2mjsc0s7l94yvx85zr7ry7033v8yy2rrn30uz8acfr9n64254452ap0vves06d9gz7as3dwuyx5y4fpzdlwyy40qyvc9kkk",
        ];

        for case in cases {
            let (rest, consumed) = parse_aleo_literal(case).unwrap();
            assert_eq!(rest, "");
            assert_eq!(consumed, case);
        }
    }

    #[test]
    fn test_parse_identifier() {
        let cases = ["caller", "caller_", "c_aller", "c1aller", "Caller"];
        for case in cases {
            let (rest, consumed) = parse_identifier(case).unwrap();
            assert_eq!(rest, "");
            assert_eq!(consumed, case);
        }
    }

    #[test]
    fn test_parse_whitespace() {
        let cases = ["\t", "\r", "\n", " "];
        for case in cases {
            let (rest, consumed) = parse_whitespace(case).unwrap();
            assert_eq!(rest, "");
            assert_eq!(consumed, case);
        }
    }

    #[test]
    fn arbtest_parse_signed_numeric_literal() {
        arbtest(|u| {
            let sign: &str = u.choose(&["-", ""])?;
            let number: u128 = u.arbitrary()?;
            let suffix = u.choose(&["i8", "i16", "i32", "i64", "i128"])?;
            let i = format!("{sign}{number}{suffix}");

            let result = parse_numeric_literal(&i);
            assert!(result.is_ok());
            Ok(())
        })
        .budget(Duration::from_millis(500));
    }

    #[test]
    fn arbtest_parse_unsigned_numeric_literal() {
        arbtest(|u| {
            let number: u128 = u.arbitrary()?;
            let suffix = u.choose(&["u8", "u16", "u32", "u64", "u128"])?;
            let i = format!("{number}{suffix}");

            let result = parse_numeric_literal(&i);
            assert!(result.is_ok());
            Ok(())
        })
        .budget(Duration::from_millis(500));
    }

    #[test]
    fn arbtest_parse_unsigned_numeric_literal_with_sign() {
        arbtest(|u| {
            let number: u128 = u.arbitrary()?;
            let suffix = u.choose(&["u8", "u16", "u32", "u64", "u128"])?;
            let i = format!("-{number}{suffix}");

            let result = parse_numeric_literal(&i);
            assert!(result.is_err());
            Ok(())
        })
        .budget(Duration::from_millis(500));
    }

    #[test]
    fn arbtest_parse_unsuffixed_numeric_literal() {
        arbtest(|u| {
            let sign = u.choose(&["", "-"])?;
            let number: u128 = u.arbitrary()?;
            let i = format!("{sign}{number}");

            let result = parse_numeric_literal(&i);
            assert!(result.is_err());
            Ok(())
        })
        .budget(Duration::from_millis(500));
    }
}
