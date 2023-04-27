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
            ExecuteCircuitOneTwo,
            "19c9391f2f03a5206caac2618b8ab32847b6a1e19500fec27a3a96b9a84b200c"
        );
        test_circuit!(
            ExecuteCircuitTwoTwo,
            "ea59814e99b4c8789cff85d6623749f823c56383e300761537b3e248c537a033"
        );
        test_circuit!(
            ExecuteCircuitThreeTwo,
            "4e03eb1686949f9f17d13d285a4a9c5bc9596a84765f36a3491a981a29135987"
        );
        test_circuit!(
            ExecuteCircuitFourTwo,
            "2a34871c45dd993c6217199c5c000aff24621f5953aca3a1755fe052a8e4e7b9"
        );
    }
}
