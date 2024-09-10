import argparse

parser = argparse.ArgumentParser(description='Retrieve OSM graphs and data')
parser.add_argument('-s',
                    '--save',
                    type=str,
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

args = parser.parse_args()