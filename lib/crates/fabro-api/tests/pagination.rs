//! Tests that paginated list endpoints return `{ data, meta: { has_more } }`.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use fabro_api::jwt_auth::AuthMode;
use fabro_api::server::{build_router, create_app_state};
use fabro_interview::Interviewer;
use fabro_workflows::handler::exit::ExitHandler;
use fabro_workflows::handler::start::StartHandler;
use fabro_workflows::handler::HandlerRegistry;
use tower::ServiceExt;

fn test_registry(_interviewer: Arc<dyn Interviewer>) -> HandlerRegistry {
    let mut registry = HandlerRegistry::new(Box::new(StartHandler));
    registry.register("start", Box::new(StartHandler));
    registry.register("exit", Box::new(ExitHandler));
    registry
}

async fn test_db() -> sqlx::SqlitePool {
    let pool = fabro_db::connect_memory().await.unwrap();
    fabro_db::initialize_db(&pool).await.unwrap();
    pool
}

async fn get_json(app: axum::Router, uri: &str) -> serde_json::Value {
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .header("x-fabro-demo", "1")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK, "GET {uri} failed");
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap()
}

/// Assert that a value has the paginated shape: `{ data: [...], meta: { has_more: bool } }`
fn assert_paginated_shape(json: &serde_json::Value, context: &str) {
    assert!(json.get("data").is_some(), "{context}: missing 'data' key");
    assert!(json["data"].is_array(), "{context}: 'data' is not an array");
    assert!(json.get("meta").is_some(), "{context}: missing 'meta' key");
    assert!(
        json["meta"].get("has_more").is_some(),
        "{context}: missing 'meta.has_more'"
    );
    assert!(
        json["meta"]["has_more"].is_boolean(),
        "{context}: 'meta.has_more' is not boolean"
    );
}

struct PaginatedEndpoint {
    path: &'static str,
    name: &'static str,
    default_has_more: bool,
}

const ENDPOINTS: &[PaginatedEndpoint] = &[
    PaginatedEndpoint {
        path: "/workflows",
        name: "listWorkflows",
        default_has_more: false,
    },
    PaginatedEndpoint {
        path: "/workflows/implement/runs",
        name: "listWorkflowRuns",
        default_has_more: false,
    },
    PaginatedEndpoint {
        path: "/retros",
        name: "listRetros",
        default_has_more: false,
    },
    PaginatedEndpoint {
        path: "/sessions",
        name: "listSessions",
        default_has_more: false,
    },
    PaginatedEndpoint {
        path: "/insights/queries",
        name: "listSavedQueries",
        default_has_more: false,
    },
    PaginatedEndpoint {
        path: "/insights/history",
        name: "listQueryHistory",
        default_has_more: false,
    },
    PaginatedEndpoint {
        path: "/models",
        name: "listModels",
        default_has_more: true,
    },
    PaginatedEndpoint {
        path: "/runs/run-1/stages/detect-drift/turns",
        name: "listStageTurns",
        default_has_more: false,
    },
    PaginatedEndpoint {
        path: "/runs/run-1/questions",
        name: "listRunQuestions",
        default_has_more: false,
    },
    PaginatedEndpoint {
        path: "/runs/run-1/stages",
        name: "listRunStages",
        default_has_more: false,
    },
    PaginatedEndpoint {
        path: "/runs/run-1/verification",
        name: "retrieveRunVerification",
        default_has_more: false,
    },
];

#[tokio::test]
async fn paginated_endpoints_return_correct_shape() {
    let state = create_app_state(test_db().await, test_registry);
    let app = build_router(state, AuthMode::Disabled);

    for ep in ENDPOINTS {
        // Default request: paginated shape, with endpoint-specific default has_more.
        let json = get_json(app.clone(), ep.path).await;
        assert_paginated_shape(&json, ep.name);
        assert_eq!(
            json["meta"]["has_more"], ep.default_has_more,
            "{}: unexpected default has_more value",
            ep.name,
        );

        // limit=1: at most 1 item, has_more = true (all fixtures have >1 item)
        let uri = if ep.path.contains('?') {
            format!("{}&page[limit]=1", ep.path)
        } else {
            format!("{}?page[limit]=1", ep.path)
        };
        let json = get_json(app.clone(), &uri).await;
        assert_paginated_shape(&json, &format!("{} limit=1", ep.name));
        assert!(
            json["data"].as_array().unwrap().len() <= 1,
            "{}: limit=1 returned more than 1 item",
            ep.name
        );
        assert_eq!(
            json["meta"]["has_more"], true,
            "{}: limit=1 should have has_more=true",
            ep.name
        );
    }
}
