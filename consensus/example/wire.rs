// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use consensus::messages::{Message, Serializable};
use std::io::{self, Read, Write};

/// Wire Frame definition.
#[derive(Debug, Default)]
pub struct Frame {
    header: FrameHeader,
    payload: FramePayload,
}

/// Frame Header definition.
#[derive(Debug, Default)]
struct FrameHeader {
    version: [u8; 8],
    reserved: u64,
    checksum: [u8; 4],
}

/// Frame Payload definition.
#[derive(Debug, Default)]
struct FramePayload(Message);

impl Serializable for FrameHeader {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&self.version[..])?;
        w.write_all(&self.reserved.to_le_bytes())?;
        w.write_all(&self.checksum[..])?;
        Ok(())
    }

    /// Deserialize struct from buf by consuming N bytes.
    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut version = [0u8; 8];
        r.read_exact(&mut version)?;

        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        let reserved = u64::from_le_bytes(buf);

        let mut checksum = [0u8; 4];
        r.read_exact(&mut checksum)?;

        Ok(FrameHeader {
            version,
            reserved,
            checksum,
        })
    }
}

fn calc_checksum(buf: &[u8]) -> [u8; 4] {
    use blake2::{digest::consts::U32, Blake2b, Digest};

    let mut h = Blake2b::<U32>::new();
    h.update(buf);
    let res = h.finalize();

    let mut v = [0u8; 4];
    v.clone_from_slice(&res[0..4]);
    v
}

impl Frame {
    pub fn encode(msg: Message) -> io::Result<Vec<u8>> {
        let mut payload_buf = vec![];
        msg.write(&mut payload_buf)?;

        let mut header = FrameHeader::default();
        header.checksum = calc_checksum(&payload_buf[..]);
        header.version = [0, 0, 0, 0, 1, 0, 0, 0];

        let mut header_buf = vec![];
        header.write(&mut header_buf)?;

        let frame_size = (header_buf.len() + payload_buf.len()) as u64;

        Ok(
            [Vec::from(frame_size.to_le_bytes()), header_buf, payload_buf]
                .concat(),
        )
    }

    pub fn decode<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut buf = [0u8; 8];
        _ = r.read_exact(&mut buf)?;

        let header = FrameHeader::read(r)?;
        let payload = FramePayload(Message::read(r)?);

        Ok(Frame { header, payload })
    }

    pub fn get_topic(&self) -> u8 {
        self.payload.0.header.topic
    }

    pub fn get_msg(&self) -> &Message {
        &self.payload.0
    }
}

#[cfg(test)]
#[allow(unused)]
mod tests {
    use consensus::commons::{Block, Certificate, Topics};
    use consensus::messages::payload::{
        AggrAgreement, Agreement, NewBlock, Reduction, StepVotes,
    };
    use consensus::messages::{self, Header, Message, Serializable};
    use consensus::util::pubkey::ConsensusPublicKey;

    use crate::wire::Frame;

    const FIXED_HASH: [u8; 32] = [
        105, 202, 186, 101, 26, 74, 160, 61, 42, 33, 92, 232, 251, 35, 67, 147,
        73, 198, 100, 5, 115, 67, 61, 212, 81, 61, 185, 60, 118, 99, 152, 143,
    ];

    #[test]
    fn test_new_block_wire_msg() {
        let buf: Vec<u8> = vec![
            94, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 218, 45, 189, 21, 16, 96, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 63, 66, 15,
            0, 0, 0, 0, 0, 255, 105, 202, 186, 101, 26, 74, 160, 61, 42, 33,
            92, 232, 251, 35, 67, 147, 73, 198, 100, 5, 115, 67, 61, 212, 81,
            61, 185, 60, 118, 99, 152, 143, 14, 14, 14, 14, 14, 14, 14, 14, 14,
            14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14,
            14, 14, 14, 14, 14, 14, 0, 200, 0, 0, 0, 0, 0, 0, 0, 30, 143, 169,
            0, 0, 0, 0, 0, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
            10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
            10, 10, 32, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11,
            11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11,
            11, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13,
            13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 12,
            12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12,
            12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12,
            12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12,
            12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12,
            12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12,
            12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 64, 226, 1, 0, 0, 0, 0, 0,
            48, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 48, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 105, 202, 186, 101, 26, 74, 160, 61, 42, 33, 92,
            232, 251, 35, 67, 147, 73, 198, 100, 5, 115, 67, 61, 212, 81, 61,
            185, 60, 118, 99, 152, 143, 0, 48, 15, 15, 15, 15, 15, 15, 15, 15,
            15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15,
            15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15,
            15, 15, 15, 15, 15, 15,
        ];

        let blk_header = consensus::commons::Header {
            version: 0,
            height: 200,
            timestamp: 11112222,
            gas_limit: 123456,
            prev_block_hash: [10; 32],
            seed: [11; 32],
            generator_bls_pubkey: [12; 96],
            state_hash: [13; 32],
            hash,
            cert: Certificate {
                first_reduction: ([0; 48], 0),
                second_reduction: ([0; 48], 0),
                step: 0,
            },
        };

        let candidate = Block::new(blk_header.clone(), vec![])
            .expect("should be valid hash");

        // Check if calculate hash is correct
        assert_eq!(candidate.header.hash, FIXED_HASH);

        // Ensure that the dumped message is properly encoded
        assert_eq!(
            Frame::encode(Message::new_newblock(
                messages::Header {
                    pubkey_bls: ConsensusPublicKey::default(),
                    round: 999999,
                    step: 255,
                    block_hash: candidate.header.hash,
                    topic: Topics::NewBlock as u8,
                },
                NewBlock {
                    prev_hash: [14; 32],
                    candidate,
                    signed_hash: [15; 48],
                },
            ))
            .expect("reduction serialization should be valid"),
            buf
        );
    }

