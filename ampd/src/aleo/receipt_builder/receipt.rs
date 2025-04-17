use aleo_types::transition::Transition;
use error_stack::Report;

use crate::aleo::error::Error;

#[derive(Debug)]
pub enum Receipt<T> {
    Found(T),
    NotFound(Transition, Report<Error>),
}
