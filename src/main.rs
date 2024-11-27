use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpResponse, HttpServer};
use std::sync::Arc;
use tracing::info;

mod game;
mod session;

async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logging with tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting game server...");

    // Shared game state
    let game_manager = Arc::new(game::GameManager::new());
    let game_manager_data = web::Data::new(game_manager);

    // Start server
    info!("Binding to 127.0.0.1:3001");
    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin_fn(|origin, _| {
                origin.as_bytes().starts_with(b"http://localhost:")
            })
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .app_data(game_manager_data.clone())
            .route("/health", web::get().to(health_check))
            .service(
                web::resource("/ws")
                    .route(web::get().to(session::handle_ws_connection))
            )
    })
    .workers(num_cpus::get())
    .bind("127.0.0.1:3001")?
    .run()
    .await
}