/*
- Read intersections data
- Read street data
- Process intersection and street data into network graph
- Process network graph into markov chain graph
*/

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
    start: Intersection,
    end: Intersection,
    lanes: u8,
    maxspeed: u8,
    length: f64,
    oneway: bool,
    highway: String,
}

impl Street {
    fn new(
        id: u64,
        start: Intersection,
        end: Intersection,
        lanes: u8,
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
pub struct NetworkGraph {
    name: String,
    nodes: Vec<Intersection>,
    edges: Vec<Street>,
}

impl NetworkGraph {
    fn new(
        name: String,
        nodes: Vec<Intersection>,
        edges: Vec<Street>,
    ) -> Self {
        NetworkGraph {
            name,
            nodes,
            edges,
        }
    }
}
