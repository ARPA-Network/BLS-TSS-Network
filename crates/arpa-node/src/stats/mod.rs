use crate::context::chain::Chain;
use crate::context::{types::GeneralContext, Context};
use crate::context::ContextFetcher;
use crate::scheduler::FixedTaskScheduler;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use arpa_core::{ListenerType, RpcServerType, TaskType};
use std::sync::Arc;
use threshold_bls::{
    group::Curve,
    sig::{SignatureScheme, ThresholdScheme},
};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

type NodeContext<PC, S> = Arc<RwLock<GeneralContext<PC, S>>>;

#[derive(Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_name: String,
    pub spec_version: String,
    pub node_version: String,
}

#[derive(Serialize, Deserialize)]
pub struct ServiceInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: String,
}

#[get("/hello/{name}")]
async fn greet(name: web::Path<String>) -> impl Responder {
    format!("Hello {name}!")
}

async fn health<
    PC: Curve + std::fmt::Debug + Clone + Sync + Send + 'static,
    SS: SignatureScheme
        + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
        + Clone
        + Send
        + Sync
        + 'static,
>(
    context: web::Data<NodeContext<PC, SS>>,
) -> impl Responder {
    let task_count = context
        .into_inner()
        .read()
        .await
        .get_fixed_task_handler()
        .read()
        .await
        .get_tasks()
        .len();

    // this is an imperfect health check to demonstrate how to use the context
    if task_count > 0 {
        HttpResponse::Ok()
    } else {
        HttpResponse::ServiceUnavailable()
    }
}

async fn node_info<
    PC: Curve + std::fmt::Debug + Clone + Sync + Send + 'static,
    SS: SignatureScheme
        + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
        + Clone
        + Send
        + Sync
        + 'static,
>(
    context: web::Data<NodeContext<PC, SS>>,
) -> impl Responder
where
    <SS as ThresholdScheme>::Error: Sync + Send,
    <SS as SignatureScheme>::Error: Sync + Send,
{
    // Try to retrieve group and node index, and if there is any error, throw it in internal server error. 
    let group_cache = context
        .into_inner()
        .read()
        .await
        .get_main_chain()
        .get_group_cache();
    let (group_index, node_index) = match (group_cache.read().await.get_index(), group_cache.read().await.get_self_index()) {
        (Ok(group_idx), Ok(node_idx)) => (Some(group_idx), Some(node_idx)),
        (Err(err), _) | (_, Err(err)) => {
            return HttpResponse::InternalServerError().json(format!("Not able to retrieve node info: {}", err));
        }
    };

    // Format output accordingly.
    let node_name = match (group_index, node_index) {
        (Some(group_idx), Some(node_idx)) => {
            format!("Arpa-Randcast-group{}-node{}", group_idx, node_idx)
        }
        _ => "Arpa-Randcast-node".to_string(),
    };

    let node_info = NodeInfo {
        node_name,
        spec_version: "v0.0.1".to_string(),
        node_version: "v1.0.0".to_string(),
    };

    HttpResponse::Ok().json(node_info)
}


async fn is_node_info_filled<
    PC: Curve + std::fmt::Debug + Clone + Sync + Send + 'static,
    SS: SignatureScheme
        + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
        + Clone
        + Send
        + Sync
        + 'static,
    >(
        context: web::Data<NodeContext<PC, SS>>,
    ) -> bool 
    where
    <SS as ThresholdScheme>::Error: Sync + Send,
    <SS as SignatureScheme>::Error: Sync + Send,
{
    let node_cache_handler = context
    .into_inner()
    .read()
    .await
    .get_main_chain()
    .get_node_cache();
    let node_cache = node_cache_handler.read().await;

    matches!((
        node_cache.get_id_address(),
        node_cache.get_node_rpc_endpoint(),
        node_cache.get_dkg_private_key(),
        node_cache.get_dkg_public_key(),
        ), (Ok(_), Ok(_), Ok(_), Ok(_)))
}

