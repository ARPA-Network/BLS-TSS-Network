use crate::context::chain::Chain;
use crate::context::{types::GeneralContext, Context};
use crate::context::ContextFetcher;
use crate::scheduler::FixedTaskScheduler;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
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

pub async fn node_info<
    PC: Curve + std::fmt::Debug + Clone + Sync + Send + 'static,
    SS: SignatureScheme
        + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
        + Clone
        + Send
        + Sync
        + 'static,
>(
    context: NodeContext<PC, SS>,
) -> impl Responder
where
    <SS as ThresholdScheme>::Error: Sync + Send,
    <SS as SignatureScheme>::Error: Sync + Send,
{
    let group_cache = context
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

    return HttpResponse::Ok().json(node_info);
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
    use arpa_core::{Config, GeneralMainChainIdentity, ListenerType, RandomnessTask, TaskType};
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
    use threshold_bls::{curve::bn254::G2Curve, schemes::bn254::G2Scheme};

    async fn build_context() -> NodeContext<G2Curve, G2Scheme> {
        let config = Config::default();

        let fake_wallet = "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318"
            .parse()
            .unwrap();

        let node_cache: Arc<RwLock<Box<dyn NodeInfoHandler<G2Curve>>>> = Arc::new(RwLock::new(
            Box::new(InMemoryNodeInfoCache::<G2Curve>::new(Address::random())),
        ));

        let group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>> = Arc::new(RwLock::new(
            Box::new(InMemoryGroupInfoCache::<G2Curve>::default()),
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
}
