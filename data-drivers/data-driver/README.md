

# Data Drivers Overview

In this module, we provide data drivers whose task is to ease communication with Dusk smart contracts
for JavaScript clients.
By a data driver we mean here a separate Web Assembly module which provides methods converting
JavaScript arguments into a form which is understood by Dusk smart contracts,
and also converting Dusk smart contracts outputs into a form which is understood
by JavaScript.

Data drivers do not call Dusk smart contracts. A particular data driver is aware of methods of the corresponding 
Dusk smart contract, yet it does not call them.

Each data driver has its assigned Dusk smart contract, yet it is important to
be aware that data driver for a contract does not call its methods, although it is aware of them
and of their arguments and outputs.

# Module data-driver

Module data-driver contains core code to be used by all drivers. It can be thought of as a framework to
be used by concrete drivers, which are written for particular contracts. 
Module data-driver makes it easy for the authors of concrete drivers to create a driver. 
The authors need just to implement one trait, the complexity is delegated to module data-driver.

Module data-driver provides functionality which make it easy to call drivers from both Rust and JavaScript. 
In order to achieve that it does not use wasm-bindgen, but rather provides a set methods which are easy to call 
from modules written in both languages. The methods are declared with no argument mangling, making them callable
from within any language which supports "C"-like calling conventions.

The following methods are provided by module data-driver:

```

alloc(size: usize) -> *mut u8

dealloc(ptr: *mut u8, size: usize)

get_last_error(...) -> ErrorCode

encode_input_fn(...) -> ErrorCode

decode_input_fn(...) -> ErrorCode

decode_output_fn(...) -> ErrorCode

decode_event(...) -> ErrorCode

get_schema(...) -> ErrorCode

get_version(...) -> ErrorCode

```

In the following sections we will describe the above methods grouped by their areas of responsibility.

## Memory Allocation Methods

Methods `alloc` and `dealloc` are needed for both JavaScript and Rust callers (and callers from
other languages as well) to pass parameters to other methods. Typical driver method accepts the
following parameters:

```
    fn_name_ptr: *mut u8,
    fn_name_size: usize,
    data_ptr: *mut u8,
    data_size: usize,
    out_ptr: *mut u8,
    out_buf_size: usize,
```
In order to pass method name, caller must allocate a buffer, copy the name to it, and pass
a pointer to the buffer and its length. Similarly, with input data and with output data.
To allocate a buffer, caller needs to use `alloc` method which will return a buffer offset
in Wasm memory. Note that `alloc` and `dealloc` methods deal with offsets, they do not provide
absolute memory locations but rather offsets which need to be added to the pointer to Wasm
memory segment. This can be easily done in both JavaScript and Rust. First, Wasm memory pointer
needs to be obtained, then, alloc is called and the result of its call needs to be added
to memory pointer. Only then such pointer can be used as buffer for arguments for the remaining
driver methods. Once the buffer is no longer needed, `dealloc` should be used to free memory.

## Argument encoding and decoding methods

Here by encoding we mean converting the arguments from JSON to rkyv, a serialization format which
Dusk smart contracts understand. By decoding we mean the opposite conversion.

There are four methods in this group, so lets consider each one of them in turn:

### encode_input_fn

Parameters:
```
    fn_name_ptr: *mut u8,
    fn_name_size: usize,
    json_ptr: *mut u8,
    json_size: usize,
    out_ptr: *mut u8,
    out_buf_size: usize,
```

This method accepts method name and a buffer containing arguments in JSON form.
On return, it fills out the output buffer with output length (first 4 bytes, as a big-endian 32-bit number),
and then it fills out the subsequent bytes of the buffer with rkyv serialization ready to be used when 
making a call to a given smart contract method.
Caller needs to read first 4 bytes of the output buffer first to know the returned data length.
Say, first 4 bytes contained number N. The caller then needs to read the remaining N bytes of rkyv
serialization returned. Assuming that the method name passed to `encode_input_fn` was M, 
the caller can pass the obtained data to smart contract method M as its arguments.

### decode_input_fn

Parameters:
```
    fn_name_ptr: *mut u8,
    fn_name_size: usize,
    rkyv_ptr: *const u8,
    rkyv_len: usize,
    out_ptr: *mut u8,
    out_buf_size: usize,
```

This method does the opposite to what `encode_input_fn` does, it converts arguments in form of an
rkyv serialization string into JSON. Other calling details are the same as in `encode_input_fn`.

### decode_output_fn

Parameters:
```
    fn_name_ptr: *mut u8,
    fn_name_size: usize,
    rkyv_ptr: *const u8,
    rkyv_len: usize,
    out_ptr: *mut u8,
    out_buf_size: usize,
```

This method does the same conversion as `decode_input_fn`, yet it is performed on method's output
rather than input. Other calling details are the same as in `encode_input_fn` and `decode_input_fn`.

### decode_event

