use cosmwasm_std::{
    entry_point, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult
};
mod contract;
mod messages;
mod state;
mod error;

use error::ContractError;
use messages::{QueryMsg, InstantiateMsg, ExecuteMsg};

#[entry_point]
pub fn execute(deps: DepsMut, env: Env, info : MessageInfo,msg : ExecuteMsg) -> Result<Response, ContractError> {
    crate::contract::execute(deps, env, info, msg)
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary>{
    crate::contract::query(deps, env, msg)  
}

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg) -> StdResult<Response> {
        crate::contract::instantiate(deps, env, info, msg)
    }