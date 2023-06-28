// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#[cfg(test)]
mod tests {
    use phoenix_core::transaction::TRANSFER_TREE_DEPTH;
    use transfer_circuits::*;

    const A: usize = 4;

    macro_rules! test_circuit {
        ($circuit:ty, $expected_id:literal) => {
            let expected_id =
                hex::decode($expected_id).expect("Cannot decode hex");
            assert_eq!(
                <$circuit>::circuit_id(),
                &expected_id[..],
                "Check failed for {} circuit",
                std::any::type_name::<$circuit>()
            );
        };
    }

    #[test]
    fn circuits_id() {
        // This test is required to explicitly check that circuits ID are the
        // one expected.
        //
        // When a circuit id change, it should be noticed with a compiler error
        // because the circuits key file are renamed. But this error is
        // not raised if the `make keys` command is configured to preserve old
        // keys (like the one launched by the CI)

        test_circuit!(
            SendToContractTransparentCircuit,
            "9d267dfe1d1ede4f2ffa35c3609f8662cd84e4df1066b2185a0f3b5b17721c79"
        );
        test_circuit!(
            SendToContractObfuscatedCircuit,
            "c8c7d7fa2fe8eeabd5505056ae3c00b44c1aa13d9578eeff3a4fc7ddb3035da4"
        );
        test_circuit!(
            WithdrawFromTransparentCircuit,
            "dcc4561c1bbd8a10cd14c9e826d51373567dd41bb2cfd498f92230abc602ed47"
        );
        test_circuit!(
            WithdrawFromObfuscatedCircuit,
            "8f7301b53f3af3eb14563c7e474a539a6e12c1248e1e9bdb4b07eeb2ef1a8f2e"
        );
        test_circuit!(
            ExecuteCircuitOneTwo<(), TRANSFER_TREE_DEPTH, A>,
            "4d5e60c2cdb7b3f273649487ad277eb0e380e44dd2f2effb0d2dcb3c1ff615d4"
        );
        test_circuit!(
            ExecuteCircuitTwoTwo<(), TRANSFER_TREE_DEPTH, A>,
            "77d27ac80d397cfec7d621e61af4fa4b7fb4b9e503fa347082c5e1e187e08d48"
        );
        test_circuit!(
            ExecuteCircuitThreeTwo<(), TRANSFER_TREE_DEPTH, A>,
            "4fb4e239548c5bdf9f5c6125cd07da64ce70edb99e79478f13140b53f136c441"
        );
        test_circuit!(
            ExecuteCircuitFourTwo<(), TRANSFER_TREE_DEPTH, A>,
            "05fb339e4fb471c745c8f90181a349ccf9226d8ee719073d45986fadcde466b4"
        );
    }
}
