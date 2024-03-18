# bimhd

## Authors
- Adam Hladík
- Martin Hric

## Introduction
Our project aims to develop a REST API in Rust using the Actix web framework to process General Transit Feed Specification (GTFS) data of public transportation in Bratislava. The goal is to provide a comprehensive tool for accessing and utilizing public transportation data efficiently. Through this project, we aim to enhance our understanding of API development and data processing in Rust.

## Objectives

### 1. Data Parsing
- Develop robust parsing mechanisms to interpret [the retrieved CSV files](https://opendata.bratislava.sk/dataset/show/cestovny-poriadok-20240311) compliant with the GTFS standard.
- Handle various data structures and formats present in the GTFS files, including routes, stops, schedules, and other relevant transit information.

### 2. REST API Development
- Design RESTful API endpoints using the Rust programming language to expose the retrieved GTFS data.

## Requirements
- Displaying routes from stop A to stop B:
  - Based on specific time
  - Overall (without specific time)
- Estimated Time of Arrival (ETA) (excluding walking)
- Listing available connections:
  - For a particular stop
    - Listing departures for connections at the given stop
- Ability to select specific types of vehicles
- Listing nearest stops for a specific GPS location
- Result caching (in memory / filesystem)
- Development metrics - Return processing time within responses

## Optional Requirements
- Rate limiting
- ETA including walking
- Authentication (JWT)
- Swagger

## Dependencies
- [Actix Web Framework](https://crates.io/crates/actix-web)
- [serde](https://crates.io/crates/serde)
- [gtfs-structure](https://crates.io/crates/gtfs-structures)
- (To be further explored and specified during development)
