# Protocol Documentation

## Table of Contents

- [rusk.proto](#rusk.proto)
    - [EchoRequest](#rusk.EchoRequest)
    - [EchoResponse](#rusk.EchoResponse)
    - [ValidateStateTransitionRequest](#rusk.ValidateStateTransitionRequest)
    - [ValidateStateTransitionResponse](#rusk.ValidateStateTransitionResponse)
    - [Rusk](#rusk.Rusk)

## rusk.proto

### EchoRequest

### EchoResponse

### ValidateStateTransitionRequest

| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| txs | [phoenix.Transaction](#phoenix.Transaction) | repeated | List of transactions to be validated |

### ValidateStateTransitionResponse

| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| success | [bool](#bool) |  |  |

### Rusk

| Method Name | Request Type | Response Type | Description |
| ----------- | ------------ | ------------- | ------------|
| Echo | [EchoRequest](#rusk.EchoRequest) | [EchoResponse](#rusk.EchoResponse) |  |
| ValidateStateTransition | [ValidateStateTransitionRequest](#rusk.ValidateStateTransitionRequest) | [ValidateStateTransitionResponse](#rusk.ValidateStateTransitionResponse) |  |
