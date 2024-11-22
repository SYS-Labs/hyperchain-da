use crate::client::SyscoinClient;
use zksync_da_client::DataAvailabilityClient;
use zksync_node_framework::implementations::resources::da_client::DAClientResource;
use zksync_node_framework::{
    wiring_layer::{WiringError, WiringLayer},
    IntoContext,
};

#[derive(Debug)]
pub struct SyscoinWiringLayer {}

impl SyscoinWiringLayer {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Debug, IntoContext)]
pub struct SyscoinOutput {
    pub client: DAClientResource,
}

#[async_trait::async_trait]
impl WiringLayer for SyscoinWiringLayer {
    type Input = ();
    type Output = SyscoinOutput;

    fn layer_name(&self) -> &'static str {
        "syscoin_client_layer"
    }

    async fn wire(self, _input: Self::Input) -> Result<Self::Output, WiringError> {
        let client: Box<dyn DataAvailabilityClient> = Box::new(SyscoinClient::new());

        Ok(Self::Output {
            client: DAClientResource(client),
        })
    }
}