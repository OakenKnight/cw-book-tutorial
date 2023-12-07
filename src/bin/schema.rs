use cosmwasm_schema::write_api;
use cw_book_tutorial::messages::{InstantiateMsg, ExecuteMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg
    }
}