async fn is_node_connected<
    PC: Curve + std::fmt::Debug + Clone + Sync + Send + 'static,
    SS: SignatureScheme
        + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
        + Clone
        + Send
        + Sync
        + 'static,
>(
    context: web::Data<NodeContext<PC, SS>>,
) -> bool {

    let tasks: Vec<TaskType> =
        context
        .into_inner()
        .read()
        .await
        .get_fixed_task_handler()
        .read()
        .await
        .get_tasks()
        .into_iter()
        .cloned()
        .collect();
   
    let (has_committer, has_management) = tasks.iter().fold((false,false), |(has_committer, has_management), task| {
        match task {
            TaskType::RpcServer(rpc_server_type) => {
                (
                    has_committer || matches!(rpc_server_type, RpcServerType::Committer),
                    has_management || matches!(rpc_server_type, RpcServerType::Management),
                )
            }
            _ => (has_committer, has_management),
        }
    });

    has_committer && has_management
}

async fn is_node_registered<
    PC: Curve + std::fmt::Debug + Clone + Sync + Send + 'static,
    SS: SignatureScheme
        + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
        + Clone
        + Send
        + Sync
        + 'static,
    >(
        context: web::Data<NodeContext<PC, SS>>,
    ) -> bool 
    where
    <SS as ThresholdScheme>::Error: Sync + Send,
    <SS as SignatureScheme>::Error: Sync + Send,
{
    let dkg_start_block_height = context
        .into_inner()
        .read()
        .await
        .get_main_chain()
        .get_group_cache()
        .read()
        .await
        .get_dkg_start_block_height()
        .unwrap_or(0);
    dkg_start_block_height > 0
}

async fn node_health<
    PC: Curve + std::fmt::Debug + Clone + Sync + Send + 'static,
    SS: SignatureScheme
        + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
        + Clone
        + Send
        + Sync
        + 'static,
>(
    context: web::Data<NodeContext<PC, SS>>,
) -> impl Responder
where
    <SS as ThresholdScheme>::Error: Sync + Send,
    <SS as SignatureScheme>::Error: Sync + Send,
{   
    if !is_node_info_filled(context.clone()).await {
        return HttpResponse::ServiceUnavailable().json("AVS Node info is not filled completely.");
    }
    if !is_node_connected(context.clone()).await {
        return HttpResponse::ServiceUnavailable().json("AVS Node is not connected since RPC servers not started.");
    }
    if !is_node_registered(context.clone()).await {
        return HttpResponse::PartialContent().json("AVS Node is healthy, but not registered.");
    }
    HttpResponse::Ok().json("Node is fully healthy")
}

async fn services_info_value<
    PC: Curve + std::fmt::Debug + Clone + Sync + Send + 'static,
    SS: SignatureScheme
        + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
        + Clone
        + Send
        + Sync
        + 'static,
