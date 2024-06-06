use tonic::{Request, Status};
use tonic::service::Interceptor;

#[derive(Debug, Clone)]
pub enum Credentials {
    None,
    Jwt(String),
}

impl Default for Credentials {
    fn default() -> Self {
        Credentials::unauthneticated()
    }
}

impl Credentials {

    ///
    /// Do nothing
    pub fn unauthneticated() -> Self {
        Credentials::None
    }

    ///
    /// Authenticate using a JWT token, i.e., by putting it in the `Authorization` header
    pub fn jwt<S: ToString>(jwt: S) -> Self {
        Credentials::Jwt(jwt.to_string())
    }
}

impl Interceptor for Credentials {
    fn call(&mut self, request: Request<()>) -> Result<Request<()>, Status> {
        match self {
            Credentials::None => Ok(request),
            Credentials::Jwt(s) => {
                let (mut meta, ext, m) = request.into_parts();
                let header_value  = format!("Bearer {}", s);
                meta.insert("authorization", header_value.as_str().parse().unwrap());
                Ok(Request::from_parts(meta, ext, m))
            }
        }
    }
}
