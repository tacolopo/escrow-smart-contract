use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Escrow not found")]
    EscrowNotFound {},

    #[error("Escrow already completed")]
    EscrowCompleted {},

    #[error("Insufficient funds sent")]
    InsufficientFunds {},

    #[error("Invalid beneficiary address")]
    InvalidBeneficiary {},

    #[error("Invalid approver address")]
    InvalidApprover {},

    #[error("Approver already approved")]
    AlreadyApproved {},

    #[error("Cannot approve your own escrow as the creator")]
    CannotSelfApprove {},

    #[error("Escrow conditions not met for release")]
    ConditionsNotMet {},
}