>(
    context: web::Data<NodeContext<PC, SS>>,
) -> Vec<ServiceInfo>
where
    <SS as ThresholdScheme>::Error: Sync + Send,
    <SS as SignatureScheme>::Error: Sync + Send,
{
    let tasks: Vec<TaskType> =
        context
        .into_inner()
        .read()
        .await
        .get_fixed_task_handler()
        .read()
        .await
        .get_tasks()
        .into_iter()
        .cloned()
        .collect();

    let mut services_info = vec![
        ServiceInfo {
            id: "block".to_string(),
            name: "Block".to_string(),
            description: "Block listener".to_string(),
            status: "down".to_string(),
        },
        ServiceInfo {
            id: "pre_grouping".to_string(),
            name: "PreGrouping".to_string(),
            description: "PreGrouping listener".to_string(),
            status: "down".to_string(),
        },
        ServiceInfo {
            id: "post_commit_grouping".to_string(),
            name: "PostCommitGrouping".to_string(),
            description: "PostCommitGrouping listener".to_string(),
            status: "down".to_string(),
        },
        ServiceInfo {
            id: "post_grouping".to_string(),
            name: "PostGrouping".to_string(),
            description: "PostGrouping listener".to_string(),
            status: "down".to_string(),
        },
        ServiceInfo {
            id: "new_randomness_task".to_string(),
            name: "NewRandomnessTask".to_string(),
            description: "NewRandomnessTask listener".to_string(),
            status: "down".to_string(),
        },
        ServiceInfo {
            id: "ready_to_handle_randomness_task".to_string(),
            name: "ReadyToHandleRandomnessTask".to_string(),
            description: "ReadyToHandleRandomnessTask listener".to_string(),
            status: "down".to_string(),
        },
        ServiceInfo {
            id: "randomness_signature_aggregation".to_string(),
            name: "RandomnessSignatureAggregation".to_string(),
            description: "RandomnessSignatureAggregation listener".to_string(),
            status: "down".to_string(),
        },
    ];

    for task_type in tasks {
        if let TaskType::Listener(_, listener_type) = task_type {
            let service_id = match listener_type {
                ListenerType::Block => "block",
                ListenerType::PreGrouping => "pre_grouping",
                ListenerType::PostCommitGrouping => "post_commit_grouping",
                ListenerType::PostGrouping => "post_grouping",
                ListenerType::NewRandomnessTask => "new_randomness_task",
                ListenerType::ReadyToHandleRandomnessTask => "ready_to_handle_randomness_task",
                ListenerType::RandomnessSignatureAggregation => "randomness_signature_aggregation",
            };

            if let Some(service) = services_info.iter_mut().find(|s| s.id == service_id) {
                service.status ="up".to_string();
            }
        }
    }

    services_info
}

async fn services_info<
    PC: Curve + std::fmt::Debug + Clone + Sync + Send + 'static,
    SS: SignatureScheme
        + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
        + Clone
        + Send
        + Sync
        + 'static,
>(
    context: web::Data<NodeContext<PC, SS>>,
) -> impl Responder
where
    <SS as ThresholdScheme>::Error: Sync + Send,
    <SS as SignatureScheme>::Error: Sync + Send,
{
    HttpResponse::Ok().json(services_info_value(context).await)
}

async fn service_health<
    PC: Curve + std::fmt::Debug + Clone + Sync + Send + 'static,
    SS: SignatureScheme
        + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
        + Clone
        + Send
        + Sync
        + 'static,
>(
    context: web::Data<NodeContext<PC, SS>>,
    service_id: web::Path<String>,
) -> impl Responder 
where
    <SS as ThresholdScheme>::Error: Sync + Send,
    <SS as SignatureScheme>::Error: Sync + Send,
{
    let services_info = services_info_value(context).await;
    let service_id = service_id.into_inner();

    if let Some(service) = services_info.iter().find(|s| s.id == service_id) {
        if service.status == "up" {
            HttpResponse::Ok().finish()
        } else {
            HttpResponse::ServiceUnavailable().finish()
        }
    } else {
        HttpResponse::NotFound().finish()
    }
}

pub async fn start_statistics_server<
    PC: Curve + std::fmt::Debug + Clone + Sync + Send + 'static,
    SS: SignatureScheme
        + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
        + Clone
        + Send
        + Sync
        + 'static,