Parameters:
```
    event_name_ptr: *mut u8,
    event_name_size: usize,
    rkyv_ptr: *const u8,
    rkyv_len: usize,
    out_ptr: *mut u8,
    out_buf_size: usize,
```

This method converts event from rkyv-serialized form into JSON. Unlike other argument encoding/decoding methods,
this method accepts event name rather than method name.

All the above methods make it possible to:
- convert JSON arguments to rkyv bytes ready to be used in smart contract method calls
- convert rkyv bytes returned by smart contract method calls into JSON
- convert events emitted by smart contract into JSON

Concrete smart contract driver developer needs only to implement one trait named ConvertibleContract,
in order to implement the above functionality. Note that ConvertibleContract is much simplified compared
to the above, more about it in the following sections.

## Miscellaneous methods

Method get_last_error() allows obtaining detailed error description when ErrorCode::OperationError is returned

Method get_schema() should return JSON schema describing all contract calls. This feature is not used yet
and in current implementation an empty string is returned.

Method get_version() returns versions of the driver. Currently, versioning is up to the discretion of 
driver's author.

## Trait ConvertibleContract - to be used by drivers' creators

Fortunately, the above functions with their rather complex parameters are to be used only internally, by this module
(i.e. module data-driver) and by the infrastructure. Regular smart contract authors are dealing 
only with a much simplified trait named `ConvertibleContract`. The trait is a Rust concept and 
its methods are to be implemented in Rust, yet thanks to 
the architecture enabled by the module data-driver, the driver which implements `ConvertibleContract` and uses
module data-driver will provide arguments and return values translations to both JavaScript and Rust users.

Smart contract author needs to implement the following methods of trait `ConvertibleContract`:

```
    fn encode_input_fn(&self, fn_name: &str, json: &str) -> Result<Vec<u8>, Error>;
    fn decode_input_fn(&self, fn_name: &str, rkyv: &[u8]) -> Result<JsonValue, Error>;
    fn decode_output_fn(&self, fn_name: &str, rkyv: &[u8]) -> Result<JsonValue, Error>;
    fn decode_event(&self, event_name: &str, rkyv: &[u8]) -> Result<JsonValue, Error>;
    fn get_schema(&self) -> String;
    fn get_version(&self) -> &'static str
```

As you can see, thanks to the fact that the trait is a Rust concept and confined within the the realm
of a single language, the parameters are much simplified. Module data-driver provides a set of 
conversion methods which make implementing the trait ConvertibleContract easy.
To such helper functions, provided by module data-driver, belong:

```
    json_to_rkyv
    rkyv_to_json
    from_rkyv
    json_to_rkyv_u64
    rkyv_to_json_u64  
    json_to_rkyv_pair_u64
    rkyv_to_json_pair_u64
```

Example ConvertibleContract implementation may look as follows:
(assuming that the contract has only one method `deposit_to` and one event
`Withdrawal`)

```
impl ConvertibleContract for ContractDriver {
    fn encode_input_fn(
        &self,
        fn_name: &str,
        json: &str,
    ) -> Result<Vec<u8>, Error> {
        match fn_name {
            "deposit_to" => {
                json_to_rkyv::<(EVMAddress, u64, u64, Vec<u8>)>(json)
            }
            // Unsupported
            name => Err(Error::Unsupported(format!("fn_name {name}"))),
        }
    }

    fn decode_input_fn(
        &self,
        fn_name: &str,
        rkyv: &[u8],
    ) -> Result<JsonValue, Error> {
        match fn_name {
            "deposit_to" => {
                rkyv_to_json::<(EVMAddress, u64, u64, Vec<u8>)>(rkyv)
            }
            // Unsupported
            name => Err(Error::Unsupported(format!("fn_name {name}"))),
        }
    }

    fn decode_output_fn(
        &self,
        fn_name: &str,
        rkyv: &[u8],
    ) -> Result<JsonValue, Error> {
        match fn_name {
            "deposit_to" => Ok(JsonValue::Null),
            "finalize_withdrawal" => Ok(JsonValue::Null),
            // Unsupported
            name => Err(Error::Unsupported(format!("fn_name {name}"))),
        }
    }

    fn decode_event(
        &self,
        event_name: &str,
        rkyv: &[u8],
    ) -> Result<JsonValue, Error> {
        match event_name {
            "Withdrawal" => {
                rkyv_to_json::<events::Withdrawal>(rkyv)
            }
            event => Err(Error::Unsupported(format!("event {event}"))),
        }
    }

    fn get_schema(&self) -> String {
        "".to_string()
    }
}
```

When your ConvertibleContract implementation is compiled to Wasm, it is injected into
module data-driver, which is a part of your Wasm. The module data-driver does all the heavy-lifting,
yet it calls your ConvertibleContract implementation to do the actual conversion. Your implementation,
in turn, uses helper functions provided by the module, thus, forming only a thin slice inside
this sandwich design. Such architecture makes writing drivers easy.

