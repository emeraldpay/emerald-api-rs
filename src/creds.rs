use tonic::{Request, Status};
use tonic::service::Interceptor;

#[derive(Debug, Clone)]
pub enum Credentials {
    None,
}

impl Default for Credentials {
    fn default() -> Self {
        Credentials::unauthneticated()
    }
}

impl Credentials {
    pub fn unauthneticated() -> Self {
        Credentials::None
    }
}

impl Interceptor for Credentials {
    fn call(&mut self, request: Request<()>) -> Result<Request<()>, Status> {
        Ok(request)
    }
}
