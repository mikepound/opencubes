from libraries.pcube import read, write, Orientation, Compression
from libraries.packing import pack, unpack
import numpy as np
import argparse

def npy_to_pcube(infile, outfile, orientation, compress):
    cubes = np.load(infile, allow_pickle=True)
    packed = [pack(cube) for cube in cubes]
    write(outfile, orientation, packed, compress)

def pcube_to_npy(infile, outfile):
    result = read(infile)
    unpacked = [unpack(cube) for cube in result.polycubes]
    np.save(outfile, unpacked)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        prog='Pollycube Converter',
        description='Converts .npy files into .pcube files and vice versa')

    parser.add_argument('filename', metavar='File Name', type=str,
                        help='The File to convert')
    parser.add_argument('--compress', dest='compress', action='append', type=int, default=0, choices=[0,1],
                        help='whether to compress the cubes if writing to the pcubes format, options are: 0: no compression 1: gzip compression')
    parser.add_argument('--orientation', dest='orientation', action='append', type=int, default=0, choices=[0,1],
                        help='whether the cubes are oriented, options are: 0: unoriented 1: orientated by bitwise highest value')
    args = parser.parse_args()

    filename: str = args.filename
    compress: Compression = Compression(args.compress)
    orientation: Orientation = Orientation(args.orientation)

    with open(filename, 'rb') as fp:
        if(filename.endswith('.npy')):
            output_file_name = filename.removesuffix('.npy') + '.pcube'
            with open(output_file_name, 'xb') as ofp:
                npy_to_pcube(fp, ofp, orientation, compress)
        elif(filename.endswith('.pcube')):
            output_file_name = filename.removesuffix('.pcube') + '.npy'
            with open(output_file_name, 'xb') as ofp:
                pcube_to_npy(fp, ofp)
        else:
            print(f'unkown file extension on file {filename}')

