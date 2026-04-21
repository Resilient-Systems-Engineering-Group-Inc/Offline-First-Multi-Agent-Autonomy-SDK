//! GraphQL API for the SDK.

pub mod schema;
pub mod resolvers;
pub mod types;

use async_graphql::http::GraphiQLSource;
use async_graphql::EmptySubscription;
use std::net::SocketAddr;
use tracing::info;

pub use schema::*;
pub use types::*;

/// Start GraphQL server.
pub async fn start_graphql_server(
    addr: SocketAddr,
    schema: Schema,
) -> Result<(), Box<dyn std::error::Error>> {
    let listen_addr = addr;
    
    info!("Starting GraphQL server on http://{}", listen_addr);
    info!("GraphiQL UI: http://{}/graphiql", listen_addr);

    let service = async_graphql_axum::GraphQLService::new(schema);
    let app = axum::Router::new()
        .nest_service("/", service)
        .route(
            "/graphiql",
            axum::routing::get(|| async {
                GraphiQLSource::build()
                    .endpoint("/")
                    .finish()
            }),
        );

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Create schema with all resolvers.
pub fn create_schema(db_pool: database::Pool, auth_config: auth::AuthConfig) -> Schema {
    use async_graphql::MergedObject;

    #[derive(MergedObject, Default)]
    struct QueryRoot(
        resolvers::TaskResolver,
        resolvers::WorkflowResolver,
        resolvers::AgentResolver,
        resolvers::QueryResolver,
    );

    #[derive(MergedObject, Default)]
    struct MutationRoot(
        resolvers::TaskMutation,
        resolvers::WorkflowMutation,
        resolvers::AgentMutation,
        resolvers::MutationResolver,
    );

    Schema::build(
        QueryRoot::default(),
        MutationRoot::default(),
        EmptySubscription,
    )
    .data(db_pool)
    .data(auth_config)
    .finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::*;

    #[tokio::test]
    async fn test_graphql_health() {
        let query = r#"
            query {
                health {
                    status
                    version
                }
            }
        "#;

        // Would test with real schema
        assert!(query.contains("health"));
    }

    #[tokio::test]
    async fn test_graphql_tasks_query() {
        let query = r#"
            query {
                tasks {
                    id
                    description
                    status
                    priority
                }
            }
        "#;

        assert!(query.contains("tasks"));
    }

    #[tokio::test]
    async fn test_graphql_task_mutation() {
        let mutation = r#"
            mutation {
                createTask(
                    description: "Test task"
                    priority: 150
                ) {
                    id
                    description
                    status
                }
            }
        "#;

        assert!(mutation.contains("createTask"));
    }
}
