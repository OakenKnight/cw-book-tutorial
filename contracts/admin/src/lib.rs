use cosmwasm_std::{
    entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult
};
pub mod contract;
pub mod messages;
pub mod state;
pub mod error;
pub mod integration_tests;
use error::ContractError;
use messages::{QueryMsg, InstantiateMsg, ExecuteMsg};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info : MessageInfo,msg : ExecuteMsg) -> Result<Response, ContractError> {
    crate::contract::execute(deps, env, info, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary>{
    crate::contract::query(deps, env, msg)  
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg) -> StdResult<Response> {
        crate::contract::instantiate(deps, env, info, msg)
    }