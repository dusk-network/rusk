// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod panic_handling {
    use core::panic::PanicInfo;

    #[panic_handler]
    #[allow(unused)]
    #[cfg(target_family = "wasm")]
    fn panic(info: &PanicInfo) -> ! {
        #[cfg(debug_assertions)]
        eprintln!("{}", info);

        loop {}
    }
}
