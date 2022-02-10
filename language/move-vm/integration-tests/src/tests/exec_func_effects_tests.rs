// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use std::convert::TryInto;

use move_core_types::{
    account_address::AccountAddress,
    identifier::{Identifier},
    language_storage::{ModuleId},
    value::{MoveValue, serialize_values}, vm_status::StatusCode
};
use move_vm_runtime::{move_vm::MoveVM, session::{ExecutionResult, self}};
use move_vm_test_utils::{InMemoryStorage};
use move_vm_types::gas_schedule::GasStatus;

use crate::compiler::{compile_units, as_module};


const TEST_ADDR: AccountAddress = AccountAddress::new([42; AccountAddress::LENGTH]);
const TEST_MODULE_ID: &str = "M";
const EXPECT_MUTREF_OUT_VALUE: u64 = 90;


#[test]
fn mutref_arg_success() {
    match run(MoveValue::U64(1)) {
        ExecutionResult::Success { mutable_ref_values, .. } => {
            let first_parsed = parse_u64_arg(mutable_ref_values.first().unwrap());
            assert_eq!(EXPECT_MUTREF_OUT_VALUE, first_parsed)
        },
        ExecutionResult::Fail { error, .. } => {
            panic!("{:?}", error);
        }
    }
}

#[test]
fn fail_arg_deserialize() {
    vec![MoveValue::U8(16), MoveValue::U128(512), MoveValue::Bool(true)]
    .iter()
    .for_each(|mv| {
        match run(mv.clone()) {
            ExecutionResult::Success { .. } => {
                panic!("Should have failed to deserialize non-u64 type to u64");
            },
            ExecutionResult::Fail { error, .. } => {
                println!("{:?}", error);
                assert_eq!(error.major_status(), StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT);
            }
        }
    });
}

fn run(arg_val0: MoveValue) -> ExecutionResult {
    let use_mutref_label = "use_mutref";
    // use_mutref writes to the mutable reference, so we can exercise mut ref output code
    let code = format!(
        r#"
        module 0x{}::{} {{
            fun {}(a: &mut u64) {{ *a = {}; }}
        }}
    "#,
        TEST_ADDR, TEST_MODULE_ID, use_mutref_label, EXPECT_MUTREF_OUT_VALUE
    );

    let module_id = ModuleId::new(TEST_ADDR, Identifier::new(TEST_MODULE_ID).unwrap());

    let modules = vec![(module_id.clone(), code)];
    let (vm, storage) = setup_vm(&modules);
    let sess = vm.new_session(&storage);

    let use_mutref_name = Identifier::new(use_mutref_label).unwrap();
    let mut gas_status = GasStatus::new_unmetered();

    let result = sess
        .execute_function_for_effects(
            &module_id,
            &use_mutref_name,
            vec![],
            serialize_values(&vec![arg_val0]),
            &mut gas_status
        );

    //log_exec_result(&result);
    result
}

type ModuleCode = (ModuleId, String);

fn setup_vm(modules: &Vec<ModuleCode>) -> (MoveVM, InMemoryStorage) {
    let mut storage = InMemoryStorage::new();
    compile_modules(&mut storage, modules);
    (MoveVM::new(vec![]).unwrap(), storage)
}

// TODO - move this to where test infra lives, see about unifying with similar code
fn compile_modules(mut storage: &mut InMemoryStorage, modules: &Vec<ModuleCode>) {
    modules.iter().for_each(|(id, code)| {
        compile_module(&mut storage, &id, &code);
    });
}

fn log_exec_result(result: &ExecutionResult) {
    match result {
        session::ExecutionResult::Success { 
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
        session::ExecutionResult::Fail { error, gas_used } => {
            println!("execution result:  FAIL");
            println!("error:  {}", &error);
            println!("gas used:  {}", gas_used);
        }
    }
}

fn compile_module(storage: &mut InMemoryStorage, mod_id: &ModuleId, code: &String) {
    let mut units = compile_units(&code).unwrap();
    let module = as_module(units.pop().unwrap());
    let mut blob = vec![];
    module.serialize(&mut blob).unwrap();
    storage.publish_or_overwrite_module(mod_id.clone(), blob);
}

fn parse_u64_arg(arg: &Vec<u8>) -> u64 {
    let as_arr: [u8; 8] = arg[..8].try_into().expect("wrong u64 length, must be 8 bytes");
    u64::from_le_bytes(as_arr)
}