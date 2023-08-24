// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#[cfg(test)]
mod tests {
    use transfer_circuits::*;

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
            ExecuteCircuitOneTwo,
            "cff6ae2993e629cffb5b9b6fb04e368e64f79cd2f8bd3fc6095cedbbfd5cdc1d"
        );
        test_circuit!(
            ExecuteCircuitTwoTwo,
            "2b987ac4bcb3eeda279b5c1e36018f9537db02e0a6d55f8b46c608b9690c3a1e"
        );
        test_circuit!(
            ExecuteCircuitThreeTwo,
            "51846c23e307b4d2904230ff14acaa1af7b032867065cbcb5c693d8ff8cb6063"
        );
        test_circuit!(
            ExecuteCircuitFourTwo,
            "f2a04c3a1de344ba9f52cc6693b569a2a5b683871e1fb58d1845a1a11a8a5542"
        );
    }
}
