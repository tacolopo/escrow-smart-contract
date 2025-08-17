use cosmwasm_std::{
    entry_point, to_json_binary, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Order,
    Response, StdResult,
};
use cw_storage_plus::Bound;
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, EscrowResponse, EscrowListResponse, MigrateMsg};
use crate::state::{Escrow, ESCROW_COUNTER, ESCROWS, ESCROWS_BY_CREATOR, ESCROWS_BY_BENEFICIARY, ESCROWS_BY_APPROVER};

// Version info for migration
const CONTRACT_NAME: &str = "crates.io:cosmoscrow";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    
    // Initialize the escrow counter
    ESCROW_COUNTER.save(deps.storage, &0)?;
    
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("contract_name", CONTRACT_NAME)
        .add_attribute("contract_version", CONTRACT_VERSION))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateEscrow {
            beneficiary,
            approver1,
            approver2,
            approver3,
            description,
        } => execute_create_escrow(deps, env, info, beneficiary, approver1, approver2, approver3, description),
        ExecuteMsg::ApproveRelease { escrow_id } => execute_approve_release(deps, env, info, escrow_id),
        ExecuteMsg::CancelEscrow { escrow_id } => execute_cancel_escrow(deps, env, info, escrow_id),
    }
}

pub fn execute_create_escrow(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    beneficiary: String,
    approver1: String,
    approver2: String,
    approver3: Option<String>,
    description: String,
) -> Result<Response, ContractError> {
    // Validate that exactly one coin was sent
    if info.funds.len() != 1 {
        return Err(ContractError::InsufficientFunds {});
    }
    
    let amount = info.funds[0].clone();
    if amount.amount.is_zero() {
        return Err(ContractError::InsufficientFunds {});
    }

    // Validate addresses
    let beneficiary_addr = deps.api.addr_validate(&beneficiary)?;
    let approver1_addr = deps.api.addr_validate(&approver1)?;
    let approver2_addr = deps.api.addr_validate(&approver2)?;
    let approver3_addr = if let Some(addr) = approver3 {
        Some(deps.api.addr_validate(&addr)?)
    } else {
        None
    };

    // Note: We intentionally allow non-unique addresses between beneficiary and approvers
    // to support flows where the beneficiary is also an approver.

    // Get next escrow ID
    let escrow_id = ESCROW_COUNTER.update(deps.storage, |id| -> StdResult<u64> {
        Ok(id + 1)
    })?;

    // Create the escrow
    let escrow = Escrow {
        id: escrow_id,
        creator: info.sender.clone(),
        beneficiary: beneficiary_addr.clone(),
        amount: amount.clone(),
        approver1: approver1_addr.clone(),
        approver2: approver2_addr.clone(),
        approver3: approver3_addr.clone(),
        description: description.clone(),
        approvals: vec![],
        is_completed: false,
        created_at: env.block.time.seconds(),
        completed_at: None,
    };

    // Save the escrow
    ESCROWS.save(deps.storage, escrow_id, &escrow)?;

    // Update indexes
    update_escrow_indexes(deps.storage, &escrow, true)?;

    Ok(Response::new()
        .add_attribute("method", "create_escrow")
        .add_attribute("escrow_id", escrow_id.to_string())
        .add_attribute("creator", info.sender)
        .add_attribute("beneficiary", beneficiary)
        .add_attribute("amount", amount.to_string())
        .add_attribute("description", description))
}

pub fn execute_approve_release(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    escrow_id: u64,
) -> Result<Response, ContractError> {
    let mut escrow = ESCROWS.load(deps.storage, escrow_id)?;
    
    if escrow.is_completed {
        return Err(ContractError::EscrowCompleted {});
    }

    // Check if sender is an approver
    if !escrow.is_approver(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    // Check if already approved
    if escrow.has_approved(&info.sender) {
        return Err(ContractError::AlreadyApproved {});
    }

    // Creator is allowed to approve if they are one of the approvers

    // Add approval
    escrow.approvals.push(info.sender.clone());

    let mut response = Response::new()
        .add_attribute("method", "approve_release")
        .add_attribute("escrow_id", escrow_id.to_string())
        .add_attribute("approver", info.sender.to_string())
        .add_attribute("total_approvals", escrow.approvals.len().to_string());

    // Check if we have enough approvals to release funds
    if escrow.can_be_released() {
        // Mark as completed
        escrow.is_completed = true;
        escrow.completed_at = Some(env.block.time.seconds());

        // Add bank message to send funds to beneficiary
        let bank_msg = BankMsg::Send {
            to_address: escrow.beneficiary.to_string(),
            amount: vec![escrow.amount.clone()],
        };

        response = response
            .add_message(bank_msg)
            .add_attribute("released", "true")
            .add_attribute("released_to", escrow.beneficiary.to_string())
            .add_attribute("amount_released", escrow.amount.to_string());
    }

    // Save updated escrow
    ESCROWS.save(deps.storage, escrow_id, &escrow)?;

    Ok(response)
}

pub fn execute_cancel_escrow(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    escrow_id: u64,
) -> Result<Response, ContractError> {
    let mut escrow = ESCROWS.load(deps.storage, escrow_id)?;
    
    // Only creator can cancel
    if escrow.creator != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    if escrow.is_completed {
        return Err(ContractError::EscrowCompleted {});
    }

    // Can only cancel if no approvals yet
    if !escrow.approvals.is_empty() {
        return Err(ContractError::Unauthorized {});
    }

    // Mark as completed
    escrow.is_completed = true;

    // Return funds to creator
    let bank_msg = BankMsg::Send {
        to_address: escrow.creator.to_string(),
        amount: vec![escrow.amount.clone()],
    };

    // Update indexes
    update_escrow_indexes(deps.storage, &escrow, false)?;

    // Save updated escrow
    ESCROWS.save(deps.storage, escrow_id, &escrow)?;

    Ok(Response::new()
        .add_message(bank_msg)
        .add_attribute("method", "cancel_escrow")
        .add_attribute("escrow_id", escrow_id.to_string())
        .add_attribute("refunded_to", escrow.creator.to_string()))
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetEscrow { escrow_id } => to_json_binary(&query_escrow(deps, escrow_id)?),
        QueryMsg::GetEscrowsByAddress { address, start_after, limit } => {
            to_json_binary(&query_escrows_by_address(deps, address, start_after, limit)?)
        }
        QueryMsg::GetAllEscrows { start_after, limit } => {
            to_json_binary(&query_all_escrows(deps, start_after, limit)?)
        }
    }
}

