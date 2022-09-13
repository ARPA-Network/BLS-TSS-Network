#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MineRequest {
    #[prost(uint32, tag="1")]
    pub block_number_increment: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MineReply {
    #[prost(uint32, tag="1")]
    pub block_number: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RequestRandomnessRequest {
    #[prost(string, tag="1")]
    pub message: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FulfillRandomnessRequest {
    #[prost(string, tag="1")]
    pub id_address: ::prost::alloc::string::String,
    #[prost(uint32, tag="2")]
    pub group_index: u32,
    #[prost(uint32, tag="3")]
    pub signature_index: u32,
    #[prost(bytes="vec", tag="4")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
    #[prost(map="string, bytes", tag="5")]
    pub partial_signatures: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::vec::Vec<u8>>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SetInitialGroupRequest {
    #[prost(string, tag="1")]
    pub id_address: ::prost::alloc::string::String,
    #[prost(bytes="vec", tag="2")]
    pub group: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FulfillRelayRequest {
    #[prost(string, tag="1")]
    pub id_address: ::prost::alloc::string::String,
    #[prost(uint32, tag="2")]
    pub relayer_group_index: u32,
    #[prost(uint32, tag="3")]
    pub task_index: u32,
    #[prost(bytes="vec", tag="4")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="5")]
    pub group_as_bytes: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CancelInvalidRelayConfirmationTaskRequest {
    #[prost(string, tag="1")]
    pub id_address: ::prost::alloc::string::String,
    #[prost(uint32, tag="2")]
    pub task_index: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ConfirmRelayRequest {
    #[prost(string, tag="1")]
    pub id_address: ::prost::alloc::string::String,
    #[prost(uint32, tag="2")]
    pub task_index: u32,
    #[prost(bytes="vec", tag="3")]
    pub group_relay_confirmation_as_bytes: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="4")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetGroupRequest {
    #[prost(uint32, tag="1")]
    pub index: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GroupReply {
    #[prost(uint32, tag="1")]
    pub index: u32,
    #[prost(uint32, tag="2")]
    pub epoch: u32,
    #[prost(uint32, tag="3")]
    pub capacity: u32,
    #[prost(uint32, tag="4")]
    pub size: u32,
    #[prost(uint32, tag="5")]
    pub threshold: u32,
    #[prost(bool, tag="6")]
    pub state: bool,
    #[prost(bytes="vec", tag="7")]
    pub public_key: ::prost::alloc::vec::Vec<u8>,
    #[prost(btree_map="string, message", tag="8")]
    pub members: ::prost::alloc::collections::BTreeMap<::prost::alloc::string::String, Member>,
    #[prost(string, repeated, tag="9")]
    pub committers: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Member {
    #[prost(uint32, tag="1")]
    pub index: u32,
    #[prost(string, tag="2")]
    pub id_address: ::prost::alloc::string::String,
    #[prost(bytes="vec", tag="3")]
    pub partial_public_key: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SignatureTaskReply {
    #[prost(uint32, tag="1")]
    pub index: u32,
    #[prost(string, tag="2")]
    pub message: ::prost::alloc::string::String,
    #[prost(uint32, tag="3")]
    pub group_index: u32,
    #[prost(uint32, tag="4")]
    pub assignment_block_height: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LastOutputReply {
    #[prost(uint64, tag="1")]
    pub last_output: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetSignatureTaskCompletionStateRequest {
    #[prost(uint32, tag="1")]
    pub index: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetSignatureTaskCompletionStateReply {
    #[prost(bool, tag="1")]
    pub state: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GroupRelayConfirmationTaskReply {
    #[prost(uint32, tag="1")]
    pub index: u32,
    #[prost(uint32, tag="2")]
    pub group_relay_cache_index: u32,
    #[prost(uint32, tag="3")]
    pub relayed_group_index: u32,
    #[prost(uint32, tag="4")]
    pub relayed_group_epoch: u32,
    #[prost(uint32, tag="5")]
    pub relayer_group_index: u32,
    #[prost(uint32, tag="6")]
    pub assignment_block_height: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetGroupRelayCacheRequest {
    #[prost(uint32, tag="1")]
    pub index: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetGroupRelayConfirmationTaskStateRequest {
    #[prost(uint32, tag="1")]
    pub index: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetGroupRelayConfirmationTaskStateReply {
    #[prost(enumeration="get_group_relay_confirmation_task_state_reply::GroupRelayConfirmationTaskState", tag="1")]
    pub state: i32,
}
/// Nested message and enum types in `GetGroupRelayConfirmationTaskStateReply`.
pub mod get_group_relay_confirmation_task_state_reply {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum GroupRelayConfirmationTaskState {
        NotExisted = 0,
        Available = 1,
        Invalid = 2,
    }
    impl GroupRelayConfirmationTaskState {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                GroupRelayConfirmationTaskState::NotExisted => "NotExisted",
                GroupRelayConfirmationTaskState::Available => "Available",
                GroupRelayConfirmationTaskState::Invalid => "Invalid",
            }
        }
    }
}
/// Generated client implementations.
pub mod transactions_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    #[derive(Debug, Clone)]
    pub struct TransactionsClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl TransactionsClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: std::convert::TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> TransactionsClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> TransactionsClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + Send + Sync,
        {
            TransactionsClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        pub async fn mine(
            &mut self,
            request: impl tonic::IntoRequest<super::MineRequest>,
        ) -> Result<tonic::Response<super::MineReply>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/adapter.Transactions/Mine",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn request_randomness(
            &mut self,
            request: impl tonic::IntoRequest<super::RequestRandomnessRequest>,
        ) -> Result<tonic::Response<()>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/adapter.Transactions/RequestRandomness",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn fulfill_randomness(
            &mut self,
            request: impl tonic::IntoRequest<super::FulfillRandomnessRequest>,
        ) -> Result<tonic::Response<()>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/adapter.Transactions/FulfillRandomness",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn set_initial_group(
            &mut self,
            request: impl tonic::IntoRequest<super::SetInitialGroupRequest>,
        ) -> Result<tonic::Response<()>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/adapter.Transactions/SetInitialGroup",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn fulfill_relay(
            &mut self,
            request: impl tonic::IntoRequest<super::FulfillRelayRequest>,
        ) -> Result<tonic::Response<()>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/adapter.Transactions/FulfillRelay",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn cancel_invalid_relay_confirmation_task(
            &mut self,
            request: impl tonic::IntoRequest<
                super::CancelInvalidRelayConfirmationTaskRequest,
            >,
        ) -> Result<tonic::Response<()>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/adapter.Transactions/CancelInvalidRelayConfirmationTask",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn confirm_relay(
            &mut self,
            request: impl tonic::IntoRequest<super::ConfirmRelayRequest>,
        ) -> Result<tonic::Response<()>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/adapter.Transactions/ConfirmRelay",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
    }
}
/// Generated client implementations.
pub mod views_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    #[derive(Debug, Clone)]
    pub struct ViewsClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl ViewsClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: std::convert::TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> ViewsClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> ViewsClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + Send + Sync,
        {
            ViewsClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        pub async fn get_group(
            &mut self,
            request: impl tonic::IntoRequest<super::GetGroupRequest>,
        ) -> Result<tonic::Response<super::GroupReply>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/adapter.Views/GetGroup");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn get_last_output(
            &mut self,
            request: impl tonic::IntoRequest<()>,
        ) -> Result<tonic::Response<super::LastOutputReply>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/adapter.Views/GetLastOutput",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn emit_signature_task(
            &mut self,
            request: impl tonic::IntoRequest<()>,
        ) -> Result<tonic::Response<super::SignatureTaskReply>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/adapter.Views/EmitSignatureTask",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn get_signature_task_completion_state(
            &mut self,
            request: impl tonic::IntoRequest<
                super::GetSignatureTaskCompletionStateRequest,
            >,
        ) -> Result<
            tonic::Response<super::GetSignatureTaskCompletionStateReply>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/adapter.Views/GetSignatureTaskCompletionState",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn get_group_relay_cache(
            &mut self,
            request: impl tonic::IntoRequest<super::GetGroupRelayCacheRequest>,
        ) -> Result<tonic::Response<super::GroupReply>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/adapter.Views/GetGroupRelayCache",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn get_group_relay_confirmation_task_state(
            &mut self,
            request: impl tonic::IntoRequest<
                super::GetGroupRelayConfirmationTaskStateRequest,
            >,
        ) -> Result<
            tonic::Response<super::GetGroupRelayConfirmationTaskStateReply>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/adapter.Views/GetGroupRelayConfirmationTaskState",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn emit_group_relay_confirmation_task(
            &mut self,
            request: impl tonic::IntoRequest<()>,
        ) -> Result<
            tonic::Response<super::GroupRelayConfirmationTaskReply>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/adapter.Views/EmitGroupRelayConfirmationTask",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
    }
}
/// Generated server implementations.
pub mod transactions_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    ///Generated trait containing gRPC methods that should be implemented for use with TransactionsServer.
    #[async_trait]
    pub trait Transactions: Send + Sync + 'static {
        async fn mine(
            &self,
            request: tonic::Request<super::MineRequest>,
        ) -> Result<tonic::Response<super::MineReply>, tonic::Status>;
        async fn request_randomness(
            &self,
            request: tonic::Request<super::RequestRandomnessRequest>,
        ) -> Result<tonic::Response<()>, tonic::Status>;
        async fn fulfill_randomness(
            &self,
            request: tonic::Request<super::FulfillRandomnessRequest>,
        ) -> Result<tonic::Response<()>, tonic::Status>;
        async fn set_initial_group(
            &self,
            request: tonic::Request<super::SetInitialGroupRequest>,
        ) -> Result<tonic::Response<()>, tonic::Status>;
        async fn fulfill_relay(
            &self,
            request: tonic::Request<super::FulfillRelayRequest>,
        ) -> Result<tonic::Response<()>, tonic::Status>;
        async fn cancel_invalid_relay_confirmation_task(
            &self,
            request: tonic::Request<super::CancelInvalidRelayConfirmationTaskRequest>,
        ) -> Result<tonic::Response<()>, tonic::Status>;
        async fn confirm_relay(
            &self,
            request: tonic::Request<super::ConfirmRelayRequest>,
        ) -> Result<tonic::Response<()>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct TransactionsServer<T: Transactions> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: Transactions> TransactionsServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for TransactionsServer<T>
    where
        T: Transactions,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/adapter.Transactions/Mine" => {
                    #[allow(non_camel_case_types)]
                    struct MineSvc<T: Transactions>(pub Arc<T>);
                    impl<T: Transactions> tonic::server::UnaryService<super::MineRequest>
                    for MineSvc<T> {
                        type Response = super::MineReply;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::MineRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).mine(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = MineSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/adapter.Transactions/RequestRandomness" => {
                    #[allow(non_camel_case_types)]
                    struct RequestRandomnessSvc<T: Transactions>(pub Arc<T>);
                    impl<
                        T: Transactions,
                    > tonic::server::UnaryService<super::RequestRandomnessRequest>
                    for RequestRandomnessSvc<T> {
                        type Response = ();
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::RequestRandomnessRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).request_randomness(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = RequestRandomnessSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/adapter.Transactions/FulfillRandomness" => {
                    #[allow(non_camel_case_types)]
                    struct FulfillRandomnessSvc<T: Transactions>(pub Arc<T>);
                    impl<
                        T: Transactions,
                    > tonic::server::UnaryService<super::FulfillRandomnessRequest>
                    for FulfillRandomnessSvc<T> {
                        type Response = ();
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::FulfillRandomnessRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).fulfill_randomness(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = FulfillRandomnessSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/adapter.Transactions/SetInitialGroup" => {
                    #[allow(non_camel_case_types)]
                    struct SetInitialGroupSvc<T: Transactions>(pub Arc<T>);
                    impl<
                        T: Transactions,
                    > tonic::server::UnaryService<super::SetInitialGroupRequest>
                    for SetInitialGroupSvc<T> {
                        type Response = ();
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::SetInitialGroupRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).set_initial_group(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = SetInitialGroupSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/adapter.Transactions/FulfillRelay" => {
                    #[allow(non_camel_case_types)]
                    struct FulfillRelaySvc<T: Transactions>(pub Arc<T>);
                    impl<
                        T: Transactions,
                    > tonic::server::UnaryService<super::FulfillRelayRequest>
                    for FulfillRelaySvc<T> {
                        type Response = ();
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::FulfillRelayRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).fulfill_relay(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = FulfillRelaySvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/adapter.Transactions/CancelInvalidRelayConfirmationTask" => {
                    #[allow(non_camel_case_types)]
                    struct CancelInvalidRelayConfirmationTaskSvc<T: Transactions>(
                        pub Arc<T>,
                    );
                    impl<
                        T: Transactions,
                    > tonic::server::UnaryService<
                        super::CancelInvalidRelayConfirmationTaskRequest,
                    > for CancelInvalidRelayConfirmationTaskSvc<T> {
                        type Response = ();
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::CancelInvalidRelayConfirmationTaskRequest,
                            >,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner)
                                    .cancel_invalid_relay_confirmation_task(request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = CancelInvalidRelayConfirmationTaskSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/adapter.Transactions/ConfirmRelay" => {
                    #[allow(non_camel_case_types)]
                    struct ConfirmRelaySvc<T: Transactions>(pub Arc<T>);
                    impl<
                        T: Transactions,
                    > tonic::server::UnaryService<super::ConfirmRelayRequest>
                    for ConfirmRelaySvc<T> {
                        type Response = ();
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ConfirmRelayRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).confirm_relay(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ConfirmRelaySvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        Ok(
                            http::Response::builder()
                                .status(200)
                                .header("grpc-status", "12")
                                .header("content-type", "application/grpc")
                                .body(empty_body())
                                .unwrap(),
                        )
                    })
                }
            }
        }
    }
    impl<T: Transactions> Clone for TransactionsServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
            }
        }
    }
    impl<T: Transactions> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: Transactions> tonic::server::NamedService for TransactionsServer<T> {
        const NAME: &'static str = "adapter.Transactions";
    }
}
/// Generated server implementations.
pub mod views_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    ///Generated trait containing gRPC methods that should be implemented for use with ViewsServer.
    #[async_trait]
    pub trait Views: Send + Sync + 'static {
        async fn get_group(
            &self,
            request: tonic::Request<super::GetGroupRequest>,
        ) -> Result<tonic::Response<super::GroupReply>, tonic::Status>;
        async fn get_last_output(
            &self,
            request: tonic::Request<()>,
        ) -> Result<tonic::Response<super::LastOutputReply>, tonic::Status>;
        async fn emit_signature_task(
            &self,
            request: tonic::Request<()>,
        ) -> Result<tonic::Response<super::SignatureTaskReply>, tonic::Status>;
        async fn get_signature_task_completion_state(
            &self,
            request: tonic::Request<super::GetSignatureTaskCompletionStateRequest>,
        ) -> Result<
            tonic::Response<super::GetSignatureTaskCompletionStateReply>,
            tonic::Status,
        >;
        async fn get_group_relay_cache(
            &self,
            request: tonic::Request<super::GetGroupRelayCacheRequest>,
        ) -> Result<tonic::Response<super::GroupReply>, tonic::Status>;
        async fn get_group_relay_confirmation_task_state(
            &self,
            request: tonic::Request<super::GetGroupRelayConfirmationTaskStateRequest>,
        ) -> Result<
            tonic::Response<super::GetGroupRelayConfirmationTaskStateReply>,
            tonic::Status,
        >;
        async fn emit_group_relay_confirmation_task(
            &self,
            request: tonic::Request<()>,
        ) -> Result<
            tonic::Response<super::GroupRelayConfirmationTaskReply>,
            tonic::Status,
        >;
    }
    #[derive(Debug)]
    pub struct ViewsServer<T: Views> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: Views> ViewsServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for ViewsServer<T>
    where
        T: Views,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/adapter.Views/GetGroup" => {
                    #[allow(non_camel_case_types)]
                    struct GetGroupSvc<T: Views>(pub Arc<T>);
                    impl<T: Views> tonic::server::UnaryService<super::GetGroupRequest>
                    for GetGroupSvc<T> {
                        type Response = super::GroupReply;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetGroupRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).get_group(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetGroupSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/adapter.Views/GetLastOutput" => {
                    #[allow(non_camel_case_types)]
                    struct GetLastOutputSvc<T: Views>(pub Arc<T>);
                    impl<T: Views> tonic::server::UnaryService<()>
                    for GetLastOutputSvc<T> {
                        type Response = super::LastOutputReply;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(&mut self, request: tonic::Request<()>) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).get_last_output(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetLastOutputSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/adapter.Views/EmitSignatureTask" => {
                    #[allow(non_camel_case_types)]
                    struct EmitSignatureTaskSvc<T: Views>(pub Arc<T>);
                    impl<T: Views> tonic::server::UnaryService<()>
                    for EmitSignatureTaskSvc<T> {
                        type Response = super::SignatureTaskReply;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(&mut self, request: tonic::Request<()>) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).emit_signature_task(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = EmitSignatureTaskSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/adapter.Views/GetSignatureTaskCompletionState" => {
                    #[allow(non_camel_case_types)]
                    struct GetSignatureTaskCompletionStateSvc<T: Views>(pub Arc<T>);
                    impl<
                        T: Views,
                    > tonic::server::UnaryService<
                        super::GetSignatureTaskCompletionStateRequest,
                    > for GetSignatureTaskCompletionStateSvc<T> {
                        type Response = super::GetSignatureTaskCompletionStateReply;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::GetSignatureTaskCompletionStateRequest,
                            >,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).get_signature_task_completion_state(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetSignatureTaskCompletionStateSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/adapter.Views/GetGroupRelayCache" => {
                    #[allow(non_camel_case_types)]
                    struct GetGroupRelayCacheSvc<T: Views>(pub Arc<T>);
                    impl<
                        T: Views,
                    > tonic::server::UnaryService<super::GetGroupRelayCacheRequest>
                    for GetGroupRelayCacheSvc<T> {
                        type Response = super::GroupReply;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetGroupRelayCacheRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).get_group_relay_cache(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetGroupRelayCacheSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/adapter.Views/GetGroupRelayConfirmationTaskState" => {
                    #[allow(non_camel_case_types)]
                    struct GetGroupRelayConfirmationTaskStateSvc<T: Views>(pub Arc<T>);
                    impl<
                        T: Views,
                    > tonic::server::UnaryService<
                        super::GetGroupRelayConfirmationTaskStateRequest,
                    > for GetGroupRelayConfirmationTaskStateSvc<T> {
                        type Response = super::GetGroupRelayConfirmationTaskStateReply;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::GetGroupRelayConfirmationTaskStateRequest,
                            >,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner)
                                    .get_group_relay_confirmation_task_state(request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetGroupRelayConfirmationTaskStateSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/adapter.Views/EmitGroupRelayConfirmationTask" => {
                    #[allow(non_camel_case_types)]
                    struct EmitGroupRelayConfirmationTaskSvc<T: Views>(pub Arc<T>);
                    impl<T: Views> tonic::server::UnaryService<()>
                    for EmitGroupRelayConfirmationTaskSvc<T> {
                        type Response = super::GroupRelayConfirmationTaskReply;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(&mut self, request: tonic::Request<()>) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).emit_group_relay_confirmation_task(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = EmitGroupRelayConfirmationTaskSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        Ok(
                            http::Response::builder()
                                .status(200)
                                .header("grpc-status", "12")
                                .header("content-type", "application/grpc")
                                .body(empty_body())
                                .unwrap(),
                        )
                    })
                }
            }
        }
    }
    impl<T: Views> Clone for ViewsServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
            }
        }
    }
    impl<T: Views> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: Views> tonic::server::NamedService for ViewsServer<T> {
        const NAME: &'static str = "adapter.Views";
    }
}
