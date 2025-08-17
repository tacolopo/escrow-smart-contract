use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Escrow {
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

impl Escrow {
    pub fn is_approver(&self, addr: &Addr) -> bool {
        &self.approver1 == addr 
            || &self.approver2 == addr 
            || self.approver3.as_ref() == Some(addr)
    }

    pub fn has_approved(&self, addr: &Addr) -> bool {
        self.approvals.contains(addr)
    }

    pub fn required_approvals(&self) -> usize {
        // Determine number of unique approver addresses
        let mut unique_approvers: Vec<&Addr> = vec![&self.approver1, &self.approver2];
        if let Some(ref a3) = self.approver3 { unique_approvers.push(a3); }
        unique_approvers.sort();
        unique_approvers.dedup();

        match unique_approvers.len() {
            0 => 0,
            1 => 1,          // If there is only one unique approver, require just one approval
            2 => 2,          // If there are two unique approvers, require both approvals
            _ => 2,          // If there are three unique approvers, require 2 of 3 approvals
        }
    }

    pub fn total_approvers(&self) -> usize {
        let mut unique_approvers: Vec<&Addr> = vec![&self.approver1, &self.approver2];
        if let Some(ref a3) = self.approver3 { unique_approvers.push(a3); }
        unique_approvers.sort();
        unique_approvers.dedup();
        unique_approvers.len()
    }

    pub fn can_be_released(&self) -> bool {
        !self.is_completed && self.approvals.len() >= self.required_approvals()
    }
}

/// Counter for generating unique escrow IDs
pub const ESCROW_COUNTER: Item<u64> = Item::new("escrow_counter");

/// Map from escrow ID to escrow data
pub const ESCROWS: Map<u64, Escrow> = Map::new("escrows");

/// Map from creator address to list of escrow IDs they created
pub const ESCROWS_BY_CREATOR: Map<&Addr, Vec<u64>> = Map::new("escrows_by_creator");

/// Map from beneficiary address to list of escrow IDs where they are the beneficiary
pub const ESCROWS_BY_BENEFICIARY: Map<&Addr, Vec<u64>> = Map::new("escrows_by_beneficiary");

/// Map from approver address to list of escrow IDs where they are an approver
pub const ESCROWS_BY_APPROVER: Map<&Addr, Vec<u64>> = Map::new("escrows_by_approver");
