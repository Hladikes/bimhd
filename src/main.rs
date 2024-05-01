
mod transit_index;
mod util;

use std::{sync::Arc, time::{Instant, SystemTime}};
use gtfs_structures::{Gtfs, Id};
//use util::format_time;
use chrono::{DateTime, Local, Timelike};

use crate::{transit_index::{TransitIndex, DirectTrip}, util::read_line};

fn format_time(time: DateTime<Local>) -> String {
    format!("{:02}:{:02}:{:02}", time.hour(), time.minute(), time.second())
}

fn main() {
    let current_time: DateTime<Local> = DateTime::<Local>::from(SystemTime::now()).with_timezone(&Local);
    print!("Teraz je: {}\n", format_time(current_time));
    let gtfs = Gtfs::new("gtfs.zip").unwrap();
    let transit_index = TransitIndex::new(&gtfs);

    let cintorin = "000000035700001";
    let hlavna = "000000009300025"; 
    let zochova = "000000050000001";
    let bajkalska = "000000000800001";
    
    let start = Instant::now();

    let route = transit_index.find_route(cintorin, hlavna, None/*Some(43200)*/);
  
    match route {
        Some(path) => {
            path.iter().for_each(|trip| {
                let departure_time = format!("{:02}:{:02}", trip.get_departure_time() / 3600, (trip.get_departure_time() / 60) % 60);
                let arrival_time = format!("{:02}:{:02}", trip.get_arrival_time() / 3600, (trip.get_arrival_time() / 60) % 60);

                println!(
                    "Trip {}, duration {} s, route {} -> {:#?}, Depart at: {}, Arrive by: {}", 
                    trip.trip.id(), 
                    trip.get_duration(), 
                    gtfs.get_route(&trip.trip.route_id).map_or("-".to_string(), |r| r.short_name.clone().unwrap_or("-".to_string())),
                    trip.get_stop_names(),
                    departure_time,
                    arrival_time
                )
            });
        },
        None => println!("No route found")
    }


    let elapsed = start.elapsed();
    println!("Search took {} ms\n", elapsed.as_millis());

    // loop {
    //     print!("Cintorin Slavicie (long: 17.068, lat: 48.158)\n");
    //     print!("Zochova (long: 17.106, lat: 48.144)\n");
    //     print!("Hodzovo (long: 17.107, lat: 48.148)\n");
    //     print!("Zlate piesky Wakelake (long: 17.194, lat: 48.189)\n");
    //     print!("Rajka (long: 17.204, lat: 47.993)\n");
    //     print!("long: ");
    //     let long = read_line().parse::<f64>().unwrap();

    //     print!("lat: ");
    //     let lat = read_line().parse::<f64>().unwrap();

    //     print!("count: ");
    //     let count = read_line().parse::<usize>().unwrap();

    //     let nearest_stops = transit_index.find_nearest_stops(long, lat, count);

    //     nearest_stops.iter().for_each(|stop| {
    //         println!("{}: {:.2} m", stop.stop_name, stop.distance_to_location(geo::Point::new(long, lat)));
    //     });

    //     print!("from: ");
    //     let from_stop_name = read_line();
        
    //     print!("  to: ");
    //     let to_stop_name = read_line();

    //     let start = Instant::now();
        
    //     let from = transit_index.search_by_name(from_stop_name.as_str())[0].clone();
    //     let to = transit_index.search_by_name(to_stop_name.as_str())[0].clone();

    //     let mut all_trips: Vec<Arc<DirectTrip>> = vec![];

    //     from.platforms.iter().for_each(|fp| {
    //         to.platforms.iter().for_each(|tp| {
    //             if let Some(trips) = transit_index.get_direct_trips(fp.id(), tp.id()) {
    //                 all_trips.extend(trips.clone());
    //             }
    //         });
    //     });

    //     let elapsed = start.elapsed();

    //     println!("-> Going from {} to {}", from.stop_name, to.stop_name);
    //     println!("-> Total trips {}", all_trips.len());
    //     all_trips.iter().for_each(|t| {
    //         println!(
    //             "-> Trip {}, duration {} s, route {} -> {:#?}", 
    //             t.trip.id(), 
    //             t.get_duration(), 
    //             gtfs.get_route(&t.trip.route_id).map_or("-".to_string(), |r| r.short_name.clone().unwrap_or("-".to_string())), 
    //             t.get_stop_names()
    //         )
    //     });
    //     println!("-> Took {} ms\n", elapsed.as_millis());
    // }
}