// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

export class TxData {
    memo;
    fn_name;
    fn_args;
    contract_id;

    constructor(memo, fn_name, fn_args, contract_id) {
        this.memo = memo;
        this.fn_name = fn_name;
        this.fn_args = fn_args;
        this.contract_id = contract_id;
    }
}
