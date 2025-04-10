/*
 * Copyright (C) 2025 Open Source Robotics Foundation
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
*/

use std::path::PathBuf;
use thiserror::Error as ThisError;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gz_msgs_name = "gz-msgs11";
    let libdir = pkg_config::get_variable(gz_msgs_name, "libdir")
        .map_err(box_error)?;

    let libdir_path = PathBuf::from(libdir);
    let base_protos_path = libdir_path.parent()
        .ok_or_else(|| box_error(MessageGenerationError::CannotFindShareDirectory))?
        .join("share")
        .join("gz")
        .join(gz_msgs_name)
        .join("protos");

    let protos_path = base_protos_path
        .join("gz")
        .join("msgs");

    prost_build::compile_protos(
        &[protos_path.join("scene.proto")],
        &[base_protos_path, protos_path],
    )
    .map_err(box_error)?;

    Ok(())
}

#[derive(ThisError, Debug)]
enum MessageGenerationError {
    #[error("The share directory of gz-msgs was not in its expected location relative to its library location")]
    CannotFindShareDirectory,
}

fn box_error<E: std::error::Error + 'static>(err: E) -> Box<dyn std::error::Error> {
    Box::new(err)
}