    #[test]
    fn test_reduction_wire_msg() {
        let buf: Vec<u8> = vec![
            208, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 4, 2, 27, 248, 17, 96, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 159, 134, 1, 0,
            0, 0, 0, 0, 123, 105, 202, 186, 101, 26, 74, 160, 61, 42, 33, 92,
            232, 251, 35, 67, 147, 73, 198, 100, 5, 115, 67, 61, 212, 81, 61,
            185, 60, 118, 99, 152, 143, 48, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        ];

        // Ensure that the dumped message is properly encoded
        assert_eq!(
            Frame::encode(Message::new_reduction(
                messages::Header {
                    pubkey_bls: ConsensusPublicKey::default(),
                    round: 99999,
                    step: 123,
                    block_hash: FIXED_HASH,
                    topic: Topics::Reduction as u8,
                },
                Reduction {
                    signed_hash: [1u8; 48]
                },
            ))
            .expect("reduction serialization should be valid"),
            buf
        );
    }

    #[test]
    fn test_agreement_wire_msg() {
        let buf: Vec<u8> = vec![
            67, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 180, 212, 17, 253, 18, 96, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 159, 134,
            1, 0, 0, 0, 0, 0, 123, 105, 202, 186, 101, 26, 74, 160, 61, 42, 33,
            92, 232, 251, 35, 67, 147, 73, 198, 100, 5, 115, 67, 61, 212, 81,
            61, 185, 60, 118, 99, 152, 143, 48, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
            5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
            5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 2, 103, 43, 0, 0,
            0, 0, 0, 0, 48, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 206, 86, 0, 0, 0, 0, 0, 0, 48, 2, 2, 2,
            2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
            2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
            2,
        ];

        // Ensure that the dumped message is properly encoded
        assert_eq!(
            Frame::encode(Message::new_agreement(
                messages::Header {
                    pubkey_bls: ConsensusPublicKey::default(),
                    round: 99999,
                    step: 123,
                    block_hash: FIXED_HASH,
                    topic: Topics::Agreement as u8,
                },
                Agreement {
                    signature: [5u8; 48],
                    first_step: StepVotes {
                        bitset: 11111,
                        signature: [1u8; 48]
                    },
                    second_step: StepVotes {
                        bitset: 22222,
                        signature: [2u8; 48]
                    }
                },
            ))
            .expect("agreement serialization should be valid"),
            buf
        );
    }

    #[test]
    fn test_aggr_agreement_wire_msg() {
        let buf: Vec<u8> = vec![
            124, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 248, 97, 84, 244, 19, 96, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 159, 134, 1,
            0, 0, 0, 0, 0, 123, 105, 202, 186, 101, 26, 74, 160, 61, 42, 33,
            92, 232, 251, 35, 67, 147, 73, 198, 100, 5, 115, 67, 61, 212, 81,
            61, 185, 60, 118, 99, 152, 143, 48, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
            5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
            5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 2, 103, 43, 0, 0,
            0, 0, 0, 0, 48, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 206, 86, 0, 0, 0, 0, 0, 0, 48, 2, 2, 2,
            2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
            2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
            2, 100, 0, 0, 0, 0, 0, 0, 0, 48, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
            7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
            7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
        ];

        // Ensure that the dumped message is properly encoded
        assert_eq!(
            Frame::encode(Message::new_aggr_agreement(
                messages::Header {
                    pubkey_bls: ConsensusPublicKey::default(),
                    round: 99999,
                    step: 123,
                    block_hash: FIXED_HASH,
                    topic: Topics::AggrAgreement as u8,
                },
                AggrAgreement {
                    agreement: Agreement {
                        signature: [5u8; 48],
                        first_step: StepVotes {
                            bitset: 11111,
                            signature: [1u8; 48]
                        },
                        second_step: StepVotes {
                            bitset: 22222,
                            signature: [2u8; 48]
                        }
                    },
                    bitset: 100,
                    aggr_signature: [7u8; 48]
                },
            ))
            .expect("aggr_agreement serialization should be valid"),
            buf
        );
    }
}
