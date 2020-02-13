# Protocol Documentation

## Table of Contents

- [state_transition.proto](#state_transition.proto)
    - [StateTransitionExecutionRequest](#rusk.StateTransitionExecutionRequest)
    - [StateTransitionExecutionResponse](#rusk.StateTransitionExecutionResponse)
    - [StateTransitionValidationRequest](#rusk.StateTransitionValidationRequest)
    - [StateTransitionValidationResponse](#rusk.StateTransitionValidationResponse)

- [transaction.proto](#transaction.proto)
    - [Transaction](#rusk.Transaction)
    - [TransactionItem](#rusk.TransactionItem)

- [Scalar Value Types](#scalar-value-types)

## state_transition.proto

### StateTransitionExecutionRequest

| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| txs | [Transaction](#rusk.Transaction) | repeated |  |

### StateTransitionExecutionResponse

| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| txs | [Transaction](#rusk.Transaction) | repeated |  |
| success | [bool](#bool) |  |  |

### StateTransitionValidationRequest

| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| txs | [Transaction](#rusk.Transaction) | repeated |  |

### StateTransitionValidationResponse

| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| txs | [Transaction](#rusk.Transaction) | repeated |  |
| success | [bool](#bool) |  |  |

## transaction.proto

### Transaction

| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| items | [TransactionItem](#rusk.TransactionItem) | repeated |  |
| fee | [fixed64](#fixed64) |  |  |

### TransactionItem

| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| sender | [bytes](#bytes) |  |  |
| recipient | [bytes](#bytes) |  |  |
| value | [fixed64](#fixed64) |  |  |
