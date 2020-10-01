use super::super::ServiceRequestHandler;
use super::{GenerateScoreRequest, GenerateScoreResponse};
use crate::encoding::decode_request_param;
use dusk_blindbid::bid::Bid;
use dusk_plonk::bls12_381::Scalar as BlsScalar;
use dusk_plonk::jubjub::AffinePoint as JubJubAffine;
use tonic::{Code, Request, Response, Status};
/// Implementation of the ScoreGeneration Handler.
pub struct ScoreGenHandler<'a> {
    request: &'a Request<GenerateScoreRequest>,
}

impl<'a, 'b>
    ServiceRequestHandler<'a, 'b, GenerateScoreRequest, GenerateScoreResponse>
    for ScoreGenHandler<'a>
where
    'b: 'a,
{
    fn load_request(request: &'b Request<GenerateScoreRequest>) -> Self {
        Self { request }
    }

    fn handle_request(
        &self,
    ) -> Result<Response<GenerateScoreResponse>, Status> {
        // Parse the optional request fields and return an error if
        // any of them is missing since all are required to compute
        // the score and the blindbid proof.
        let (k, seed, secret) = parse_score_gen_params(self.request)?;
        // Get bid from storage
        let (bid, bid_tree_root) = get_bid_from_tree(
            self.request.get_ref().index_stored_bid as usize,
        )?;

        // Generate Score for the Bid
        let round = BlsScalar::from(self.request.get_ref().round as u64);
        let step = BlsScalar::from(self.request.get_ref().step as u64);
        let score = match bid.compute_score(
            &secret,
            k,
            bid_tree_root,
            seed,
            round,
            step,
        ) {
            Ok(score) => score,
            Err(e) => return Err(Status::new(Code::Unknown, format!("{}", e))),
        };
        // Generate Prover ID
        let prover_id = bid.generate_prover_id(k, seed, round, step);
        // Generate Blindbid proof proving that the generated `Score` is correct.
        unimplemented!()
    }
}

// Parses the optional inputs of the GenerateScoreRequest returning an error if
// any of them isn't present (is `None`).
fn parse_score_gen_params(
    request: &Request<GenerateScoreRequest>,
) -> Result<(BlsScalar, BlsScalar, JubJubAffine), Status> {
    let k: BlsScalar =
        decode_request_param(request.get_ref().k.as_ref())?.try_into()?;
    let seed: BlsScalar =
        decode_request_param(request.get_ref().seed.as_ref())?.try_into()?;
    let secret: JubJubAffine =
        decode_request_param(request.get_ref().secret.as_ref())?.try_into()?;
    Ok((k, seed, secret))
}

fn gen_bid_for_storage() -> Bid {
    unimplemented!()
}

// This function simulates the obtention of a Bid from the
// Bid contract storage.
fn get_bid_from_tree(idx: usize) -> Result<(Bid, BlsScalar), std::io::Error> {
    use poseidon252::PoseidonTree;
    //let mut tree = PoseidonTree<_, kelvin::Blake2b>::new(17usize);
    let bid = gen_bid_for_storage();
    //tree.push(bid);
    unimplemented!()
}
