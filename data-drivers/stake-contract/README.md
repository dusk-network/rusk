
# Data Driver for the Stake Contract

This module provides data-driver implementation for the Stake Contract.
As described in the README for the data-driver module, this module
implements only the `ConvertibleContract` interface, delegating the all
other tasks to the generic data-driver, which is included into this module
as dependency.

Please refer to data-drivers/data-driver/README.md for more information.

## How to build the Stake Contract Data Driver

The following command builds the data driver:
`make wasm-js`

The command will build the driver which will also include memory allocation and de-allocation.

From the root folder this can be achieved by:
`make data-drivers-js`
