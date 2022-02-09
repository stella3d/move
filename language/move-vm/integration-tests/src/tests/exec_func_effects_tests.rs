// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{ModuleId},
    value::{MoveValue, serialize_values}
};
use move_vm_runtime::{move_vm::MoveVM, session::ExecutionResult};
use move_vm_test_utils::{InMemoryStorage};
use move_vm_types::gas_schedule::GasStatus;

use crate::compiler::{compile_units, as_module};


const TEST_ADDR: AccountAddress = AccountAddress::new([42; AccountAddress::LENGTH]);
const TEST_MODULE_ID: &str = "M";

#[test]
fn basic_mutref_out() {
    let vm = MoveVM::new(vec![]).unwrap();
    let mut storage = InMemoryStorage::new();

    let use_mutref_label = "use_mutref";
    let expect_mutref_value: u32 = 90;
    // use_mutref writes to the mutable reference, so we can exercise mut ref output code
    let code = format!(
        r#"
        module 0x{}::{} {{
            fun {}(a: &mut u64) {{ *a = {}; }}
        }}
    "#,
        TEST_ADDR, TEST_MODULE_ID, use_mutref_label, expect_mutref_value
    );

    let mut units = compile_units(&code).unwrap();
    let m = as_module(units.pop().unwrap());
    let mut blob = vec![];
    m.serialize(&mut blob).unwrap();

    let module_id = ModuleId::new(TEST_ADDR, Identifier::new(TEST_MODULE_ID).unwrap());
    storage.publish_or_overwrite_module(module_id.clone(), blob);

    let sess = vm.new_session(&storage);
    let use_mutref_name = Identifier::new(use_mutref_label).unwrap();
    let mut gas_status = GasStatus::new_unmetered();

    let result = sess
        .execute_function_for_effects(
            &module_id,
            &use_mutref_name,
            vec![],
            serialize_values(&vec![MoveValue::U64(1)]),
            &mut gas_status
        );

    log_exec_result(&result);
}

fn log_exec_result(result: &ExecutionResult) {
    match result {
        move_vm_runtime::session::ExecutionResult::Success { 
            change_set: _, 
            events, 
            return_values, 
            mutable_ref_values, 
            gas_used 
        } => {
            println!("execution result:  SUCCESS");
            println!("gas used:  {}", gas_used);
            events.iter().for_each(|e| {
                println!("event:  {:?}", e);
            });
            return_values.iter().for_each(|rv| {
                println!("return value:  {:?}", rv);
            });
            mutable_ref_values.iter().for_each(|mr| {
                println!("mut ref value:  {:?}", mr);
            });
        },
        move_vm_runtime::session::ExecutionResult::Fail { error, gas_used } => {
            println!("execution result:  FAIL");
            println!("error:  {}", &error);
            println!("gas used:  {}", gas_used);
        }
    }
}