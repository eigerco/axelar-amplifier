use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Parser)]
#[grammar = "aleo-json-like-format.pest"]
struct AleoParser;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Pest: {0}")]
    Pest(#[from] pest::error::Error<Rule>),
    #[error("AleoParser: {0}")]
    AleoParser(String),
}

#[derive(Debug, Serialize, Deserialize)]
enum AleoValue<'a> {
    Object(Vec<(&'a str, AleoValue<'a>)>),
    Array(Vec<AleoValue<'a>>),
    String(&'a str),
    Number(u8),
    Boolean(bool),
    Null,
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct CallContract {
    pub(crate) caller: String,
    pub(crate) sender: String,
    pub(crate) destination_chain: Vec<u8>,
    pub(crate) destination_address: Vec<u8>,
}

impl CallContract {
    pub fn destination_chain(&self) -> String {
        self.destination_chain
            .iter()
            .take_while(|&&value| value != 0) // Stop at the first zero
            .map(|&value| value as char) // Convert to characters
            .collect()
    }
}


fn parse(input: &str) -> Result<Option<Pair<Rule>>, Error> {
    Ok(AleoParser::parse(Rule::aleo, input)?.next())
}

pub fn generic_parse(input: &str) -> Result<AleoValue, Error> {
    let aleo = parse(input)?.ok_or(Error::AleoParser("Empty input".to_string()))?;

    fn parse_value(pair: Pair<Rule>) -> AleoValue {
        match pair.as_rule() {
            Rule::object => AleoValue::Object(
                pair.into_inner()
                    .flat_map(|pair| {
                        let mut inner_rules = pair.into_inner();
                        let name = inner_rules.next().unwrap().as_str();
                        let value = parse_value(inner_rules.next().unwrap());
                        Some((name, value))
                    })
                    .collect(),
            ),
            Rule::array => AleoValue::Array(pair.into_inner().map(parse_value).collect()),
            Rule::number => AleoValue::Number(pair.as_str().replace("u8", "").parse().unwrap()),
            Rule::boolean => AleoValue::Boolean(pair.as_str().parse().unwrap()),
            Rule::null => AleoValue::Null,
            Rule::aleo_address => AleoValue::String(pair.as_str()),
            Rule::pair | Rule::value | Rule::key | Rule::aleo | Rule::WHITESPACE | Rule::EOI => {
                unreachable!()
            }
        }
    }

    Ok(parse_value(aleo))
}

fn parse_array(pair: Pair<Rule>) -> Vec<u8> {
    pair.into_inner()
        .map(|p| p.as_str().replace("u8", "").parse::<u8>().unwrap())
        .collect()
}

pub fn parse_call_contract(input: &str) -> Option<CallContract> {
    let pair = parse(input).ok().flatten()?;

    let mut caller = String::new();
    let mut sender = String::new();
    let mut destination_chain = Vec::new();
    let mut destination_address = Vec::new();

    for field in pair.into_inner() {
        if field.as_rule() == Rule::pair {
            let mut inner = field.into_inner();
            let key = inner.next()?.as_str();
            let value = inner.next()?;

            match key {
                "caller" => caller = value.as_str().to_string(),
                "sender" => sender = value.as_str().to_string(),
                "destination_chain" => destination_chain = parse_array(value),
                "destination_address" => destination_address = parse_array(value),
                _ => {}
            }
        }
    }

    Some(CallContract {
        caller,
        sender,
        destination_chain,
        destination_address,
    })
}
