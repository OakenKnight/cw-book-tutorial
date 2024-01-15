
#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::vec;

    use crate::error::ContractError;
    use crate::messages::{JoinTimeResp, InstantiateMsg, AdminsListResp, ExecuteMsg, QueryMsg};

    use cosmwasm_std::{ Addr, coins, Empty};
    use cw_multi_test::{App, ContractWrapper, Executor, Contract};
    pub const LOCK_PERIOD: u64 = 60 * 60 * 24 ;

    pub fn challenge_contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );
        Box::new(contract)
    }
    #[ignore = "exploit patched"]
    #[test]
    fn double_admin_instantiation(){
        let mut app = App::new(|router, _, storage| {
            router.bank.init_balance(
                storage, 
                &Addr::unchecked("user"), 
                coins(3, "eth")
            ).unwrap()
        });

        let code_id = app.store_code(challenge_contract());

        let addr = app.instantiate_contract(
            code_id, 
            Addr::unchecked("owner"), 
            &InstantiateMsg
            { 
                admins : vec!["admin1".to_owned(), "admin2".to_owned(), "admin1".to_owned()], 
                donation_denom: "eth".to_string()
            },  
            &[], 
            "Contract", 
            None)
            .unwrap();

        assert_eq!(app.wrap().query_balance("user", "eth").unwrap().amount.u128(), 3);

        app.execute_contract(
            Addr::unchecked("user"), 
            addr.clone(), 
            &ExecuteMsg::Donate {  }, 
            &coins(3, "eth".to_string())
        ).unwrap(); 
        
        assert_eq!(app
            .wrap()
            .query_balance("user", "eth")
            .unwrap()
            .amount
            .u128(), 0
        );

        assert_eq!(app
            .wrap()
            .query_balance(&addr, "eth")
            .unwrap()
            .amount
            .u128(), 0);

        assert_eq!(app
            .wrap()
            .query_balance("admin2", "eth")
            .unwrap()
            .amount
            .u128(), 1);

        assert_eq!(app
            .wrap()
            .query_balance("admin1", "eth")
            .unwrap()
            .amount
            .u128(), 2);


    }
    #[ignore = "exploit patched"]
    #[test]
    fn donations_exploit(){
        let mut app = App::new(|router, _, storage| {
            router.bank.init_balance(storage, &Addr::unchecked("user"), coins(55, "eth")).unwrap()
        });

        let code_id = app.store_code(challenge_contract());

        let addr = app.instantiate_contract(
            code_id, 
            Addr::unchecked("owner"), 
            &InstantiateMsg
            { 
                admins : vec!["admin1".to_owned(), "admin2".to_owned(), "admin1".to_owned()], 
                donation_denom: "eth".to_string()
            },  
            &[], 
            "Contract", 
            None)
            .unwrap();

        assert_eq!(app.wrap().query_balance("user", "eth").unwrap().amount.u128(), 55);

        app.execute_contract(
            Addr::unchecked("user"), 
            addr.clone(), 
            &ExecuteMsg::Donate {  }, 
            &coins(55, "eth".to_string())
        ).unwrap(); 
        
        assert_eq!(app
            .wrap()
            .query_balance("user", "eth")
            .unwrap()
            .amount
            .u128(), 0
        );

        assert_eq!(app
            .wrap()
            .query_balance(&addr, "eth")
            .unwrap()
            .amount
            .u128(), 1);

        assert_eq!(app
            .wrap()
            .query_balance("admin2", "eth")
            .unwrap()
            .amount
            .u128(), 18);

        assert_eq!(app
            .wrap()
            .query_balance("admin1", "eth")
            .unwrap()
            .amount
            .u128(), 36);


    }
    #[test]
    fn donations(){
        let mut app = App::new(|router, _, storage| {
            router.bank.init_balance(
                storage, 
                &Addr::unchecked("user"),  
                coins(51, "eth")
            ).unwrap()
        });

        let code_id = app.store_code(challenge_contract());

        let addr = app.instantiate_contract(
            code_id, 
            Addr::unchecked("owner"), 
            &InstantiateMsg
            { 
                admins : vec!["admin1".to_owned(), "admin2".to_owned()], 
                donation_denom: "eth".to_string()
            },  
            &[], 
            "Contract", 
            None)
            .unwrap();

        assert_eq!(app.wrap().query_balance("user", "eth").unwrap().amount.u128(), 51);

        app.execute_contract(
            Addr::unchecked("user"), 
            addr.clone(), 
            &ExecuteMsg::Donate {  }, 
            &coins(51, "eth".to_string())
        ).unwrap(); 
        
        assert_eq!(app
            .wrap()
            .query_balance("user", "eth")
            .unwrap()
            .amount
            .u128(), 1);

        assert_eq!(app
            .wrap()
            .query_balance(&addr, "eth")
            .unwrap()
            .amount
            .u128(), 0);

        assert_eq!(app
            .wrap()
            .query_balance("admin2", "eth")
            .unwrap()
            .amount
            .u128(), 25);

        assert_eq!(app
            .wrap()
            .query_balance("admin1", "eth")
            .unwrap()
            .amount
            .u128(), 25);


    }
    #[test]
    fn add_members(){
        let mut app = App::default();
        let code_id = app.store_code(challenge_contract());

        let addr: Addr = app.instantiate_contract(
            code_id,
            Addr::unchecked("owner"),
            &InstantiateMsg{
                admins : vec!["owner".to_owned()],  
                donation_denom: "eth".to_string()
            },
            &[],
            "Contract",
            None).
            unwrap();
        let response = app.execute_contract(
            Addr::unchecked("owner"),
            addr,
            &ExecuteMsg::AddMembers { admins: vec!["user".to_owned()] },
            &[])
            .unwrap();

        let wasm_event = response.events.iter().find(|ev| ev.ty == "wasm").unwrap();
        
        assert_eq!(
            wasm_event.attributes.iter().find(|e| e.key=="action").unwrap().value, "add_members"
        );
        assert_eq!(
            wasm_event.attributes.iter().find(|e| e.key=="added_count").unwrap().value, "1"
        );
        let admin_events : Vec<_> = response.events.iter().filter(|ev| ev.ty == "wasm-admin_added").collect();
        assert_eq!(admin_events[0].attributes.iter().find(|event| event.key == "addr").unwrap().value, "user")

    }
    #[test]
    fn get_admins_after_block_time(){
        let mut app = App::default();
        let code_id = app.store_code(challenge_contract());

        let addr = app.instantiate_contract(
            code_id,
            Addr::unchecked("owner"),
            &InstantiateMsg{admins: vec!["owner".to_owned()],  donation_denom: "eth".to_string()}, 
            &[], "Contract 1", 
            None)
            .unwrap();
        let timestamp = app.block_info().time;

        app.execute_contract(
            Addr::unchecked("owner"),
            addr.clone(),
            &ExecuteMsg::AddMembers { admins: vec!["admin1".to_owned()] },
            &[])
            .unwrap();
        

        app.update_block(|b| {
            b.time = b.time.plus_seconds(LOCK_PERIOD).plus_nanos(1);
        });

        app.execute_contract(
            Addr::unchecked("owner"),
            addr.clone(),
            &ExecuteMsg::AddMembers { admins: vec!["admin2".to_owned(),"admin3".to_owned()] },
            &[])
            .unwrap();
        let resp : AdminsListResp = app.wrap()
            .query_wasm_smart(
                addr.clone(), 
            &QueryMsg::AdminsAfterBlockTime { timestamp: timestamp }
            ).unwrap();

        assert_eq!(
            resp,
            AdminsListResp {
                admins: vec![Addr::unchecked("admin2"), Addr::unchecked("admin3")],
            }
        );
    }
    #[test]
    fn instantiation(){

        let mut app = App::default();
        let code_id = app.store_code(challenge_contract());

        let addr = app.instantiate_contract(
            code_id,
            Addr::unchecked("owner"),
            &InstantiateMsg{admins: vec![],  donation_denom: "eth".to_string()}, 
            &[], "Contract 1", 
            None)
            .unwrap();
        let resp : AdminsListResp = app
                                    .wrap()
                                    .query_wasm_smart(
                                        addr, 
                                        &QueryMsg::AdminsList {  }
                                    ).unwrap();

        assert_eq!(resp, AdminsListResp{admins: vec![]});
        
        let block = app.block_info();

        let addr = app.instantiate_contract(
            code_id,
             Addr::unchecked("owner"),
              &InstantiateMsg{
                admins : vec![ "admin1".to_owned(), "admin2".to_owned(), "admin3".to_owned()],  
                donation_denom: "eth".to_string()}, 
              &[],
               "Contract 1",
                None).unwrap();

        let resp : AdminsListResp = app.wrap()
            .query_wasm_smart(
                addr.clone(), 
            &QueryMsg::AdminsList {}
            ).unwrap();

        assert_eq!(
            resp,
            AdminsListResp {
                admins: vec![Addr::unchecked("admin1"), Addr::unchecked("admin2"), Addr::unchecked("admin3")],
            }
        );

        let resp_joined_time : JoinTimeResp = app.wrap()
            .query_wasm_smart(
                addr, 
                &QueryMsg::JoinTime { 
                    admin: "admin1".to_owned() 
                }
            ).unwrap();
        assert_eq!(block.time, resp_joined_time.joined)

    }
    #[test]
    fn unauthorized(){
        let mut app = App::default();
        let code_id = app.store_code(challenge_contract());

        let addr = app.instantiate_contract(
            code_id, 
            Addr::unchecked("owner"), 
            &InstantiateMsg{
                admins : vec![],  
                donation_denom: "eth".to_string()
            }, 
            &[], 
            "Contract", 
            None
        ).unwrap();

        let err = app.execute_contract(
            Addr::unchecked("user"), 
            addr, 
            &ExecuteMsg::AddMembers{
                admins: vec!["user".to_owned()]
            }, 
            &[]
        ).unwrap_err();

        assert_eq!(ContractError::Unauthorized { sender: Addr::unchecked("user") }, err.downcast().unwrap())
    }

    #[test]
    fn execute_ok(){
        let mut app = App::default();
        let code_id = app.store_code(challenge_contract());

        let addr: Addr = app.instantiate_contract(
            code_id,
            Addr::unchecked("owner"),
            &InstantiateMsg{
                admins: vec!["admin1".to_owned()],  
                donation_denom: "eth".to_string()
            },
            &[],
            "Contract",
            None).
            unwrap();
        app.execute_contract(
            Addr::unchecked("admin1"),
            addr.clone(),
            &ExecuteMsg::AddMembers { admins: vec!["admin2".to_owned()] },
            &[])
            .unwrap();
        let query_resp : AdminsListResp = app
            .wrap()
            .query_wasm_smart(
                addr, 
                &QueryMsg::AdminsList {  }
            ).unwrap();
        let query_resp : HashSet<Addr> = query_resp.admins.into_iter().collect();

        assert_eq!(query_resp, vec![Addr::unchecked("admin1"), Addr::unchecked("admin2")].into_iter().collect());
    }


}
