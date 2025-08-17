use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin};

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    /// Create a new escrow with the sent funds
    CreateEscrow {
        /// Address that will receive the funds when released
        beneficiary: String,
        /// First approver address (usually the creator)
        approver1: String,
        /// Second approver address
        approver2: String,
        /// Optional third party approver address
        approver3: Option<String>,
        /// Description of the escrow conditions
        description: String,
    },
    /// Approve the release of funds for a specific escrow
    ApproveRelease {
        /// ID of the escrow to approve
        escrow_id: u64,
    },
    /// Cancel an escrow (only creator can do this if no approvals yet)
    CancelEscrow {
        /// ID of the escrow to cancel
        escrow_id: u64,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get details of a specific escrow
    #[returns(EscrowResponse)]
    GetEscrow { escrow_id: u64 },
    
    /// Get all escrows for a specific address (as creator, beneficiary, or approver)
    #[returns(EscrowListResponse)]
    GetEscrowsByAddress { 
        address: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    
    /// Get all escrows (paginated)
    #[returns(EscrowListResponse)]
    GetAllEscrows {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct EscrowResponse {
    pub id: u64,
    pub creator: Addr,
    pub beneficiary: Addr,
    pub amount: Coin,
    pub approver1: Addr,
    pub approver2: Addr,
    pub approver3: Option<Addr>,
    pub description: String,
    pub approvals: Vec<Addr>,
    pub is_completed: bool,
    pub created_at: u64,
    pub completed_at: Option<u64>,
}

#[cw_serde]
pub struct EscrowListResponse {
    pub escrows: Vec<EscrowResponse>,
}

#[cw_serde]
pub struct MigrateMsg {}
