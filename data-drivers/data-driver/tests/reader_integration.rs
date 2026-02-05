// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg(feature = "reader")]

//! Integration tests for the data-driver reader with actual WASM blobs.
//!
//! These tests use the compiled stake-contract data-driver WASM module
//! to verify the DriverReader API works end-to-end.
//!
//! ## Building the WASM
//!
//! The stake_driver.wasm is built from stake-contract:
//!
//! ```bash
//! cd stake-contract && make wasm-js
//! cp target/.../dusk_stake_contract_dd_opt.wasm ../data-driver/tests/stake_driver.wasm
//! ```

use base64::prelude::*;
use base64::Engine as _;
use dusk_data_driver::reader::DriverReader;
use dusk_wasmtime::{Engine, Instance, Module, Store};

const STAKE_DRIVER_WASM: &[u8] = include_bytes!("stake_driver.wasm");

#[test]
fn test_decode_reward_event() {
    // Event data from the user's example
    let topic = "reward";
    let data_b64 = "p9TzswAUM5jeO7Pb2LwCjJw9w/lu9CMtkPVvUHkswGCZ2KwFsmwxtxkQFDSGevMDIiI9EuBahkI/srNGJAmN6gQ+MPjGuKOubmo0+TB3J770u+6PnhRoIwCmpboKIRgMcRzg3G/DAlIuGOmtZKGIgaLCz8Y2/70seM/U2QKYbhZwBO9hKQkC4lnzoQdRdCoFBvhadNzFOHfWAklCezQgvtNB0Rs/VqYb5CYryHq9VbplEDNYN3LUYUfx15lsudoFAAAAAAAAAAAg6oM8AwAAAAAAAAAAAAAAuOYoE8KrZK1eA80bAH8J3SY+L/my04UKS0nE/nxvDnQBncywWdMBqic8ojky15wTCgWkzr4+nSLhJqJ7cmn5KWQluR6k9aIJWcObo6k6KGX+np0eIMQcVap8JBexL5YSeDtNzZFFc4ymL5pZCl8UEHXvbNiwzMShz3rhDTJCi+rjCmGGbaykUaXCnl9AeaQD5Izj81fvszd8vX6rfiW4iHo/TlLFWRFFC1srFfJGz71QRlog9UPP6JUoZhvnmawEAAAAAAAAAADg/Ft2AAAAAAMAAAAAAAAAp9TzswAUM5jeO7Pb2LwCjJw9w/lu9CMtkPVvUHkswGCZ2KwFsmwxtxkQFDSGevMDIiI9EuBahkI/srNGJAmN6gQ+MPjGuKOubmo0+TB3J770u+6PnhRoIwCmpboKIRgMcRzg3G/DAlIuGOmtZKGIgaLCz8Y2/70seM/U2QKYbhZwBO9hKQkC4lnzoQdRdCoFBvhadNzFOHfWAklCezQgvtNB0Rs/VqYb5CYryHq9VbplEDNYN3LUYUfx15lsudoFAAAAAAAAAADg/Ft2AAAAAAEAAAAAAAAAp9TzswAUM5jeO7Pb2LwCjJw9w/lu9CMtkPVvUHkswGCZ2KwFsmwxtxkQFDSGevMDIiI9EuBahkI/srNGJAmN6gQ+MPjGuKOubmo0+TB3J770u+6PnhRoIwCmpboKIRgMcRzg3G/DAlIuGOmtZKGIgaLCz8Y2/70seM/U2QKYbhZwBO9hKQkC4lnzoQdRdCoFBvhadNzFOHfWAklCezQgvtNB0Rs/VqYb5CYryHq9VbplEDNYN3LUYUfx15lsudoFAAAAAAAAAACA/Ft2AAAAAAIAAAAAAAAAoPz//wQAAAA=";

    // Decode base64
    let rkyv_bytes = BASE64_STANDARD
        .decode(data_b64)
        .expect("Failed to decode base64");

    // Load driver and initialize it
    let driver =
        DriverReader::new(STAKE_DRIVER_WASM).expect("Failed to create driver");

    // Decode event
    let result = driver
        .decode_event(topic, &rkyv_bytes)
        .expect("Failed to decode reward event");

    // Verify it's a valid JSON array (reward events are Vec<Reward>)
    assert!(result.is_array(), "Expected array of rewards");
    let rewards = result.as_array().unwrap();
    assert!(!rewards.is_empty(), "Expected at least one reward");

    println!(
        "Decoded rewards: {}",
        serde_json::to_string_pretty(&result).unwrap()
    );
}

