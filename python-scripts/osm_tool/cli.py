import argparse

# Example
# poetry run python3 osm_tool/__init__.py -p "José Mendes, Florianópolis" -n jose_mendes

parser = argparse.ArgumentParser(description='Retrieve OSM graphs and data')
parser.add_argument('-d',
                    '--dry_run',
                    action='store_true',
                    help='Save results to file')

parser.add_argument('-p',
                    '--place',
                    type=str,
                    help='Place for retrieving data')

parser.add_argument('-a',
                    '--latitude',
                    type=float,
                    help='Latitude for the center of data retrieval')

parser.add_argument('-o',
                    '--longitude',
                    type=float,
                    help='Longitude for the center of data retrieval')

parser.add_argument('-r',
                    '--radius',
                    type=int,
                    help='Radius around the center of data retrieval')

parser.add_argument('-f',
                    '--file_path',
                    type=str,
                    help='Path for storing results')

parser.add_argument('-n',
                    '--name',
                    type=str,
                    help='Name for the data retrieval results')

args = parser.parse_args()