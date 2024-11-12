import osmnx as ox
import networkx as nx
import pathlib
import json
import matplotlib.pyplot as plt

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
    if cli.args.image_generation:
        request_info["retrieval_type"] = "image"
        request_info["place"] = cli.args.place
        request_info["source"] = cli.args.source
    elif cli.args.place is not None:
        request_info["retrieval_type"] = "place"
        request_info["place"] = cli.args.place
    elif cli.args.latitude is not None and cli.args.longitude is not None and cli.args.radius is not None:
        request_info["retrieval_type"] = "coordinates"
        request_info["latitude"] = cli.args.latitude
        request_info["longitude"] = cli.args.longitude
        request_info["radius"] = cli.args.radius
    else:
        request_info["retrieval_type"] = "noop"
        return request_info
    
    request_info["path"] = cli.args.file_path if cli.args.file_path is not None else "output"
    request_info["name"] = cli.args.name if cli.args.name is not None else "default"
    request_info["save_results"] = not cli.args.dry_run

    return request_info
    
def process_request(request_info: dict) -> bool:
    nw = None
    match request_info["retrieval_type"]:
        case "image":
            match request_info["source"]:
                case "osm":
                    print_image("osm", request_info["place"], request_info["path"])
                    return True
                case "gmaps":
                    print_image("gmaps", request_info["place"], request_info["path"])
                    return True
                case "all":
                    print_all_images(request_info["place"], request_info["path"])
                    return True
                case _:
                    print(f"Faile to process images for source")
                    return False
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
    data = list(graph.nodes(data=True))
    for i in range(len(data)):
        entry = {
            "id": data[i][0],
            "latitude": data[i][1]["y"],
            "longitude": data[i][1]["x"],
        }
        data[i] = entry
    try:
        fullpath = path + "/nodes.json"
        with open(fullpath, 'w') as f:
            json.dump(data, f, indent=4, ensure_ascii=False)
        return True
    except Exception as e:
        print(f"Exception: {e}")
        return False

def save_edges_info(graph: nx.MultiDiGraph, path: str) -> bool:
    data = list(graph.edges(data=True))
    for i in range(len(data)):
        entry = {
            "id": int(data[i][2]["osmid"]),
            "start": data[i][0],
            "end": data[i][1],
            "lanes": float(data[i][2]["lanes"]),
            "maxspeed": int(data[i][2]["maxspeed"]),
            "length": data[i][2]["length"],
            "oneway": data[i][2]["oneway"],
            "highway": data[i][2]["highway"],
        }
        data[i] = entry
    try:
        fullpath = path + "/edges.json"
        with open(fullpath, 'w') as f:
            json.dump(data, f, indent=4, ensure_ascii=False)
        return True
    except Exception as e:
        print(f"Exception: {e}")
        return False

# Image processing functions

def print_all_images(place: str, path: str) -> bool:
    osm_data = print_image("osm", place, path)
    gmaps_data = print_image("gmaps", place, path)
    diff_data = print_diff(osm_data, gmaps_data, place, path)
    return True

def print_diff(source1: dict, source2: dict, place: str, path:str) -> dict:
    px = 1/plt.rcParams['figure.dpi']
    size = 2048*px

    graph = get_graph_from_place(place)
    diff_data_dict = {}
    for key in source1:
        diff_data_dict[key] = source1[key] - source2[key]
    ec = [diff_color_dict(diff_data_dict[(u, v)], 15) for u, v, _ in graph.edges(keys=True)]
    _, _ = ox.plot_graph(graph, node_color='w', node_edgecolor='k', node_size=0, figsize=(size,size), 
                           node_zorder=1, edge_color=ec, edge_linewidth=2, bgcolor='white', show=False, save=True, filepath=path+"/test_diff.png")
    return diff_data_dict

def print_image(source: str, place: str, path: str) -> dict:
    px = 1/plt.rcParams['figure.dpi']
    size = 2048*px

    graph = get_graph_from_place(place)
    markov_chain_filename = path + "/markov_chain_" + source + ".json"
    markov_chain_data = {}
    with open(markov_chain_filename) as json_file:
        markov_chain_data = json.load(json_file)
    markov_chain_data = [(x["street_data"]["start"], x["street_data"]["end"], x["traffic_data"]["estimated_density"]["Known"]) for x in markov_chain_data["graph"]]
    markov_chain_data_dict = {}
    for (u, v, d) in markov_chain_data:
        markov_chain_data_dict[(u, v)] = d
    ec = [density_color_dict(markov_chain_data_dict[(u, v)], 15) for u, v, _ in graph.edges(keys=True)]
    _, _ = ox.plot_graph(graph, node_color='w', node_edgecolor='k', node_size=0, figsize=(size,size), 
                           node_zorder=1, edge_color=ec, edge_linewidth=2, bgcolor='white', show=False, save=True, filepath=path+"/test_"+source+".png")
    return markov_chain_data_dict

def density_color_dict(value: float, max: float):
    if value is None:
        return 'blue'

    if value <= 7:
        return 'green'
    elif value <= 16:
        return 'lime'
    elif value <= 22:
        return 'orange'
    elif value <= 28:
        return 'red'
    else:
        return 'darkred'

def diff_color_dict(value: float, max: float):
    if value is None:
        return 'yellow'

    if value <= -20:
        return 'red'
    elif value <= -10:
        return 'lightcoral'
    elif value <= -3:
        return 'mistyrose'
    elif value <= 3:
        return 'lightgray'
    elif value <= 10:
        return 'lightsteelblue'
    elif value <= 20:
        return 'cornflowerblue'
    else:
        return 'blue'

# Main function

def main():
    request = process_args()
    if process_request(request_info=request):
        pass
    else:
        print("Failed to process request")

if __name__ == "__main__":
    main()