#[test]
fn test_init_signature_is_void() {
    let engine = Engine::default();
    let module =
        Module::from_binary(&engine, STAKE_DRIVER_WASM).expect("compile wasm");
    let mut store = Store::new(&engine, ());
    let instance =
        Instance::new(&mut store, &module, &[]).expect("instantiate wasm");

    let init_void = instance
        .get_typed_func::<(), ()>(&mut store, "init")
        .expect("init should be exported as () -> ()");

    let init_result =
        init_void.call(&mut store, ()).expect("init() call failed");
    assert_eq!(init_result, ());

    assert!(
        instance
            .get_typed_func::<(), i32>(&mut store, "init")
            .is_err(),
        "init should not return i32"
    );
}

#[test]
fn test_driver_metadata() {
    // Load driver and initialize it
    let driver =
        DriverReader::new(STAKE_DRIVER_WASM).expect("Failed to create driver");

    // Get version
    let version = driver.get_version().expect("Failed to get version");

    assert!(!version.is_empty(), "Expected non-empty version string");
    println!("Driver version: {}", version);

    // Get schema (should succeed even if returning todo)
    let schema_result = driver.get_schema();
    println!("Schema result: {:?}", schema_result);
}

#[test]
fn test_decode_stake_call() {
    // Stake call data from the user's example
    let fn_name = "stake";
    let fn_args_b64 = "AAAAAAAAAAC7JZxRK9wkcga9BL9DSOWV0Raf/uzUuEcNUe/H4zy/wD/tKt+NtnUeXscJvmFguRC3EWAIUERUD3QkClzhbipJ0ceoDmggUz3AkS3PkkiWBNA3yanT4BK0CG8RKwb9kxe2kV/P2nfVOnMwVbogH57E+QA+rBj6jMWOvGFfWIDk1anWej5JxD8Qtji6P+uBqwrkPe16g4IDhvjCK7EZbSaabQ55//n+ByLH0SSZ3Q6hxlN4Oa7MnZNwD84mniVoPhYAAAAAAAAAAJUuIIcVbwLYuytgY2vWbJQaeba0j+QLIfJX0h8xbaCrQfPe21J3GgxYmFNsKW0xC/nGXhywWQv2XElqfGIRiPHfzrBTNiFGADmWzXOUvZKsaDJaHgDxExfWdf53JKwCBCLI5rbOBOuKsQMlE3GNyHFFsGcqrUU6w5LQGvv+UkUOccXhb9tVU1fuqiMEk0F2E3BTfor3bTFrPQJ2pUymS46nf9jjtzgc2PnDjYjWBKTEL+a+h6Nlo9fiqFFqEkHADgAAAAAAAAAA3J1iOV4AAAA9hdXxrTQCzl9IETBuzAw+itAov+2dz8q6ELB29QllzOclz1acn4twS5wPs9q2iRcUd4ONNMz6X+9yiek7+EcLg9pHY2lu5v9x5Q12Vns/Z2d/IxUHCIZyBrshODxxKRMAAAAAAAAAAM5DZW92wT6JxsWWQrpkVkkshIXdRYdgsNdBPOkvvsLhfJnUYw3bW48IxXOlfYdBAy3Jdv33J8sxql/dQ6WrmHwWI6DLLYZN37h9cYbgCLRsmjhlaJeN8wFroIM/4mmvCgAAAAAAAAAAAQAAAAAAAAA=";

    // Decode base64
    let rkyv_bytes = BASE64_STANDARD
        .decode(fn_args_b64)
        .expect("Failed to decode base64");

    // Load driver and initialize it
    let driver =
        DriverReader::new(STAKE_DRIVER_WASM).expect("Failed to create driver");

    // Decode input function arguments
    let result = driver
        .decode_input_fn(fn_name, &rkyv_bytes)
        .expect("Failed to decode stake input");

    // Verify it's a valid JSON object
    assert!(result.is_object(), "Expected stake input to be an object");

    println!(
        "Decoded stake call: {}",
        serde_json::to_string_pretty(&result).unwrap()
    );
}

