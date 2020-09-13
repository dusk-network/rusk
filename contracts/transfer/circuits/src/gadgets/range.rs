// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details. 

use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::jubjub::{
    Fr, AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::*;

/// This gadget simply wraps around the composer's `range_gate` function,
/// but takes in any type that implements the [`TransactionItem`] trait,
/// for ease-of-use in circuit construction.
pub fn range(composer: &mut StandardComposer, value: u64) {
    let value = composer.add_input(BlsScalar::from(value));
    
    composer.range_gate(value, 64);
}