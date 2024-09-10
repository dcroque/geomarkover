import osmnx as ox
import networkx as nx
import pathlib
import json

import cli

def process_args():
    for _, arg in vars(cli.args).items():
        argtype = type(arg)
        print(f"{arg}: {argtype}")

def get_graph_from_place(place_name: str) -> nx.MultiDiGraph:
    nw = ox.graph_from_place(query=place, simplify=True, network_type='drive')
    nw = ox.add_edge_speeds(nw)
    nw = ox.add_edge_travel_times(nw)
    return nw

def get_graph_from_coord(lati: float, long: float, radius: int) -> nx.MultiDiGraph:
    point = lati, long
    nw = ox.graph_from_point(center_point=point, dist=radius, simplify=True, network_type="drive")
    nw = ox.add_edge_speeds(nw)
    nw = ox.add_edge_travel_times(nw)
    return nw

def create_path(path: str) -> bool:
    try:
        pathlib.Path(path).mkdir(parents=True, exist_ok=True)
        return True
    except e:
        print(f"Failed to create path {path} with exception: {e}")
        return False

def save_graph():
    pass

def save_node_info():
    pass

def save_edges_info():
    pass

def prune_graph_info(nw: nx.MultiDiGraph ) -> nx.MultiDiGraph:
    pass

def main():
    place = "Jose Mendes, Florianópolis"
    nw = ox.graph_from_place(query=place, simplify=True, network_type='drive')
    nw = ox.add_edge_speeds(nw)
    nw = ox.add_edge_travel_times(nw)
    edges = list(nw.edges(data=True))
    for key,value in edges[0][2].items():
        print(f"{key}: {value}")
    print()

    nodes = list(nw.nodes(data=True))
    print(nodes[0])
    print()

    process_args()
if __name__ == "__main__":
    main()
