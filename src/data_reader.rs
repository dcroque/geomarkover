/*
- Read intersections data
- Read street data
- Process intersection and street data into network graph
- Process network graph into markov chain graph
*/
use std::path::Path;
use std::fs::{read, File};
use std::io::BufReader;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Intersection {
    id: u64,
    latitude: f64,
    longitude: f64,
}

impl Intersection {
    fn new(id: u64, latitude: f64, longitude: f64) -> Self {
        Intersection {
            id,
            latitude,
            longitude,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Street {
    id: u64,
    start: u64,
    end: u64,
    lanes: f32,
    maxspeed: u8,
    length: f64,
    oneway: bool,
    highway: String,
}

impl Street {
    fn new(
        id: u64,
        start: u64,
        end: u64,
        lanes: f32,
        maxspeed: u8,
        length: f64,
        oneway: bool,
        highway: String,
    ) -> Self {
        Street {
            id,
            start,
            end,
            lanes,
            maxspeed,
            length,
            oneway,
            highway,
        }
    }
}

#[derive(Debug)]
pub struct NetworkData {
    name: String,
    nodes: Vec<Intersection>,
    edges: Vec<Street>,
}

impl NetworkData {
    fn new(name: String, nodes: Vec<Intersection>, edges: Vec<Street>) -> Self {
        NetworkData { name, nodes, edges }
    }

    fn new_from_file(name: String, files_location: String) -> Self {
        let node_filename = format!("{files_location}/nodes.json");
        let path = Path::new(&node_filename);
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let nodes: Vec<Intersection> =
            serde_json::from_reader(reader).expect("Nodes JSON was not well-formatted");

        let edge_filename = format!("{files_location}/edges.json");
        let path = Path::new(&edge_filename);
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let edges: Vec<Street> =
            serde_json::from_reader(reader).expect("Edges JSON was not well-formatted");

        NetworkData { name, nodes, edges }
    }
}

mod tests {
    use super::*;
    use crate::osm::get_data_from_place;

    #[test]
    fn read_output_from_osm() {
        get_data_from_place("jose_mendes", "José Mendes, Florianópolis");
        let _ = NetworkData::new_from_file("jose_mendes".to_string(), "output/jose_mendes".to_string());
    }
}