fn query_escrow(deps: Deps, escrow_id: u64) -> StdResult<EscrowResponse> {
    let escrow = ESCROWS.load(deps.storage, escrow_id)?;
    Ok(escrow_to_response(escrow))
}

fn query_escrows_by_address(
    deps: Deps,
    address: String,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<EscrowListResponse> {
    let addr = deps.api.addr_validate(&address)?;
    let limit = limit.unwrap_or(10) as usize;
    let start = start_after.unwrap_or(0);

    let mut escrow_ids = vec![];
    
    // Get escrows where address is creator
    if let Ok(creator_escrows) = ESCROWS_BY_CREATOR.load(deps.storage, &addr) {
        escrow_ids.extend(creator_escrows);
    }
    
    // Get escrows where address is beneficiary
    if let Ok(beneficiary_escrows) = ESCROWS_BY_BENEFICIARY.load(deps.storage, &addr) {
        escrow_ids.extend(beneficiary_escrows);
    }
    
    // Get escrows where address is approver
    if let Ok(approver_escrows) = ESCROWS_BY_APPROVER.load(deps.storage, &addr) {
        escrow_ids.extend(approver_escrows);
    }

    // Remove duplicates and sort
    escrow_ids.sort();
    escrow_ids.dedup();

    // Apply pagination
    let filtered_ids: Vec<u64> = escrow_ids
        .into_iter()
        .filter(|&id| id > start)
        .take(limit)
        .collect();

    let mut escrows = vec![];
    for id in filtered_ids {
        if let Ok(escrow) = ESCROWS.load(deps.storage, id) {
            escrows.push(escrow_to_response(escrow));
        }
    }

    Ok(EscrowListResponse { escrows })
}

fn query_all_escrows(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<EscrowListResponse> {
    let limit = limit.unwrap_or(10) as usize;
    let start = start_after.map(|s| Bound::exclusive(s));

    let escrows: StdResult<Vec<_>> = ESCROWS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, escrow) = item?;
            Ok(escrow_to_response(escrow))
        })
        .collect();

    Ok(EscrowListResponse { escrows: escrows? })
}

fn escrow_to_response(escrow: Escrow) -> EscrowResponse {
    EscrowResponse {
        id: escrow.id,
        creator: escrow.creator,
        beneficiary: escrow.beneficiary,
        amount: escrow.amount,
        approver1: escrow.approver1,
        approver2: escrow.approver2,
        approver3: escrow.approver3,
        description: escrow.description,
        approvals: escrow.approvals,
        is_completed: escrow.is_completed,
        created_at: escrow.created_at,
        completed_at: escrow.completed_at,
    }
}

fn update_escrow_indexes(
    storage: &mut dyn cosmwasm_std::Storage,
    escrow: &Escrow,
    add: bool,
) -> StdResult<()> {
    // Update creator index
    ESCROWS_BY_CREATOR.update(storage, &escrow.creator, |existing| -> StdResult<Vec<u64>> {
        let mut ids = existing.unwrap_or_default();
        if add {
            ids.push(escrow.id);
        } else {
            ids.retain(|&id| id != escrow.id);
        }
        Ok(ids)
    })?;

    // Update beneficiary index
    ESCROWS_BY_BENEFICIARY.update(storage, &escrow.beneficiary, |existing| -> StdResult<Vec<u64>> {
        let mut ids = existing.unwrap_or_default();
        if add {
            ids.push(escrow.id);
        } else {
            ids.retain(|&id| id != escrow.id);
        }
        Ok(ids)
    })?;

    // Update approver indexes (avoid duplicate updates for the same address)
    let mut unique_approvers: Vec<&cosmwasm_std::Addr> = vec![&escrow.approver1, &escrow.approver2];
    if let Some(ref approver3) = escrow.approver3 {
        unique_approvers.push(approver3);
    }
    unique_approvers.sort();
    unique_approvers.dedup();

    for approver in unique_approvers.into_iter() {
        ESCROWS_BY_APPROVER.update(storage, approver, |existing| -> StdResult<Vec<u64>> {
            let mut ids = existing.unwrap_or_default();
            if add {
                if !ids.contains(&escrow.id) {
                    ids.push(escrow.id);
                }
            } else {
                ids.retain(|&id| id != escrow.id);
            }
            Ok(ids)
        })?;
    }

    Ok(())
}

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // Update stored contract version for future migrations
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new()
        .add_attribute("method", "migrate")
        .add_attribute("contract_name", CONTRACT_NAME)
        .add_attribute("contract_version", CONTRACT_VERSION))
}
