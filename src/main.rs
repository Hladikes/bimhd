mod transit_index;
mod util;

use std::{collections::HashMap, sync::Arc};
use gtfs_structures::{Gtfs, Id};
use serde_json::to_string;
use tiny_http::{Header, Response, Server};
use transit_index::TransitIndex;
use util::format_u32_time;

fn main() {
    let gtfs = Gtfs::from_path("./gtfs.zip").expect("Could not open gtfs.zip file");
    let transit_index = TransitIndex::new(&gtfs);
    let server = Server::http("0.0.0.0:8000").expect("Failed to start the server");

    for request in server.incoming_requests() {
        let full_url = format!("http://localhost:8000{}", request.url());
        let parsed_url = url::Url::parse(&full_url).unwrap();
        let query_params: HashMap<String, String> = parsed_url
            .query_pairs()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        let response = match parsed_url.path() {
            "/api/v1/stops" => {
                let (stops, time_taken) = util::measure(|| {
                    if let Some(stop_name) = query_params.get("stop_name") {
                        let results = transit_index.search_by_name(stop_name);
                        results[0].platforms.clone()
                    } else {
                        let stops: Vec<Arc<gtfs_structures::Stop>> = gtfs.stops.values().cloned().collect();
                        stops
                    }
                });

                let response = serde_json::json!({
                    "time_taken": time_taken,
                    "stops": stops,
                });

                Response::from_string(to_string(&response).unwrap())
                    .with_status_code(200)
                    .with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
            }, 
            "/api/v1/swagger" => {
                Response::from_string(include_str!("../openapi.yaml"))
                    .with_status_code(200)
                    .with_header(Header::from_bytes(&b"Content-Type"[..], &b"text/plain"[..]).unwrap())
                    .with_header(Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap())
                    .with_header(Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, PUT, DELETE, OPTIONS"[..]).unwrap())
            },
            "/api/v1/stops/nearest" => {
                let parsed_lon = query_params.get("lon").and_then(|s| s.parse::<f64>().ok());
                let parsed_lat = query_params.get("lat").and_then(|s| s.parse::<f64>().ok());
                
                if let (Some(lon), Some(lat)) = (parsed_lon, parsed_lat) {
                    let max_count = query_params.get("max").and_then(|s| s.parse::<usize>().ok()).unwrap_or(5);
                    let nearest_stops = transit_index.find_nearest_stops(lon, lat, max_count);

                    Response::from_string(to_string(&nearest_stops).unwrap())
                        .with_status_code(200)
                        .with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
                } else {
                    let response = serde_json::json!({
                        "error": "Invalid lon and lat query parameters",
                    });

                    Response::from_string(to_string(&response).unwrap()).with_status_code(400)
                }
            },
            "/api/v1/trip" => {
                let (route, time_taken) = util::measure(|| {
                    let from_param = query_params.get("from").map_or("Cintorin Slavicie", |v| v.as_str());
                    let to_param = query_params.get("to").map_or("Hlavna stanica", |v| v.as_str());
                
                    let time_at = query_params.get("time_at").and_then(|time_str| {
                        time_str.split(':').map(|s| s.parse::<u32>().ok()).collect::<Option<Vec<u32>>>().and_then(|parts| {
                            if parts.len() == 2 {
                                Some(parts[0] * 3600 + parts[1] * 60)
                            } else {
                                None
                            }
                        })
                    });
                
                    let from_stop = transit_index.search_by_name(from_param).get(0).cloned().expect("Stop not found");
                    let to_stop = transit_index.search_by_name(to_param).get(0).cloned().expect("Stop not found");
                
                    let route = transit_index.find_route(from_stop, to_stop, time_at);
                    route
                });
                
                let response = route.map(|trips| {
                    let first_trip_departure = trips.first().map(|t| format_u32_time(t.get_departure_time()));
                    let last_trip_arrival = trips.last().map(|t| format_u32_time(t.get_arrival_time()));
            
                    serde_json::json!({
                        "time_taken": time_taken,
                        "departure_at": first_trip_departure,
                        "arrival_at": last_trip_arrival,
                        "trips": trips.iter().map(|trip| {
                            serde_json::json!({
                                "trip_id": trip.trip.id(),
                                "duration": trip.get_duration(),
                                "route": gtfs.get_route(&trip.trip.route_id).map_or("-".to_string(), |r| r.short_name.clone().unwrap_or("-".to_string())),
                                "stop_names": trip.get_stop_names(),
                            })
                        }).collect::<Vec<_>>()
                    })
                }).unwrap_or(serde_json::json!({
                    "error": "No route found"
                }));

                Response::from_string(to_string(&response).unwrap())
                    .with_status_code(200)
                    .with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
            },
            _ => Response::from_string("Not Found")
        };
        
        if let Err(e) = request.respond(response) {
            println!("Error sending response: {}", e);
        }
    }
}