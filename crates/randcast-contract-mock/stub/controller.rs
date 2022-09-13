#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NodeRegisterRequest {
    #[prost(string, tag = "1")]
    pub id_address: ::prost::alloc::string::String,
    #[prost(bytes = "vec", tag = "2")]
    pub id_public_key: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CommitDkgRequest {
    #[prost(string, tag = "1")]
    pub id_address: ::prost::alloc::string::String,
    #[prost(uint32, tag = "2")]
    pub group_index: u32,
    #[prost(uint32, tag = "3")]
    pub group_epoch: u32,
    #[prost(bytes = "vec", tag = "4")]
    pub public_key: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "5")]
    pub partial_public_key: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, repeated, tag = "6")]
    pub disqualified_nodes: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PostProcessDkgRequest {
    #[prost(string, tag = "1")]
    pub id_address: ::prost::alloc::string::String,
    #[prost(uint32, tag = "2")]
    pub group_index: u32,
    #[prost(uint32, tag = "3")]
    pub group_epoch: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MineRequest {
    #[prost(uint32, tag = "1")]
    pub block_number_increment: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MineReply {
    #[prost(uint32, tag = "1")]
    pub block_number: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetNodeRequest {
    #[prost(string, tag = "1")]
    pub id_address: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NodeReply {
    #[prost(string, tag = "1")]
    pub id_address: ::prost::alloc::string::String,
    #[prost(bytes = "vec", tag = "2")]
    pub id_public_key: ::prost::alloc::vec::Vec<u8>,
    #[prost(bool, tag = "3")]
    pub state: bool,
    #[prost(uint32, tag = "4")]
    pub pending_until_block: u32,
    #[prost(uint32, tag = "5")]
    pub staking: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Member {
    #[prost(uint32, tag = "1")]
    pub index: u32,
    #[prost(string, tag = "2")]
    pub id_address: ::prost::alloc::string::String,
    #[prost(bytes = "vec", tag = "3")]
    pub partial_public_key: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DkgTaskReply {
    #[prost(uint32, tag = "1")]
    pub group_index: u32,
    #[prost(uint32, tag = "2")]
    pub epoch: u32,
    #[prost(uint32, tag = "3")]
    pub size: u32,
    #[prost(uint32, tag = "4")]
    pub threshold: u32,
    #[prost(btree_map = "string, uint32", tag = "5")]
    pub members: ::prost::alloc::collections::BTreeMap<::prost::alloc::string::String, u32>,
    #[prost(uint32, tag = "6")]
    pub assignment_block_height: u32,
    #[prost(string, tag = "7")]
    pub coordinator_address: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GroupRelayTaskReply {
    #[prost(uint32, tag = "1")]
    pub controller_global_epoch: u32,
    #[prost(uint32, tag = "2")]
    pub relayed_group_index: u32,
    #[prost(uint32, tag = "3")]
    pub relayed_group_epoch: u32,
    #[prost(uint32, tag = "4")]
    pub assignment_block_height: u32,
}
#[doc = r" Generated server implementations."]
pub mod transactions_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    #[doc = "Generated trait containing gRPC methods that should be implemented for use with TransactionsServer."]
    #[async_trait]
    pub trait Transactions: Send + Sync + 'static {
        async fn node_register(
            &self,
            request: tonic::Request<super::NodeRegisterRequest>,
        ) -> Result<tonic::Response<()>, tonic::Status>;
        async fn commit_dkg(
            &self,
            request: tonic::Request<super::CommitDkgRequest>,
        ) -> Result<tonic::Response<()>, tonic::Status>;
        async fn post_process_dkg(
            &self,
            request: tonic::Request<super::PostProcessDkgRequest>,
        ) -> Result<tonic::Response<()>, tonic::Status>;
        async fn mine(
            &self,
            request: tonic::Request<super::MineRequest>,
        ) -> Result<tonic::Response<super::MineReply>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct TransactionsServer<T: Transactions> {
        inner: _Inner<T>,
        accept_compression_encodings: (),
        send_compression_encodings: (),
    }
    struct _Inner<T>(Arc<T>);
    impl<T: Transactions> TransactionsServer<T> {
        pub fn new(inner: T) -> Self {
            let inner = Arc::new(inner);
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
            }
        }
        pub fn with_interceptor<F>(inner: T, interceptor: F) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for TransactionsServer<T>
    where
        T: Transactions,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = Never;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/controller.Transactions/NodeRegister" => {
                    #[allow(non_camel_case_types)]
                    struct NodeRegisterSvc<T: Transactions>(pub Arc<T>);
                    impl<T: Transactions> tonic::server::UnaryService<super::NodeRegisterRequest>
                        for NodeRegisterSvc<T>
                    {
                        type Response = ();
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::NodeRegisterRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).node_register(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = NodeRegisterSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/controller.Transactions/CommitDkg" => {
                    #[allow(non_camel_case_types)]
                    struct CommitDkgSvc<T: Transactions>(pub Arc<T>);
                    impl<T: Transactions> tonic::server::UnaryService<super::CommitDkgRequest> for CommitDkgSvc<T> {
                        type Response = ();
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::CommitDkgRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).commit_dkg(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = CommitDkgSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/controller.Transactions/PostProcessDkg" => {
                    #[allow(non_camel_case_types)]
                    struct PostProcessDkgSvc<T: Transactions>(pub Arc<T>);
                    impl<T: Transactions> tonic::server::UnaryService<super::PostProcessDkgRequest>
                        for PostProcessDkgSvc<T>
                    {
                        type Response = ();
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::PostProcessDkgRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).post_process_dkg(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = PostProcessDkgSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/controller.Transactions/Mine" => {
                    #[allow(non_camel_case_types)]
                    struct MineSvc<T: Transactions>(pub Arc<T>);
                    impl<T: Transactions> tonic::server::UnaryService<super::MineRequest> for MineSvc<T> {
                        type Response = super::MineReply;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
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
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => Box::pin(async move {
                    Ok(http::Response::builder()
                        .status(200)
                        .header("grpc-status", "12")
                        .header("content-type", "application/grpc")
                        .body(empty_body())
                        .unwrap())
                }),
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
    impl<T: Transactions> tonic::transport::NamedService for TransactionsServer<T> {
        const NAME: &'static str = "controller.Transactions";
    }
}
#[doc = r" Generated server implementations."]
pub mod views_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    #[doc = "Generated trait containing gRPC methods that should be implemented for use with ViewsServer."]
    #[async_trait]
    pub trait Views: Send + Sync + 'static {
        async fn get_node(
            &self,
            request: tonic::Request<super::GetNodeRequest>,
        ) -> Result<tonic::Response<super::NodeReply>, tonic::Status>;
        async fn emit_dkg_task(
            &self,
            request: tonic::Request<()>,
        ) -> Result<tonic::Response<super::DkgTaskReply>, tonic::Status>;
        async fn emit_group_relay_task(
            &self,
            request: tonic::Request<()>,
        ) -> Result<tonic::Response<super::GroupRelayTaskReply>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct ViewsServer<T: Views> {
        inner: _Inner<T>,
        accept_compression_encodings: (),
        send_compression_encodings: (),
    }
    struct _Inner<T>(Arc<T>);
    impl<T: Views> ViewsServer<T> {
        pub fn new(inner: T) -> Self {
            let inner = Arc::new(inner);
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
            }
        }
        pub fn with_interceptor<F>(inner: T, interceptor: F) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for ViewsServer<T>
    where
        T: Views,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = Never;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/controller.Views/GetNode" => {
                    #[allow(non_camel_case_types)]
                    struct GetNodeSvc<T: Views>(pub Arc<T>);
                    impl<T: Views> tonic::server::UnaryService<super::GetNodeRequest> for GetNodeSvc<T> {
                        type Response = super::NodeReply;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetNodeRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).get_node(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetNodeSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/controller.Views/EmitDkgTask" => {
                    #[allow(non_camel_case_types)]
                    struct EmitDkgTaskSvc<T: Views>(pub Arc<T>);
                    impl<T: Views> tonic::server::UnaryService<()> for EmitDkgTaskSvc<T> {
                        type Response = super::DkgTaskReply;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(&mut self, request: tonic::Request<()>) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).emit_dkg_task(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = EmitDkgTaskSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/controller.Views/EmitGroupRelayTask" => {
                    #[allow(non_camel_case_types)]
                    struct EmitGroupRelayTaskSvc<T: Views>(pub Arc<T>);
                    impl<T: Views> tonic::server::UnaryService<()> for EmitGroupRelayTaskSvc<T> {
                        type Response = super::GroupRelayTaskReply;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(&mut self, request: tonic::Request<()>) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).emit_group_relay_task(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = EmitGroupRelayTaskSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => Box::pin(async move {
                    Ok(http::Response::builder()
                        .status(200)
                        .header("grpc-status", "12")
                        .header("content-type", "application/grpc")
                        .body(empty_body())
                        .unwrap())
                }),
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
    impl<T: Views> tonic::transport::NamedService for ViewsServer<T> {
        const NAME: &'static str = "controller.Views";
    }
}