>(
    endpoint: String,
    context: NodeContext<PC, SS>,
) -> Result<(), Box<dyn std::error::Error + Send>>
where
    <SS as ThresholdScheme>::Error: Sync + Send,
    <SS as SignatureScheme>::Error: Sync + Send,
{
    if let Err(err) = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(context.clone()))
            .route("/health", web::get().to(health::<PC, SS>))
            .route("/eigen/node", web::get().to(node_info::<PC, SS>))
            .route("/eigen/node/health", web::get().to(node_health::<PC, SS>))
            .route("/eigen/node/services", web::get().to(services_info::<PC, SS>))
            .route("/eigen/node/services/{service_id}/health", web::get().to(service_health::<PC, SS>))
            .service(greet)
    })
    .bind(endpoint)
    .unwrap()
    .run()
    .await
    {
        return Err(Box::new(err));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::chain::types::GeneralMainChain;
    use crate::scheduler::TaskScheduler;
    use actix_web::{
        http::{self},
        test,
    };
    use arpa_core::{Config, DKGStatus, GeneralMainChainIdentity, Group, RandomnessTask};
    use arpa_dal::{
        cache::{
            InMemoryBLSTasksQueue, InMemoryGroupInfoCache, InMemoryNodeInfoCache,
            InMemorySignatureResultCache, RandomnessResultCache,
        },
        BLSTasksHandler, GroupInfoHandler, NodeInfoHandler, SignatureResultCacheHandler,
    };
    use ethers::{
        providers::{Provider, Ws},
        types::Address,
        utils::Anvil,
    };
    
    use threshold_bls::{curve::bn254::G2Curve, schemes::bn254::G2Scheme, sig::Scheme};

    async fn build_context() -> NodeContext<G2Curve, G2Scheme> {
        let config = Config::default();

        let fake_wallet = "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318"
            .parse()
            .unwrap();

        let id_address = Address::random();
        let rng = &mut rand::thread_rng();

        let (dkg_private_key, dkg_public_key) = G2Scheme::keypair(rng);
        let node_rpc_endpoint = "dummy_test_string".to_string();
    
        let node_cache: Arc<RwLock<Box<dyn NodeInfoHandler<G2Curve>>>> = Arc::new(RwLock::new(
            Box::new(InMemoryNodeInfoCache::<G2Curve>::rebuild(
                id_address,
                node_rpc_endpoint,
                dkg_private_key,
                dkg_public_key,
            )),
        ));

        let group = Group::<G2Curve>::new();
        let dkg_status = DKGStatus::None;
        let self_index = 0;
        let dkg_start_block_height = 1;

        let group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>> = Arc::new(RwLock::new(
            Box::new(InMemoryGroupInfoCache::<G2Curve>::rebuild(
                None,
                group,
                dkg_status,
                self_index,
                dkg_start_block_height,
            )),
        ));

        let randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>> =
            Arc::new(RwLock::new(Box::new(InMemoryBLSTasksQueue::new())));

        let randomness_result_cache: Arc<
            RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>,
        > = Arc::new(RwLock::new(Box::new(InMemorySignatureResultCache::<
            RandomnessResultCache,
        >::new())));

        let avnil = Anvil::new().spawn();

        let provider = Arc::new(Provider::<Ws>::connect(avnil.ws_endpoint()).await.unwrap());

        let contract_transaction_retry_descriptor = config
            .get_time_limits()
            .contract_transaction_retry_descriptor;

        let contract_view_retry_descriptor =
            config.get_time_limits().contract_view_retry_descriptor;

        let main_chain_identity = GeneralMainChainIdentity::new(
            config.get_main_chain_id(),
            fake_wallet,
            provider,
            avnil.ws_endpoint(),
            Address::random(),
            Address::random(),
            Address::random(),
            contract_transaction_retry_descriptor,
            contract_view_retry_descriptor,
        );

        let main_chain = GeneralMainChain::<G2Curve, G2Scheme>::new(
            "main chain".to_string(),
            main_chain_identity.clone(),
            node_cache.clone(),
            group_cache.clone(),
            randomness_tasks_cache,
            randomness_result_cache,
            *config.get_time_limits(),
            config.get_listeners().clone(),
        );

        let context = GeneralContext::new(main_chain, config);

        context
            .get_fixed_task_handler()
            .write()
            .await
            .add_task(TaskType::Listener(0, ListenerType::Block), async {})
            .unwrap();

        context
            .get_fixed_task_handler()
            .write()
            .await
            .add_task(TaskType::RpcServer(RpcServerType::Committer), async {})
            .unwrap();
        context
            .get_fixed_task_handler()
            .write()
            .await
            .add_task(TaskType::RpcServer(RpcServerType::Management), async {})
            .unwrap();

        Arc::new(RwLock::new(context))
    }

    #[actix_web::test]
    async fn test_health_get() {
        let context = build_context().await;
        let app = App::new()
            .app_data(web::Data::new(context))
            .route("/health", web::get().to(health::<G2Curve, G2Scheme>));
        let app = test::init_service(app).await;

        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
    }

    #[actix_web::test]
    async fn test_health_post() {
        let context = build_context().await;
        let app = App::new()
            .app_data(web::Data::new(context))
            .route("/health", web::get().to(health::<G2Curve, G2Scheme>));
        let app = test::init_service(app).await;

        let req = test::TestRequest::post().uri("/").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_client_error());
    }

    #[actix_web::test]
    async fn test_greet() {
        let app = App::new().service(greet);
        let app = test::init_service(app).await;

        let req = test::TestRequest::get().uri("/hello/world").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        let body = test::read_body(resp).await;
        assert_eq!(body, "Hello world!");
    }

    #[actix_web::test]
    async fn test_node_info() {
        let context = build_context().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(context))
                .route("/eigen/node", web::get().to(node_info::<G2Curve, G2Scheme>)),
        )
        .await;

        let req = test::TestRequest::get().uri("/eigen/node").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);

        let body: NodeInfo = test::read_body_json(resp).await;
        assert_eq!(body.spec_version, "v0.0.1");
        assert_eq!(body.node_version, "v1.0.0");
    }

    #[actix_web::test]
    async fn test_node_health() {
        let context = build_context().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(context))
                .route("/eigen/node/health", web::get().to(node_health::<G2Curve, G2Scheme>)),
        )
        .await;

        let req = test::TestRequest::get().uri("/eigen/node/health").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);

        let body: String = test::read_body_json(resp).await;
        assert_eq!(body, "Node is fully healthy");
    }

    #[actix_web::test]
    async fn test_services_info() {
        let context = build_context().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(context))
                .route("/eigen/services", web::get().to(services_info::<G2Curve, G2Scheme>)),
        )
        .await;

        let req = test::TestRequest::get().uri("/eigen/services").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);

        let body: Vec<ServiceInfo> = test::read_body_json(resp).await;
        assert_eq!(body.len(), 7);
        assert!(body.iter().any(|s| s.id == "block" && s.status == "up"));
    }

    #[actix_web::test]
    async fn test_service_health() {
        let context = build_context().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(context))
                .route(
                    "/eigen/node/services/{service_id}/health",
                    web::get().to(service_health::<G2Curve, G2Scheme>),
                ),
        )
        .await;

        // Test a healthy service
        let req = test::TestRequest::get()
            .uri("/eigen/node/services/block/health")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);

        // Test an unhealthy service
        let req = test::TestRequest::get()
            .uri("/eigen/node/services/pre_grouping/health")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::SERVICE_UNAVAILABLE);

        // Test a non-existent service
        let req = test::TestRequest::get()
            .uri("/eigen/node/services/unknown/health")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_is_node_info_filled() {
        let context = build_context().await;
        assert!(is_node_info_filled(web::Data::new(context)).await);
    }

    #[actix_web::test]
    async fn test_is_node_connected() {
        let context = build_context().await;
        assert!(is_node_connected(web::Data::new(context)).await);
    }

    #[actix_web::test]
    async fn test_is_node_registered() {
        let context = build_context().await;
        assert!(is_node_registered(web::Data::new(context)).await);
    }

    #[actix_web::test]
    async fn test_services_info_value() {
        let context = build_context().await;
        let services_info = services_info_value(web::Data::new(context)).await;
        assert_eq!(services_info.len(), 7);
        assert!(services_info.iter().any(|s| s.id == "block" && s.status == "up"));
    }
}
