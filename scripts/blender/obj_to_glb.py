# Used for reference: https://caretdashcaret.com/2015/05/19/how-to-run-blender-headless-from-the-command-line-without-the-gui/

# Usage: blender -b -P obj_to_glb.py -- <files_or_directories...>

# ---------------------------------------------
# Note: These imports are automatically performed by blender when the console is opened in the GUI
# We'll import these the same way as the GUI so that the rest of the script can be used seamlessly
# in either the headless or the GUI modes of blender.
import bpy
from bpy import data as D
from bpy import context as C
from mathutils import *
from math import *
# ---------------------------------------------

import argparse
import pathlib
import os

parser = argparse.ArgumentParser()

_, all_arguments = parser.parse_known_args()
double_dash_index = all_arguments.index('--')
script_args = all_arguments[double_dash_index + 1:]

parser.add_argument(
    'files_or_directories',
    type=pathlib.Path,
    nargs='+',
    help=(
        'Input individual .obj (wavefront) files or directories to look for '
        'files with an .obj extension inside of.'
    )
)
parser.add_argument(
    '-R', '--recursive',
    action='store_true',
    help='Traverse all directories recursively'
)
parser.add_argument(
    '-o', '--out-dir',
    type =pathlib.Path,
    default=None,
    help=(
        'Optionally name an output directory. If not provided, each .glb '
        'output file will be placed next to its source .obj file.'
    )
)
parser.add_argument(
    '-d', '--dry-run',
    action='store_true',
    help=(
        'Do a "dry run" that just identifies which files would be converted '
        'and where they would be placed if this script were run.'
    )
)

args = parser.parse_args(script_args)

# obj import settings: Y Forward Z Up
# glb export settings: turn off Y Up

all_input_paths = set()

def get_files_from_directory(directory):
    for path in directory.iterdir():
        path = pathlib.Path(path)
        print(f'looking at file {path}')
        if os.path.isfile(path):
            if path.suffix == '.obj':
                all_input_paths.add(path)
        elif os.path.isdir(path):
            print('is directory')
            if args.recursive:
                print('looking recursively...')
                get_files_from_directory(path)


for path in args.files_or_directories:
    path = path.resolve(strict=True)
    if os.path.isdir(path):
        print(f'Getting files from directory {path}')
        get_files_from_directory(path)
    elif os.path.isfile(path):
        all_input_paths.add(path)
    else:
        print(f'Unsupported path: {path}')

special_output_path = {}

if args.out_dir is not None:
    repeat_input_names = {}
    for path in all_input_paths:
        repeat_input_names.setdefault(path.stem, []).append(path)

    repeat_input_names = [(stem, paths) for (stem, paths) in repeat_input_names.items() if len(paths) > 1]
    for (stem, paths) in repeat_input_names:
        print(f'Duplicate exists for {stem}:\n{paths}')
        duplicate_exists = True
        depth = 1
        while duplicate_exists:
            depth += 1
            suggested_output_paths = {}
            maximum_depth_reached = True
            for p in paths:
                path_parts = p.parts
                if len(path_parts) < depth:
                    suggested_output_paths.setdefault(p, []).append(
                        p.with_suffix('.glb')
                    )
                else:
                    maximum_depth_reached = False
                    print(f'reducing {path_parts} to depth {depth}: {path_parts[-depth:]}')
                    output_path = pathlib.Path('').joinpath(
                        *path_parts[-depth:]
                    ).with_suffix('.glb')
                    suggested_output_paths.setdefault(output_path, []).append(p)

            duplicate_exists = False
            for input_paths in suggested_output_paths.values():
                if len(input_paths) > 1:
                    duplicate_exists = True
                    if maximum_depth_reached:
                        print(
                            f'Unable to infer unique path output names for '
                            f'conversion of these paths:\n{input_paths}'
                        )
                        exit(1)

            if not duplicate_exists:
                for (output, input) in suggested_output_paths.items():
                    special_output_path[input[0]] = args.out_dir.joinpath(output)


def get_output_path(input_path):
    if args.out_dir is None:
        return input_path.with_suffix('.glb')

    output_path = special_output_path.get(input_path, None)
    if output_path is not None:
        return output_path

    return args.out_dir.joinpath(
        pathlib.Path(input_path.name).with_suffix('.glb')
    )

if args.dry_run:
    for input_path in all_input_paths:
        print(f'{str(input_path)}\n -> {str(get_output_path(input_path))}')
    exit(0)

print(f'Converting meshes...')
counter = 0
errors = {}
for input_path in all_input_paths:
    counter += 1

    # Clear the scene before we start importing
    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.object.delete()

    print(f'{counter}/{len(all_input_paths)}: {input_path}')
    try:
        bpy.ops.import_scene.obj(
            filepath=str(input_path),
            axis_forward='Y',
            axis_up='Z'
        )

        try:
            output_path = get_output_path(input_path)
            bpy.ops.export_scene.gltf(
                filepath=str(output_path),
                export_format='GLB',
                export_yup=False
            )
        except Exception as e:
            errors[input_path] = e
    except Exception as e:
        errors[input_path] = e

if errors:
    print(f'Errors were encountered while running:')
    for (file, error) in errors.items():
        print(f'{file}:\n{error}\n')
