use aleo_types::transition::Transition;
use error_stack::Report;

use crate::aleo::error::Error;
use crate::aleo::receipt_builder::{CallContractReceipt, SignerRotation};

#[derive(Debug)]
pub enum Receipt {
    Found(FoundReceipt),
    NotFound(Transition, Report<Error>),
}

#[derive(Debug)]
pub enum FoundReceipt {
    CallContract(CallContractReceipt),
    SignerRotation(SignerRotation),
}
