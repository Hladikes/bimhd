
mod transit_index;
mod util;

use std::{sync::Arc, time::Instant};
use gtfs_structures::{Gtfs, Id};

use crate::{transit_index::TransitIndex, util::{read_line, format_u32_time}};

fn main() {
    let gtfs = Gtfs::new("gtfs.zip").unwrap();
    let transit_index = TransitIndex::new(&gtfs);

    loop {
        print!("from: ");
        let from_stop_name = read_line();

        print!("  to: ");
        let to_stop_name = read_line();

        let from: Arc<transit_index::StopPlatforms> = transit_index.search_by_name(from_stop_name.as_str())[0].clone();
        let to: Arc<transit_index::StopPlatforms> = transit_index.search_by_name(to_stop_name.as_str())[0].clone();
        
        let start = Instant::now();

        let route = transit_index.find_route(&from, &to ,None);
    
        match route {
            Some(path) => {
                path.iter().for_each(|trip| {
                    println!(
                        "Trip {}, duration {} s, route {} -> {:#?}, Depart at: {}, Arrive by: {}", 
                        trip.trip.id(), 
                        trip.get_duration(), 
                        gtfs.get_route(&trip.trip.route_id).map_or("-".to_string(), |r| r.short_name.clone().unwrap_or("-".to_string())),
                        trip.get_stop_names(),
                        format_u32_time(trip.get_departure_time()),
                        format_u32_time(trip.get_arrival_time())
                    )
                });
            },
            None => println!("No route found")
        }
        let elapsed = start.elapsed();
        println!("Search took {} ms\n", elapsed.as_millis());
    }

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
        
        // let from = transit_index.search_by_name(from_stop_name.as_str())[0].clone();
        // let to = transit_index.search_by_name(to_stop_name.as_str())[0].clone();

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