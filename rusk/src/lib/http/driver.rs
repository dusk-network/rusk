// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod wrapped;

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, RwLock},
};

use dusk_core::abi::ContractId;
use dusk_wasmtime::{AsContextMut, Config, Engine, Instance, Module, Store};
use wrapped::WrappedDataDriver;

fn config() -> Config {
    let mut config = Config::new();
    config.macos_use_mach_ports(false);
    config
}

pub type SharedDriverExecutor = Arc<Mutex<DriverExecutor>>;

/// Holds the Wasmtime store and instances
pub struct DriverExecutor {
    store: Arc<RwLock<Store<()>>>,
    instances: BTreeMap<ContractId, Instance>,
}

impl DriverExecutor {
    pub fn new() -> Self {
        let config = config();
        let engine = Engine::new(&config)
            .expect("Wasmtime engine configuration should be valid");
        let store = Store::<()>::new(&engine, ());
        let instances = BTreeMap::new();
        let store = Arc::new(RwLock::new(store));
        Self { store, instances }
    }

    pub fn load_bytecode(
        &mut self,
        contract_id: &ContractId,
        bytecode: impl AsRef<[u8]>,
    ) -> anyhow::Result<()> {
        let mut store = self.store.write().unwrap();
        let module = Module::new(store.engine(), bytecode.as_ref())?;
        let instance = Instance::new(store.as_context_mut(), &module, &[])?;
        self.instances.insert(*contract_id, instance);
        Ok(())
    }

    pub fn get_driver<'a>(
        &'a self,
        contract_id: &ContractId,
    ) -> Option<WrappedDataDriver<'a>> {
        let instance = self.instances.get(contract_id)?;
        Some(WrappedDataDriver {
            exec: self,
            id: *contract_id,
        })
    }
}
