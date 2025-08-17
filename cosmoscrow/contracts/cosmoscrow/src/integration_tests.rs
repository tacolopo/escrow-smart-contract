#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_json, Addr, Coin};

    use crate::contract::{execute, instantiate, query};
    use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, EscrowResponse};
    use crate::ContractError;

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn create_escrow_success() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Create escrow
        let info = mock_info("creator", &coins(1000, "ujuno"));
        let msg = ExecuteMsg::CreateEscrow {
            beneficiary: "beneficiary".to_string(),
            approver1: "approver1".to_string(),
            approver2: "approver2".to_string(),
            approver3: Some("approver3".to_string()),
            description: "Test escrow".to_string(),
        };

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.attributes.len(), 6);
        assert_eq!(res.attributes[0].value, "create_escrow");
        assert_eq!(res.attributes[1].value, "1");
    }

    #[test]
    fn create_escrow_insufficient_funds() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Try to create escrow without funds
        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::CreateEscrow {
            beneficiary: "beneficiary".to_string(),
            approver1: "approver1".to_string(),
            approver2: "approver2".to_string(),
            approver3: None,
            description: "Test escrow".to_string(),
        };

        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert!(matches!(err, ContractError::InsufficientFunds {}));
    }

    #[test]
    fn approve_release_success() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Create escrow
        let info = mock_info("creator", &coins(1000, "ujuno"));
        let msg = ExecuteMsg::CreateEscrow {
            beneficiary: "beneficiary".to_string(),
            approver1: "creator".to_string(),
            approver2: "approver2".to_string(),
            approver3: Some("approver3".to_string()),
            description: "Test escrow".to_string(),
        };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // First approval (should fail - creator cannot self-approve)
        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::ApproveRelease { escrow_id: 1 };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert!(matches!(err, ContractError::CannotSelfApprove {}));

        // First approval from approver2
        let info = mock_info("approver2", &[]);
        let msg = ExecuteMsg::ApproveRelease { escrow_id: 1 };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.attributes[0].value, "approve_release");
        assert_eq!(res.attributes[3].value, "1"); // total_approvals

        // Second approval from approver3 - should trigger release
        let info = mock_info("approver3", &[]);
        let msg = ExecuteMsg::ApproveRelease { escrow_id: 1 };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 1); // Bank message to send funds
    }

    #[test]
    fn query_escrow() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Create escrow
        let info = mock_info("creator", &coins(1000, "ujuno"));
        let msg = ExecuteMsg::CreateEscrow {
            beneficiary: "beneficiary".to_string(),
            approver1: "approver1".to_string(),
            approver2: "approver2".to_string(),
            approver3: None,
            description: "Test escrow".to_string(),
        };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Query escrow
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetEscrow { escrow_id: 1 }).unwrap();
        let escrow: EscrowResponse = from_json(&res).unwrap();
        
        assert_eq!(escrow.id, 1);
        assert_eq!(escrow.creator, Addr::unchecked("creator"));
        assert_eq!(escrow.beneficiary, Addr::unchecked("beneficiary"));
        assert_eq!(escrow.amount, Coin::new(1000, "ujuno"));
        assert_eq!(escrow.description, "Test escrow");
        assert!(!escrow.is_completed);
        assert_eq!(escrow.approvals.len(), 0);
    }
}
