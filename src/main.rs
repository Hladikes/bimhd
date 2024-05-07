mod util;
mod transit_index;

use std::sync::Arc;
use actix_cors::Cors;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use gtfs_structures::Gtfs;
use serde::{Deserialize, Serialize};
use transit_index::{StopPlatforms, TransitIndex};

struct AppData<'a> {
    transit_index: Arc<transit_index::TransitIndex<'a>>,
}

#[get("/stops")]
async fn get_stops(data: web::Data<AppData<'_>>) -> impl Responder {
    let (stops, time_taken) = util::measure(|| {
        let stops: Vec<Arc<gtfs_structures::Stop>> = data.transit_index.gtfs.stops.values().cloned().collect();
        stops
    });
    
    #[derive(Serialize)]
    struct StopsResponse {
        time_taken: u128,
        stops: Vec<Arc<gtfs_structures::Stop>>,
    }

    HttpResponse::Ok().json(StopsResponse {
        time_taken,
        stops,
    })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // let url = "https://www.arcgis.com/sharing/rest/content/items/aba12fd2cbac4843bc7406151bc66106/data";
    // let gtfs = Gtfs::from_url(url).unwrap();

    let transit_index = TransitIndex::new();
    transit_index.build_platforms();
    transit_index.build_direct_trips();
    transit_index.build_distances();
    
    let app_data = web::Data::new(AppData {
        transit_index: Arc::new(transit_index),
    });

    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allow_any_origin() // Allow any origin. Use carefully, especially in production.
                    .allowed_methods(vec!["GET", "POST"]) // Allow specific HTTP methods
                    .allowed_headers(vec!["Content-Type"]) // Allow specific headers
                    .max_age(3600), // Preflight request cache duration (1 hour)
            )
            .app_data(app_data.clone())
            .service(get_stops)
    })
    .bind(("0.0.0.0", 8000))?
    .run()
    .await
}