## Examples of equivalent JavaScript and Rust arguments

What seems to be sometimes hard is to come up with JavaScript equivalents of some Rust argument types.
Let's consider the following variants of Dusk smart contract argument types:

### Single primitive argument

Consider the case of a single primitive argument.
Let's say we have a function accepting one argument of type dusk_core::signatures::bls::PublicKey.
In such case, example JSON value could be as follows:

`tCR9c1pQU1jC5QgmRi3JRb2g1Rhtrc6AxT24VQPtMY3wrsuRrnMBMP6wSoKWXH2opKTeCm5aniEG2HH8ATcUHzeWe6814e8qdECGvLLZvhaRKsi7MJgLAA33PiWZ4b6ptNt`

### Single object/structural argument
In case of a single object argument of a function, a Rust struct, an example JSON representation could be as follows:

```
{
    "chain_id": 1,
    "keys": {
      "account": "tCR9c1pQU1jC5QgmRi3JRb2g1Rhtrc6AxT24VQPtMY3wrsuRrnMBMP6wSoKWXH2opKTeCm5aniEG2HH8ATcUHzeWe6814e8qdECGvLLZvhaRKsi7MJgLAA33PiWZ4b6ptNt",
      "owner": {
        "Account": "tCR9c1pQU1jC5QgmRi3JRb2g1Rhtrc6AxT24VQPtMY3wrsuRrnMBMP6wSoKWXH2opKTeCm5aniEG2HH8ATcUHzeWe6814e8qdECGvLLZvhaRKsi7MJgLAA33PiWZ4b6ptNt"
      }
    },
    "signature": {
      "account": "7kP8oaxopsWi7g6kNGtX3PHVekMF8RKRRx74tqoo1xLmh2zGVN2FmJ5EFg7UJV9stk",
      "owner": "7kP8oaxopsWi7g6kNGtX3PHVekMF8RKRRx74tqoo1xLmh2zGVN2FmJ5EFg7UJV9stk"
    },
    "value": "4014086097495"
}
```

The above JSON structure corresponds to the following Rust struct:

```
pub struct Stake {
    chain_id: u8,
    keys: StakeKeys,
    value: u64,
    signature: DoubleSignature,
}
```
where StakeKeys is:
```
pub struct StakeKeys {
    pub account: BlsPublicKey,
    pub owner: StakeFundOwner,
```
and DoubleSignature is:
```
pub struct DoubleSignature {
    pub account: BlsSignature,
    pub owner: BlsSignature,
}
```
As you can see, JSON representation does not include object name, only its named members.

### Multiple primitive arguments

In case of multiple primitive elements, say, a function which, for example, accepts two arguments, 
say, u64 and u32. An example JSON could look as follows:

`[ 22, 33 ]`

Multiple arguments, which in Rust will have different parameter names, are represented in JSON
as members of an array. Parameters' names are not represented in JSON at all.

### Multiple mixed primitive/structural arguments

In case smart contract method accepts a mix of structural and primitive arguments, or in general, multiple
arguments, they are represented in JSON as an array. Assuming we have a function which accepts a struct Stake
as above, followed by a u32 number, an example JSON representation could be as follows:

```
[
  {
    "chain_id": 1,
    "keys": {
      "account": "tCR9c1pQU1jC5QgmRi3JRb2g1Rhtrc6AxT24VQPtMY3wrsuRrnMBMP6wSoKWXH2opKTeCm5aniEG2HH8ATcUHzeWe6814e8qdECGvLLZvhaRKsi7MJgLAA33PiWZ4b6ptNt",
      "owner": {
        "Account": "tCR9c1pQU1jC5QgmRi3JRb2g1Rhtrc6AxT24VQPtMY3wrsuRrnMBMP6wSoKWXH2opKTeCm5aniEG2HH8ATcUHzeWe6814e8qdECGvLLZvhaRKsi7MJgLAA33PiWZ4b6ptNt"
      }
    },
    "signature": {
      "account": "7kP8oaxopsWi7g6kNGtX3PHVekMF8RKRRx74tqoo1xLmh2zGVN2FmJ5EFg7UJV9stk",
      "owner": "7kP8oaxopsWi7g6kNGtX3PHVekMF8RKRRx74tqoo1xLmh2zGVN2FmJ5EFg7UJV9stk"
    },
    "value": "4014086097495"
  }, 
  33
]
```
In the above example, we can see JSON representation of a Stake object and a number 33, being passed as two named
parameters to a smart contract method. Parameter names are ignored in serialization formats, as Dusk smart
contract methods assume that all input function parameters form a tuple. A tuple is represented as an array in JSON.

### Output values

Output values are always single values, and the above information about JSON representation of Rust
arguments is valid also for output values. In Rust, multiple output values can be embedded into a tuple, forming
a single aggregated value. Such value will be represented in JSON as an array of elements, each of which will be
represented in JSON in an analogous way to the above examples.
