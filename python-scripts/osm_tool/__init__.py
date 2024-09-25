import osmnx as ox
import networkx as nx
import pathlib
import json

import cli

default_needed_keys = ["osmid", "oneway", "lanes", "reversed", "maxspeed", "highway"]
result_dict_needed = {
    "osmid": str,
    "oneway": bool,
    "lanes": int,
    "reversed": bool,
    "maxspeed": float,
    "highway": str,
}

#para vias urbanas:
# – trânsito rápido: 80 km/h;
# – arterial: 60 km/h;
# – coletora: 40 km/h;
# – local: 30 km/h;
default_speeds = {
    "secundary": 40,
    "service": 30,
    "residential": 30,
}

# CLI interaction functions

def process_args() -> dict:
    request_info = {}
    if cli.args.place is not None:
        request_info["retrieval_type"] = "place"
        request_info["place"] = cli.args.place
    elif cli.arg.latitude is not None and cli.arg.longitude is not None and cli.arg.radius is not None:
        request_info["retrieval_type"] = "coordinates"
        request_info["latitude"] = cli.args.latitude
        request_info["longitude"] = cli.args.longitude
        request_info["radius"] = cli.args.radius
    else:
        request_info["retrieval_type"] = "noop"
        return request_info
    
    request_info["path"] = cli.args.file_path if cli.args.file_path is not None else "../output"
    request_info["name"] = cli.args.name if cli.args.name is not None else "default"
    request_info["save_results"] = not cli.args.dry_run

    return request_info
    
def process_request(request_info: dict) -> bool:
    nw = None
    match request_info["retrieval_type"]:
        case "place":
            nw = get_graph_from_place(request_info["place"])
        case "coordinates":
            nw = get_graph_from_coord(request_info["latitude"], request_info["longitude"], request_info["radius"])
        case _:
            print("Noop")
            return False
    return save_graph(graph=nw, graph_name=request_info["name"], base_path=request_info["path"], prune_keys=["geometry"], inverted_prune=False)

# OSMNX graph retrieval functions

def get_graph_from_place(place: str) -> nx.MultiDiGraph:
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

# Graph processing functions

def save_graph(
        graph: nx.MultiDiGraph, 
        graph_name: str, 
        base_path: str = "output", 
        prune_keys: list[str] = [],
        inverted_prune: bool = False,
        needed_keys: list[str] = result_dict_needed) -> bool:
    path = base_path + "/" + graph_name + "/"

    if not create_path(path):
        print(f"Error saving graph at {path}: Failed to create directory")
        return False

    if not check_graph_integrity(graph, needed_keys):
        print(f"Error saving graph at {path}: Graph integrity issues")
        return False

    if len(prune_keys) >= 1:
        graph = prune_graph_info(graph, prune_keys, inverted_prune)
            
    if not save_nodes_info(graph, path):
        print(f"Error saving graph at {path}: Failed to save node data")
        return False

    if not save_edges_info(graph, path):
        print(f"Error saving graph at {path}: Failed to save edge data")
        return False

    return True

def create_path(path: str) -> bool:
    try:
        pathlib.Path(path).mkdir(parents=True, exist_ok=True)
        return True
    except Exception as e:
        print(f"Failed to create path {path} with exception: {e}")
        return False

def check_graph_integrity(graph: nx.MultiDiGraph, needed_keys: list[str]) -> bool:
    def try_fix(missing_field: str, edge_info: dict) -> any:
        match missing_field:
            case "maxspeed":
                if "speed_kph" in edge_info:
                    return edge_info["speed_kph"]
                highway_type = edge_info["highway"]
                return default_speeds[highway_type]
            case "lanes":
                return 1
            case _:
                raise Exception()

    for node in graph.nodes(data=True):
        if (node[1]["x"] is None or 
            node[1]["y"] is None or 
            node[0] is None):
            return False
        
    node_list = list(graph.nodes)

    for edge in graph.edges(data=True):
        if not edge[0] in node_list or not edge[1] in node_list:
            print(f"Edge ({edge[0]}, {edge[1]}) not found in node data")
            return False
        for key, value_type in needed_keys.items():
            if key not in edge[2]:
                # print(f"WARN Edge ({edge[0]}, {edge[1]}) missing '{key}' data, trying default values")
                try:
                    edge[2][key] = try_fix(key, edge[2])
                    # print(f"Default value inserted for {key} field!")
                except:
                    print("Failed to fix. Full edge info: \n")
                    print(edge[2])
                    return False
            if type(edge[2][key]) == list:
                try:
                    edge[2][key] = [value_type(x) for x in edge[2][key]]
                    edge[2][key] = min(edge[2][key])
                except:
                    print(f"Type fixing error: List {edge[2][key]} from [{key}] for type {value_type} has no minimum")
                    print(edge)
                    return False
            elif type(edge[2][key]) != value_type:
                try:
                    edge[2][key] = value_type(edge[2][key])
                except:
                    print(f"Type fixing error: {edge[2][key]} from [{key}] for type {value_type}")
                    print(edge)
                    return False

    return True

def prune_graph_info(graph: nx.MultiDiGraph, prune_keys: list[str], is_inverted: bool = False) -> nx.MultiDiGraph:
    if is_inverted:
        for edge in graph.edges(data=True):
            remove_list = []
            for key, _ in edge[2].items():
                if key not in prune_keys:
                    remove_list.append(key)
            for key in remove_list:
                edge[2].pop(key)
    else:
        for edge in graph.edges(data=True):
            for key in prune_keys:
                if key in edge[2]:
                    edge[2].pop(key)
    return graph

def save_nodes_info(graph: nx.MultiDiGraph, path: str) -> bool:
    try:
        data = list(graph.nodes(data=True))
        fullpath = path + "/nodes.json"
        with open(fullpath, 'w') as f:
            json.dump(data, f, indent=4, ensure_ascii=False)
        return True
    except Exception as e:
        print(f"Exception: {e}")
        return False

def save_edges_info(graph: nx.MultiDiGraph, path: str) -> bool:
    try:
        data = list(graph.edges(data=True))
        fullpath = path + "/edges.json"
        with open(fullpath, 'w') as f:
            json.dump(data, f, indent=4, ensure_ascii=False)
        return True
    except Exception as e:
        print(f"Exception: {e}")
        return False

# Main function

def main():
    request = process_args()
    if process_request(request_info=request):
        print("Success")
    else:
        print("Failed")

if __name__ == "__main__":
    main()
