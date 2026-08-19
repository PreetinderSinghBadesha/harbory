use harbory_common::keypair::Keypair;
use harbory_protocol::v1::{pairing_service_server::PairingService, RegisterRequest, RegisterResponse};
use tonic::{Request, Response, Status};

use crate::store::{RegisterError, Store};

pub struct PairingServiceImpl {
    pub store: Store,
    pub signer: Keypair,
}

#[tonic::async_trait]
impl PairingService for PairingServiceImpl {
    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let req = request.into_inner();

        let public_key: [u8; 32] = req
            .public_key
            .as_slice()
            .try_into()
            .map_err(|_| Status::invalid_argument("public_key must be exactly 32 bytes"))?;

        let outcome = self
            .store
            .register_agent(&self.signer, &req.pairing_token, public_key)
            .await
            .map_err(|err| {
                tracing::warn!(?err, "pairing registration rejected");
                match err {
                    RegisterError::InvalidToken
                    | RegisterError::TokenAlreadyUsed
                    | RegisterError::TokenExpired => {
                        // Deliberately uniform message: don't tell a caller
                        // probing tokens whether one exists, was already
                        // used, or expired. The audit log keeps the detail.
                        Status::permission_denied("invalid or expired pairing token")
                    }
                    RegisterError::Db(_) => Status::internal("internal error"),
                }
            })?;

        Ok(Response::new(RegisterResponse {
            agent_id: outcome.agent_id.to_string(),
            account_id: outcome.account_id.to_string(),
            credential: outcome.credential,
            control_plane_public_key: self.signer.public_key_bytes().to_vec(),
        }))
    }
}
