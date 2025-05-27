mod mem;
mod error;

use rkyv::to_bytes;
use std::ptr;
use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;

#[no_mangle]
pub unsafe extern "C" fn accounts_into_raw(
    accounts_ptr: *const u8,
    raws_ptr: *mut *mut u8,
) -> error::ErrorCode {
    let bytes: Vec<u8> = mem::read_buffer(accounts_ptr)
        .chunks(BlsPublicKey::SIZE)
        .map(BlsPublicKey::from_slice)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| error::ErrorCode::DeserializationError)?
        .into_iter()
        .map(|bpk| to_bytes::<_, 256>(&bpk))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| error::ErrorCode::ArchivingError)?
        .iter()
        .fold(Vec::new(), |mut vec, aligned| {
            vec.extend_from_slice(aligned.as_slice());
            vec
        });

    let len = bytes.len().to_le_bytes();
    let ptr = mem::allocate(4 + bytes.len() as u32);

    *raws_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());

    error::ErrorCode::Ok
}
