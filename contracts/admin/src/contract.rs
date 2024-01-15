use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, to_json_binary};
use crate::error::ContractError;
use crate::messages::{QueryMsg, InstantiateMsg, AdminsListResp, ExecuteMsg};
use crate::state::{ADMINS, DONATION_DENOM};


pub fn instantiate(deps: DepsMut, env: Env, _info: MessageInfo, msg: InstantiateMsg) -> StdResult<Response> {
    for admin in msg.admins{ 
        let admin = deps.api.addr_validate(&admin)?;
        ADMINS.save(deps.storage, &admin, &env.block.time)?;
    }
    
    DONATION_DENOM.save(deps.storage, &msg.donation_denom)?;
    Ok(Response::new())
}

pub fn execute(
    deps: DepsMut,
    env :Env,
    info:MessageInfo,
    msg: ExecuteMsg ) -> Result<Response, ContractError> {
        use ExecuteMsg::*;
        match msg {
            AddMembers {admins} => exec::add_members(deps, env, info, admins),
            Leave {} => exec::leave(deps, info).map_err(Into::into),
            Donate {  } => exec::donate(deps, info)
        }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    use QueryMsg::*;

    match msg{
        AdminsList {} => to_json_binary(&query::admin_list(deps)?),
        JoinTime { admin } => to_json_binary(&query::join_time(deps, admin)?),
        AdminsAfterBlockTime { timestamp } => to_json_binary(&query::admins_after_block_time(deps, timestamp)?)
    }
}
// Modules
mod exec {

    use cosmwasm_std::{Event, BankMsg, coins};
    use cw_utils::*;
    use crate::error::ContractError;

    use super::*;
    

    
    pub fn donate(deps: DepsMut, info : MessageInfo) -> Result<Response, ContractError> {
        let denom = DONATION_DENOM.load(deps.storage)?;

        let admins : Result<Vec<_>, _>= ADMINS.keys(deps.storage, None, None, cosmwasm_std::Order::Ascending).collect();
        let admins = admins?;
        let donation = must_pay(&info, &denom)?.u128();
        let donation_per_admin = donation / (admins.len() as u128);
        let res = donation % (admins.len() as u128);

        let mut messages : Vec<_> = admins.into_iter()
            .map(|admin| BankMsg::Send 
            {   to_address: admin.into_string(), 
                amount: coins(donation_per_admin, &denom)
            }).collect();
        messages.push(BankMsg::Send { to_address: info.sender.into_string(), amount: coins(res, &denom) });
        
                
        Ok(Response::new()
            .add_messages(messages.into_iter())
            .add_attribute("action", "donate")
            .add_attribute("amount", donation.to_string())
            .add_attribute("per_admin", donation_per_admin.to_string()))
    }

    pub fn add_members(deps: DepsMut, env : Env, info: MessageInfo, admins: Vec<String>) -> Result<Response, ContractError>{
        if !ADMINS.has(deps.storage, &info.sender){
            return Err(ContractError::Unauthorized{sender: info.sender})
        }
        let events = admins.iter().map(|admin| Event::new("admin_added").add_attribute("addr", admin));
        
        for admin in admins.clone() {
            let admin = deps.api.addr_validate(&admin)?;
            ADMINS.save(deps.storage, &admin, &env.block.time)?;
        }

        Ok(Response::new().add_events(events)
            .add_attribute("action", "add_members")
            .add_attribute("added_count", admins.len().to_string()))
    }
    pub fn leave(deps: DepsMut, info: MessageInfo) -> StdResult<Response> {
        ADMINS.remove(deps.storage, &info.sender);
        let res = Response::new()
            .add_attribute("action", "leave")
            .add_attribute("sender", info.sender);
        Ok(res)
    }
}

mod query{
    use cosmwasm_std::{Addr, Timestamp};

    use crate::messages::JoinTimeResp;

    use super::*;
    pub fn admin_list(deps: Deps)  -> StdResult<AdminsListResp> {
        let admins: Result<_, _> =  ADMINS.keys(deps.storage, None, None, cosmwasm_std::Order::Ascending).collect();
        Ok(AdminsListResp { admins:admins?})
    }
    pub fn join_time(deps: Deps, admin : String) -> StdResult<JoinTimeResp> {
        ADMINS
            .load(deps.storage, &Addr::unchecked(admin))
            .map(|joined| JoinTimeResp{joined})
    }

    pub fn admins_after_block_time(deps: Deps, timestamp : Timestamp) -> StdResult<AdminsListResp> {    
        let admins : Vec<Addr> = ADMINS.range(deps.storage, None, None, cosmwasm_std::Order::Ascending).into_iter().filter_map(|result| {
                let tuple = result.unwrap();
                if tuple.1 > timestamp{
                    Some(tuple.0)
                }else{
                    None
                }
            }).collect();

        Ok(AdminsListResp{admins : admins})
    }
}
