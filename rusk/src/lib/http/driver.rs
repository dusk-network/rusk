// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::BTreeMap;

use dusk_wasmtime::{Instance, Module, Store};

use dusk_core::abi::ContractId;

pub struct DriverExecutor {
    store: Store::<()>,
    instances: BTreeMap<ContractId, Instance>
}

impl DriverExecutor {
    pub fn new() -> Self {
        let store = Store::<()>::default();
        let instances = BTreeMap::new();
        Self {
            store,
            instances,
        }
    }

    pub fn load_bytecode(&mut self, contract_id: &ContractId, bytecode: impl AsRef<[u8]>) -> anyhow::Result<()>{
        let module = Module::new(self.store.engine(), bytecode.as_ref())?;
        let instance = Instance::new(&mut self.store, &module, &[])?;
        self.instances.insert(*contract_id, instance);
        Ok(())
    }

    pub fn exec() {
        // let gcd = instance.get_typed_func::<(i32, i32), i32>(&mut store, "gcd")?;
        // gcd.call(&mut store, (6, 27))?;
    }
}
