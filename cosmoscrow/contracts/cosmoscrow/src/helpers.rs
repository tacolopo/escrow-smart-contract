use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    to_json_binary, Addr, CosmosMsg, CustomQuery, Querier, QuerierWrapper, StdResult, WasmMsg,
};

use crate::msg::{ExecuteMsg, QueryMsg};

/// CwTemplateContract is a wrapper around Addr that provides a lot of helpers
/// for working with this.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct CwTemplateContract(pub Addr);

impl CwTemplateContract {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    pub fn call<T: Into<ExecuteMsg>>(&self, msg: T) -> StdResult<CosmosMsg> {
        let msg = to_json_binary(&msg.into())?;
        Ok(WasmMsg::Execute {
            contract_addr: self.addr().into(),
            msg,
            funds: vec![],
        }
        .into())
    }

    /// Get Custom
    pub fn query<Q, T, CQ>(&self, querier: &Q, msg: QueryMsg) -> StdResult<T>
    where
        Q: Querier,
        T: serde::de::DeserializeOwned,
        CQ: CustomQuery,
    {
        let msg = to_json_binary(&msg)?;
        let query = cosmwasm_std::WasmQuery::Smart {
            contract_addr: self.addr().into(),
            msg,
        }
        .into();
        let res: T = QuerierWrapper::<CQ>::new(querier).query(&query)?;
        Ok(res)
    }
}
