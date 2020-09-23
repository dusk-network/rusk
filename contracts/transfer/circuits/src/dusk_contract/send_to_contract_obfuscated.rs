// // This Source Code Form is subject to the terms of the Mozilla Public
// // License, v. 2.0. If a copy of the MPL was not distributed with this
// // file, You can obtain one at http://mozilla.org/MPL/2.0/.
// //
// // Copyright (c) DUSK NETWORK. All rights reserved.

// XXX: THIS GADGET NEEDS REFACTORING WHEN THE GITBOOK SPECS ARE READY

// pub fn withdraw_from_contract_obfuscated(
//     composer: &mut StandardComposer,
//     commitment_crossover: JubJubExtended, 
//     commitment_crossover_value: AllocatedScalar,
//     commitment_crossover_blinder: AllocatedScalar,
//     message_commitment: JubJubExtended,
//     message_commitment_value: AllocatedScalar,
//     message_commitment_blinder: AllocatedScalar,
//     one_time_pk: JubJubExtended,
// ) {
    
//     commitment(composer, commitment_crossover_value, commitment_crossover_blinder, AffinePoint::from(commitment_crossover));
//     commitment(composer, message_commitment_value, message_commitment_blinder, AffinePoint::from(smessage_commitment));

//     range(composer, commitment_crossover_value, 64);
//     range(composer, message_commitment_value, 64);

//     /// XXX: need to have symmetric encryption to be able
//     /// to 'Prove that the encrypted value of the opening 
//     /// of the commitment of the Message, M, is within 
//     /// correctly encrypted to pk.'
// }