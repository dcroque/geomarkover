/*
- Read intersections data
- Read street data
- Process intersection and street data into network graph
- Process network graph into markov chain graph
*/
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Intersection {
    pub id: u64,
    pub latitude: f64,
    pub longitude: f64,
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

#[derive(Debug, Deserialize, Clone)]
pub struct Street {
    pub id: u64,
    pub start: u64,
    pub end: u64,
    pub lanes: f32,
    pub maxspeed: u8,
    pub length: f64,
    pub oneway: bool,
    pub highway: String,
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
    pub name: String,
    pub nodes: Vec<Intersection>,
    pub edges: Vec<Street>,
}

impl NetworkData {
    pub fn new(name: String, nodes: Vec<Intersection>, edges: Vec<Street>) -> Self {
        NetworkData { name, nodes, edges }
    }

    pub fn new_from_file(name: String, files_location: String) -> Self {
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
        let _ =
            NetworkData::new_from_file("jose_mendes".to_string(), "output/jose_mendes".to_string());
    }
}
