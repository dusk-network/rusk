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
            "cfebfdcd309a070b44e1b407b7228ca9b900720e7cff283d653400357161899a"
        );
        test_circuit!(
            SendToContractObfuscatedCircuit,
            "d7fbe016d385b7d3b44c510225388a0f2a9889d07294ba3e3f9c037801d3148e"
        );
        test_circuit!(
            WithdrawFromTransparentCircuit,
            "d0b52061b33cb2f2ef79448b53cd3d2dbca30819ca4a55e151c8af01e6c7efcd"
        );
        test_circuit!(
            WithdrawFromObfuscatedCircuit,
            "7824ae42a6208eb0eca9f7c5e7ca964efa04a500fc3275e1c89541a26876808a"
        );
        test_circuit!(
            ExecuteCircuitOneTwo<(), TRANSFER_TREE_DEPTH, A>,
            "1aed4ea248e24d6eb71ca40dbc8aca90e1972f0c08cce0666df248e14627d299"
        );
        test_circuit!(
            ExecuteCircuitTwoTwo<(), TRANSFER_TREE_DEPTH, A>,
            "90369a00165fcf91b792bf6d64deaf39f5a16603588fe711838e1005e58458a6"
        );
        test_circuit!(
            ExecuteCircuitThreeTwo<(), TRANSFER_TREE_DEPTH, A>,
            "942a788cf56d9ef93bda7385e86e8620b127bb47eac46829f81bc48e61bdf00e"
        );
        test_circuit!(
            ExecuteCircuitFourTwo<(), TRANSFER_TREE_DEPTH, A>,
            "076cdf6a1f160432941ac3cb14f8dece2c07da58559af4dfdda32b9be5cca884"
        );
    }
}