#[test]
fn test_decode_withdraw_call() {
    // Withdraw call data from the user's example
    let fn_name = "withdraw";
    let fn_args_b64 = "AQAAAAAAAACVLiCHFW8C2LsrYGNr1myUGnm2tI/kCyHyV9IfMW2gq0Hz3ttSdxoMWJhTbCltMQv5xl4csFkL9lxJanxiEYjx386wUzYhRgA5ls1zlL2SrGgyWh4A8RMX1nX+dySsAgQiyOa2zgTrirEDJRNxjchxRbBnKq1FOsOS0Br7/lJFDnHF4W/bVVNX7qojBJNBdhNwU36K920xaz0CdqVMpkuOp3/Y47c4HNj5w42I1gSkxC/mvoejZaPX4qhRahJBwA4AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAEAAAAAAAAAewAAAAAAAAABAAAAAAAAAFku7i3VuS84lsrW+JBeGTnAVGl+VlC4eYfAGwLW8S9ZY3rLEhbquvdKdzghho3YDg4X8LqqtSdA1t7Gs8xuIAlUSqTYoflMm6gswWI5vrL+1x3InAgl40j5QMkbWE3gBAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAmyN8PF4AAACVLiCHFW8C2LsrYGNr1myUGnm2tI/kCyHyV9IfMW2gq0Hz3ttSdxoMWJhTbCltMQv5xl4csFkL9lxJanxiEYjx386wUzYhRgA5ls1zlL2SrGgyWh4A8RMX1nX+dySsAgQiyOa2zgTrirEDJRNxjchxRbBnKq1FOsOS0Br7/lJFDnHF4W/bVVNX7qojBJNBdhNwU36K920xaz0CdqVMpkuOp3/Y47c4HNj5w42I1gSkxC/mvoejZaPX4qhRahJBwA4AAAAAAAAAAEgHQ2kxOIlSPAbRE6GP+7R52DQ/a7LbTyOyRIasIxoQKspYV0BYoJApjpeiDfj8FsjiHC01UPF6n/PWYgAp7gFjlBqeB3hAXrsumPPOGYJAVRNy54vdD7QzW5TXlruDGAAAAAAAAAAAGxxudm7q/7D9NhxS9/sVr4Y7SpHRmrii23VccSx+USN1ryXM5qAWlXXPCwy0luIWWtBWNi5KZVNYv0cYxDsxzxa1QTeLLJa1gB2L7tEF3jTL5xZdZxIosfmyoeW2cA4XAAAAAAAAAAA=";

    // Decode base64
    let rkyv_bytes = BASE64_STANDARD
        .decode(fn_args_b64)
        .expect("Failed to decode base64");

    // Load driver and initialize it
    let driver =
        DriverReader::new(STAKE_DRIVER_WASM).expect("Failed to create driver");

    // Decode input function arguments
    let result = driver
        .decode_input_fn(fn_name, &rkyv_bytes)
        .expect("Failed to decode withdraw input");

    // Verify it's a valid JSON object
    assert!(
        result.is_object(),
        "Expected withdraw input to be an object"
    );

    println!(
        "Decoded withdraw call: {}",
        serde_json::to_string_pretty(&result).unwrap()
    );
}
