mod stops_index;
mod trips_index;
mod util;

use std::time::{Instant, SystemTime};
use gtfs_structures::{Gtfs, Id};
use stops_index::StopNamesIndex;
use trips_index::{TripsIndex, DirectTrip, find_route};
use util::{read_line, format_time};
use chrono::{DateTime, Local};

fn main() {
    let current_time: DateTime<Local> = DateTime::<Local>::from(SystemTime::now()).with_timezone(&Local);
    print!("Teraz je: {}\n", format_time(current_time));
    let gtfs = Gtfs::new("gtfs.zip").unwrap();
    let stops_index = StopNamesIndex::new(&gtfs);
    let trips_index = TripsIndex::new(&gtfs);
    let graph = trips_index.build_graph();

    let start_stop = "000000035700001"; //cintorin
    let end_stop = "000000009300025"; //zochova
    
    let start = Instant::now();

    let route = find_route(&graph, start_stop, end_stop, current_time);
  
    match route {
        Some(path) => {
            println!("Route found");
            for segment in path {
                println!(
                    "Trip ID: {}, Start Stop: {}, End Stop: {}, Departure: {:?}, Arrival: {:?}, Duration: {:?} seconds",
                    segment.trip_id,
                    segment.start_stop,
                    segment.end_stop,
                    DateTime::<Local>::from(segment.departure_time).format("%H:%M:%S"),
                    DateTime::<Local>::from(segment.arrival_time).format("%H:%M:%S"),
                    segment.duration.as_secs()
                );
            }
        },
        None => println!("No route available from {} to {}.", start_stop, end_stop),
    }

    let elapsed = start.elapsed();
    println!("Search took {} ms\n", elapsed.as_millis());

    // loop {
        // print!("Cintorin Slavicie (long: 17.068, lat: 48.158)\n");
        // print!("Zochova (long: 17.106, lat: 48.144)\n");
        // print!("Hodzovo (long: 17.107, lat: 48.148)\n");
        // print!("Zlate piesky Wakelake (long: 17.194, lat: 48.189)\n");
        // print!("Rajka (long: 17.204, lat: 47.993)\n");
        // print!("long: ");
        // let long = read_line().parse::<f64>().unwrap();

        // print!("lat: ");
        // let lat = read_line().parse::<f64>().unwrap();

        // print!("count: ");
        // let count = read_line().parse::<usize>().unwrap();

        // let nearest_stops = stops_index.find_nearest_stops(long, lat, count);

        // nearest_stops.iter().for_each(|stop| {
        //     println!("{}: {:.2} m", stop.stop_name, stop.distance_to_location(geo::Point::new(long, lat)));
        // });

        // print!("from: ");
        // let from_stop_name = read_line();
        
        // print!("  to: ");
        // let to_stop_name = read_line();

        // let start = Instant::now();
        
        // let from = *stops_index.search_by_name(from_stop_name.as_str()).get(0).unwrap();
        // let to = *stops_index.search_by_name(to_stop_name.as_str()).get(0).unwrap();



        // let mut all_trips: Vec<&DirectTrip> = vec![];

        // from.platforms.iter().for_each(|fp| {
        //     to.platforms.iter().for_each(|tp| {
        //         if let Some(trips) = trips_index.get_direct_trips(fp.id(), tp.id()) {
        //             all_trips.extend(trips);
        //         }
        //     });
        // });

        // let elapsed = start.elapsed();

        // println!("-> Going from {} to {}", from.stop_name, to.stop_name);
        // println!("-> Total trips {}", all_trips.len());
        // all_trips.iter().for_each(|t| {
        //     println!(
        //         "-> Trip {}, duration {} s, route {} -> {:#?}", 
        //         t.trip.id(), 
        //         t.get_duration(), 
        //         gtfs.get_route(&t.trip.route_id).map_or("-".to_string(), |r| r.short_name.clone().unwrap_or("-".to_string())), 
        //         t.get_stop_names()
        //     )
        // });
        // println!("-> Took {} ms\n", elapsed.as_millis());
    //}
}