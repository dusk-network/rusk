// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details. 

// use phoenix_core::{Note, TransactionItem, ViewKey};

use dusk_plonk::constraint_system::StandardComposer;
use dusk_plonk::constraint_system::ecc::{Point, PointScalar};
use dusk_plonk::constraint_system::ecc::curve_addition::fast_add;
use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::constraint_system::{variable::Variable, StandardComposer};
use dusk_jubjub::{GENERATOR_EXTENDED, ExtendedPoint, Fr, GENERATOR_NUMS, GENERATOR_NUMS_EXTENDED};

/// Prove knowledge of the value and blinding factor, which make up the value commitment.
/// This commitment gadget is using the pedersen commitments.
pub fn commitment(composer: &mut StandardComposer, value: Fr, blinder: Fr) {

    let value_point = scalar_mul(composer, value, GENERATOR_EXTENDED);
    let blinder_point = scalar_mul(composer, blinder, GENERATOR_NUMS_EXTENDED);

    let commitment = value_point.Point.fast_add(&self, composer, blinder_point.Point);

}


