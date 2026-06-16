# Data Driver for the Transfer Contract

This module provides data-driver implementation for the Transfer Contract.
As described in the README for the data-driver module, this module
implements only the `ConvertibleContract` interface, delegating the all
other tasks to the generic data-driver, which is included into this module
as dependency.

Please refer to data-drivers/data-driver/README.md for more information.

## Scope

This driver intentionally targets the externally consumed transfer surface
(transaction calls, public queries, and selected feeder queries/events).

The transfer contract also exposes internal management entrypoints used by
consensus/state-transition flows. Those internal entrypoints are intentionally
not modeled in this driver.

Some feeder outputs are also intentionally left unsupported where a stable,
consumer-facing schema has not been committed yet (`opening`,
`leaves_from_height`, `leaves_from_pos`, and `sync` outputs).

## How to build the Transfer Contract Data Driver

The following command builds the data driver:
`make wasm-js`

The command will build the driver which will also include memory allocation and de-allocation.

From the root folder this can be achieved by:
`make data-drivers-js`
