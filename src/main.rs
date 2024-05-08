mod transit_index;
mod util;

use std::collections::{BTreeSet, HashMap};
use gtfs_structures::{Gtfs, Id};
use serde_json::to_string;
use tiny_http::{Header, Response, Server};
use transit_index::TransitIndex;
use util::{format_u32_time, format_seconds_to_minutes};

fn main() {
    let gtfs = Gtfs::from_url("https://www.arcgis.com/sharing/rest/content/items/aba12fd2cbac4843bc7406151bc66106/data").expect("Could not download gtfs file");
    //let gtfs = Gtfs::from_path("./gtfs-vienna.zip").expect("Could not open gtfs.zip file");
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
                let (stops_data, time_taken) = util::measure(|| {
                    if let Some(stop_name) = query_params.get("stop_name") {
                        let results = transit_index.search_by_name(stop_name);
                        serde_json::json!(vec![if let Some(first_stop_platforms) = results.first() {
                            serde_json::json!({
                                "stop_name": first_stop_platforms.stop_name,
                                "platforms": first_stop_platforms.platforms.iter().map(|platform| {
                                    serde_json::json!({
                                        "id": platform.id,
                                        "longitude": platform.longitude.unwrap_or(0.0),
                                        "latitude": platform.latitude.unwrap_or(0.0),
                                        "platform_code": platform.platform_code.as_ref().unwrap_or(&"".to_string()),
                                        "zone": platform.zone_id.as_ref().unwrap_or(&"".to_string())
                                    })
                                }).collect::<Vec<_>>()
                            })
                        } else {
                            serde_json::json!({
                                "stop_name": "",
                                "platforms": []
                            })
                        }])
                    } else {
                        serde_json::json!(transit_index.platforms.iter().map(|(stop_name, stop_platforms)| {
                            serde_json::json!({
                                "stop_name": transit_index.get_stop_name_from_id(stop_name),
                                "platforms": stop_platforms.platforms.iter().map(|platform| {
                                    serde_json::json!({
                                        "id": platform.id,
                                        "longitude": platform.longitude.unwrap_or(0.0),
                                        "latitude": platform.latitude.unwrap_or(0.0),
                                        "platform_code": platform.platform_code.as_ref().unwrap_or(&"".to_string()),
                                        "zone": platform.zone_id.as_ref().unwrap_or(&"".to_string())
                                    })
                                }).collect::<Vec<_>>()
                            })
                        }).collect::<Vec<_>>())
                    }
                });
            
                let response = serde_json::json!({
                    "time_taken": format!("{}", time_taken),
                    "stops": stops_data
                });
            
                Response::from_string(to_string(&response).unwrap())
                    .with_status_code(200)
                    .with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
                    .with_header(Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap())
                    .with_header(Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, PUT, DELETE, OPTIONS"[..]).unwrap())
            },
            "/api/v1/stops/routes/departures" => {
                if let Some(stop_name) = query_params.get("stop_name") {
                    
                    let (possibilities, time_taken) = util::measure(|| {
                        let stop_platforms = transit_index.search_by_name(stop_name)[0].clone();
                        let mut possibilities: HashMap<&str, BTreeSet<&str>> = HashMap::new();

                        stop_platforms.platforms.iter().for_each(|p| {
                            if let Some(from) = transit_index.stops_graph.get(p.id()) {
                                from.values().for_each(|trips| {
                                    trips.iter().for_each(|dt| {
                                        if let Ok(route) = gtfs.get_route(&dt.trip.route_id) {
                                            if let Some(route_name) = &route.short_name {
                                                let entry = possibilities.entry(route_name.as_str()).or_insert(BTreeSet::new());
                                                entry.insert(dt.stop_times.last().unwrap().stop.name.as_ref().unwrap().as_str());
                                            }
                                        }
                                    })
                                })
                            }
                        });

                        possibilities
                    });

                    let response = serde_json::json!({
                        "time_taken": time_taken,
                        "possibilities": possibilities,
                    });

                    Response::from_string(to_string(&response).unwrap()).with_status_code(400)
                        .with_status_code(200)
                        .with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
                        .with_header(Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap())
                        .with_header(Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, PUT, DELETE, OPTIONS"[..]).unwrap())
                } else {
                    let response = serde_json::json!({
                        "error": "Invalid stop_name query parameter",
                    });

                    Response::from_string(to_string(&response).unwrap()).with_status_code(400)
                        .with_header(Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap())
                        .with_header(Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, PUT, DELETE, OPTIONS"[..]).unwrap())
                }
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
                    let (nearest_stops, time_taken) = util::measure(|| {
                        transit_index.find_nearest_stops(lon, lat, max_count)
                    });

                    let response = serde_json::json!({
                        "time_taken": time_taken,
                        "nearest_stops": nearest_stops.iter().map(|stop_platform| {
                            serde_json::json!({
                                "stop_name": stop_platform.stop_name,
                                "distance": format!("{:.2} m", stop_platform.distance_to_location(geo::Point::new(lon, lat))),
                                "platforms": stop_platform.platforms.iter().map(|stop| {
                                    serde_json::json!({
                                        "id": stop.id,
                                        "platform_code": stop.platform_code,
                                        "zone": stop.zone_id,
                                        "latitude": stop.latitude,
                                        "longitude": stop.longitude,
                                    })
                                }).collect::<Vec<_>>()
                            })
                        }).collect::<Vec<_>>(),
                    });
            

                    Response::from_string(to_string(&response).unwrap())
                        .with_status_code(200)
                        .with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
                        .with_header(Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap())
                        .with_header(Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, PUT, DELETE, OPTIONS"[..]).unwrap())
                } else {
                    let response = serde_json::json!({
                        "error": "Invalid lon and lat query parameters",
                    });

                    Response::from_string(to_string(&response).unwrap()).with_status_code(400)
                        .with_header(Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap())
                        .with_header(Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, PUT, DELETE, OPTIONS"[..]).unwrap())
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
                                "departure_at": format_u32_time(trip.get_departure_time()),
                                "arrival_at": format_u32_time(trip.get_arrival_time()),
                                "trip_id": trip.trip.id(),
                                "duration": format_seconds_to_minutes(trip.get_duration()),
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
                    .with_header(Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap())
                    .with_header(Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, PUT, DELETE, OPTIONS"[..]).unwrap())
            },
            _ => Response::from_string("Not Found")
        };
        
        if let Err(e) = request.respond(response) {
            println!("Error sending response: {}", e);
        }
    }
}