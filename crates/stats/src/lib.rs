#[macro_use]
extern crate rocket;
use rocket::get;
use rocket::http::Status;
use rocket::response::status::Custom;
use rocket::serde::json::Json;
use rocket::Config;
use serde::{Deserialize, Serialize};

const LATEST_VERSION: &str = "0.0.1";

#[derive(Serialize, Deserialize)]
struct NodeInfo {
    node_name: String,
    spec_version: String,
    node_version: String,
}

#[derive(Serialize, Deserialize)]
struct ErrorMessage {
    message: String,
}

#[derive(Serialize, Deserialize)]
struct Service {
    id: String,
    name: String,
    description: String,
    status: String,
}

#[derive(Serialize, Deserialize)]
struct ServiceList {
    services: Vec<Service>,
}

#[get("/eigen/node?<version>")]
fn get_node_info(version: Option<&str>) -> Result<Json<NodeInfo>, Custom<Json<ErrorMessage>>> {
    match version {
        Some(v) if v == LATEST_VERSION => Ok(Json(NodeInfo {
            node_name: "EigenLayer-AVS".to_string(),
            spec_version: LATEST_VERSION.to_string(),
            node_version: "v1.0.0".to_string(),
        })),
        Some(_) => Err(Custom(
            Status::NotFound,
            Json(ErrorMessage {
                message: "API version not found".to_string(),
            }),
        )),
        None => Ok(Json(NodeInfo {
            node_name: "EigenLayer-AVS".to_string(),
            spec_version: LATEST_VERSION.to_string(),
            node_version: "v1.0.0".to_string(),
        })),
    }
}

#[get("/eigen/node/health")]
fn get_node_health() -> Status {
    Status::Ok
}

#[get("/eigen/node/services")]
fn get_node_services() -> Result<Json<ServiceList>, Custom<Json<ErrorMessage>>> {
    let services = vec![
        Service {
            id: "db-1".to_string(),
            name: "Database".to_string(),
            description: "Database description".to_string(),
            status: "Up".to_string(),
        },
        Service {
            id: "idx-2".to_string(),
            name: "Indexer".to_string(),
            description: "Indexer description".to_string(),
            status: "Down".to_string(),
        },
    ];

    Ok(Json(ServiceList { services }))
}

#[get("/eigen/node/services/<service_id>/health")]
fn get_service_health(service_id: &str) -> Status {
    match service_id {
        "db-1" => Status::Ok,
        "idx-2" => Status::ServiceUnavailable,
        _ => Status::NotFound,
    }
}

pub fn rocket() -> rocket::Rocket<rocket::Build> {
    let config = Config::default();
    rocket::build().configure(config).mount(
        "/",
        routes![
            get_node_info,
            get_node_health,
            get_node_services,
            get_service_health
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::http::ContentType;
    use rocket::http::Status;
    use rocket::local::blocking::Client;

    #[test]
    fn test_get_node_info_latest_version() {
        let client = Client::tracked(rocket()).expect("valid rocket instance");
        let response = client.get("/eigen/node?version=0.0.1").dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.content_type(), Some(ContentType::JSON));
        let node_info: NodeInfo =
            serde_json::from_str(response.into_string().unwrap().as_str()).unwrap();
        assert_eq!(node_info.node_name, "EigenLayer-AVS");
        assert_eq!(node_info.spec_version, "0.0.1");
        assert_eq!(node_info.node_version, "v1.0.0");
    }

    #[test]
    fn test_get_node_info_no_version() {
        let client = Client::tracked(rocket()).expect("valid rocket instance");
        let response = client.get("/eigen/node").dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.content_type(), Some(ContentType::JSON));
        let node_info: NodeInfo =
            serde_json::from_str(response.into_string().unwrap().as_str()).unwrap();
        assert_eq!(node_info.node_name, "EigenLayer-AVS");
        assert_eq!(node_info.spec_version, "0.0.1");
        assert_eq!(node_info.node_version, "v1.0.0");
    }

    #[test]
    fn test_get_node_info_invalid_version() {
        let client = Client::tracked(rocket()).expect("valid rocket instance");
        let response = client.get("/eigen/node?version=0.0.2").dispatch();
        assert_eq!(response.status(), Status::NotFound);
        assert_eq!(response.content_type(), Some(ContentType::JSON));
        let error_message: ErrorMessage =
            serde_json::from_str(response.into_string().unwrap().as_str()).unwrap();
        assert_eq!(error_message.message, "API version not found");
    }

    #[test]
    fn test_get_node_health() {
        let client = Client::tracked(rocket()).expect("valid rocket instance");
        let response = client.get("/eigen/node/health").dispatch();
        assert_eq!(response.status(), Status::Ok);
    }

    #[test]
    fn test_get_node_services() {
        let client = Client::tracked(rocket()).expect("valid rocket instance");
        let response = client.get("/eigen/node/services").dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.content_type(), Some(ContentType::JSON));
        let service_list: ServiceList =
            serde_json::from_str(response.into_string().unwrap().as_str()).unwrap();
        assert_eq!(service_list.services.len(), 2);
        assert_eq!(service_list.services[0].id, "db-1");
        assert_eq!(service_list.services[1].id, "idx-2");
    }

    #[test]
    fn test_get_service_health() {
        let client = Client::tracked(rocket()).expect("valid rocket instance");
        let response = client.get("/eigen/node/services/db-1/health").dispatch();
        assert_eq!(response.status(), Status::Ok);

        let response = client.get("/eigen/node/services/idx-2/health").dispatch();
        assert_eq!(response.status(), Status::ServiceUnavailable);

        let response = client.get("/eigen/node/services/unknown/health").dispatch();
        assert_eq!(response.status(), Status::NotFound);
    }
}
