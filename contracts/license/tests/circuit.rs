// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#[cfg(test)]
mod tests {
    use license_circuits::*;

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
    fn license_circuit_id() {
        // This test is required to explicitly check that circuits ID are the
        // one expected.
        //
        // When a circuit id change, it should be noticed with a compiler error
        // because the circuits key file are renamed. But this error is
        // not raised if the `make keys` command is configured to preserve old
        // keys (like the one launched by the CI)

        test_circuit!(
            LicenseCircuit,
            "f0aaeb94d6e80c1a02c9bb339241c730291a287eb6f73272c5474da71c2f6589"
        );
    }
}
