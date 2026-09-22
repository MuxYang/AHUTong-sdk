use axum::{
    Router,
    routing::{get, post},
};

use super::handlers::{self, AppState};

pub fn router(state: AppState) -> Router {
    let router = Router::new()
        .route("/health", get(handlers::health))
        .route("/init", post(handlers::init))
        .route("/cookies/dump", get(handlers::dump_cookies))
        .route("/cookies/flat", get(handlers::cookies_flat));
    #[cfg(not(target_vendor = "apple"))]
    let router = router.route("/login", post(handlers::login));
    router
        .route("/schedule", get(handlers::schedule))
        .route("/schedule/next", get(handlers::next_schedule))
        .route("/schedule/current-week", get(handlers::current_week))
        .route("/exam", get(handlers::exam))
        .route("/grade", get(handlers::grade))
        .route("/grade/profiles", get(handlers::grade_profiles))
        .route("/grade/rank", get(handlers::grade_rank))
        .route("/ycard/balance", get(handlers::balance))
        .route("/ycard/qrcode", get(handlers::qrcode))
        .route("/ycard/refresh_token", post(handlers::refresh_token))
        .with_state(state)
}
