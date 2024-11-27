/*
 * Copyright (C) 2023 Open Source Robotics Foundation
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

use std::{f32::consts::PI, io::Write, path::PathBuf};

use bevy::{
    asset::{
        io::{AssetReaderError, Reader, VecReader},
        AssetPath,
    },
    prelude::{Mesh, Vec2},
    render::{mesh::Indices, render_resource::PrimitiveTopology},
};
use itertools::Itertools;
use utm::{lat_lon_to_zone_number, to_utm_wgs84};

use crate::site_asset_io::cache_path;

const EARTH_RADIUS: f32 = 6371.0;

fn haversine_distance(lat1: f32, lon1: f32, lat2: f32, lon2: f32) -> f32 {
    let lat1 = lat1.to_radians();
    let lon1 = lon1.to_radians();
    let lat2 = lat2.to_radians();
    let lon2 = lon2.to_radians();

    let d_lan = lat2 - lat1;
    let d_lon = lon2 - lon1;

    let a = (d_lan / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (d_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    return c * EARTH_RADIUS;
}

#[test]
fn test_haversine() {
    // Distance from Singapore to Kolkata (about 2891KM)
    let d = haversine_distance(1.3521, 103.8198, 22.5726, 88.3639);
    assert!((d - 2891.0).abs() < 1.0);

    // Distance from car_factory to whale_museum (about 3.48KM)
    let car_factory = (35.503201188171076, 129.3809451273798);
    let whale_museum = (35.53330554519475, 129.38965867799482);

    let d = haversine_distance(car_factory.0, car_factory.1, whale_museum.0, whale_museum.1);
    assert!((d - 3.48).abs() < 0.1);

    // Distance from one-north mrt in Singapore to SUTD (about 20.2KM)
    let one_north = (1.2991849898682075, 103.78709256771138);
    let sutd = (1.3417113432463037, 103.96381226270485);

    let d = haversine_distance(one_north.0, one_north.1, sutd.0, sutd.1);
    assert!((d - 20.2).abs() < 0.1);
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct OSMTile {
    xtile: i32,
    ytile: i32,
    zoom: i32,
}

impl TryFrom<PathBuf> for OSMTile {
    type Error = String;

    fn try_from(p: PathBuf) -> Result<Self, Self::Error> {
        let (zoom, xtile, ytile) = p
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect_tuple()
            .ok_or(
                "Invalid path when converting to OSMTile, three elements are required".to_owned(),
            )?;
        let ytile = ytile.strip_suffix(".png").ok_or("Suffix not found")?;
        Ok(OSMTile {
            xtile: xtile.parse::<i32>().map_err(|e| e.to_string())?,
            ytile: ytile.parse::<i32>().map_err(|e| e.to_string())?,
            zoom: zoom.parse::<i32>().map_err(|e| e.to_string())?,
        })
    }
}

impl From<&OSMTile> for AssetPath<'_> {
    fn from(t: &OSMTile) -> Self {
        let mut path: PathBuf = [t.zoom, t.xtile, t.ytile]
            .iter()
            .map(|v| v.to_string())
            .collect();
        path.set_extension("png");
        AssetPath::from(path).with_source("osm-tile")
    }
}

impl OSMTile {
    pub fn zoom(&self) -> i32 {
        self.zoom
    }

    pub fn get_quad_mesh(&self) -> Option<Mesh> {
        let nw = self.get_nw_corner();
        let Ok(nw) = self.get_transform_from_lat_lon(nw.0, nw.1) else {
            return None;
        };

        let ne = self.get_ne_corner();
        let Ok(ne) = self.get_transform_from_lat_lon(ne.0, ne.1) else {
            return None;
        };

        let sw = self.get_sw_corner();
        let Ok(sw) = self.get_transform_from_lat_lon(sw.0, sw.1) else {
            return None;
        };

        let se = self.get_se_corner();
        let Ok(se) = self.get_transform_from_lat_lon(se.0, se.1) else {
            return None;
        };

        let (u_left, u_right) = (0.0, 1.0);
        let vertices = [
            ([sw.x, sw.y, 0.0], [0.0, 0.0, 1.0], [u_left, 1.0]),
            ([nw.x, nw.y, 0.0], [0.0, 0.0, 1.0], [u_left, 0.0]),
            ([ne.x, ne.y, 0.0], [0.0, 0.0, 1.0], [u_right, 0.0]),
            ([se.x, se.y, 0.0], [0.0, 0.0, 1.0], [u_right, 1.0]),
        ];

        let indices = Indices::U32(vec![0, 2, 1, 0, 3, 2]);

        let positions: Vec<_> = vertices.iter().map(|(p, _, _)| *p).collect();
        let normals: Vec<_> = vertices.iter().map(|(_, n, _)| *n).collect();
        let uvs: Vec<_> = vertices.iter().map(|(_, _, uv)| *uv).collect();

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.set_indices(Some(indices));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        Some(mesh)
    }

    /// Returns the northwest corner
    pub fn get_nw_corner(&self) -> (f32, f32) {
        let n = 2f32.powi(self.zoom);
        let lon_deg = self.xtile as f32 / n * 360.0 - 180.0;
        let lat_rad = (PI * (1.0 - 2.0 * self.ytile as f32 / n)).sinh().atan();
        let lat_deg = lat_rad.to_degrees();
        (lat_deg, lon_deg)
    }

    pub fn get_center(&self) -> (f32, f32) {
        let n = 2f32.powi(self.zoom);
        let lon_deg = (self.xtile as f32 + 0.5) / n * 360.0 - 180.0;
        let lat_rad = (PI * (1.0 - 2.0 * (self.ytile as f32 + 0.5) / n))
            .sinh()
            .atan();
        let lat_deg = lat_rad.to_degrees();
        (lat_deg, lon_deg)
    }

    pub fn get_se_corner(&self) -> (f32, f32) {
        Self {
            zoom: self.zoom,
            xtile: self.xtile + 1,
            ytile: self.ytile + 1,
        }
        .get_nw_corner()
    }

    pub fn get_ne_corner(&self) -> (f32, f32) {
        Self {
            zoom: self.zoom,
            xtile: self.xtile + 1,
            ytile: self.ytile,
        }
        .get_nw_corner()
    }

    pub fn get_sw_corner(&self) -> (f32, f32) {
        Self {
            zoom: self.zoom,
            xtile: self.xtile,
            ytile: self.ytile + 1,
        }
        .get_nw_corner()
    }

    /// Returns the position of an item in meters on the given tile
    pub fn get_transform_from_lat_lon(&self, lat: f32, lon: f32) -> Result<Vec2, String> {
        let (self_lat, self_lon) = self.get_center();
        let (self_lat, self_lon) = (self_lat as f64, self_lon as f64);
        let zone = lat_lon_to_zone_number(lat.into(), lon.into());
        let self_zone = lat_lon_to_zone_number(self_lat.into(), self_lon.into());

        if zone != self_zone {
            return Err("Scale is crossing zones. We don't know how to convert".to_string());
        }

        let (northing, easting, _convergence) = to_utm_wgs84(lat as f64, lon as f64, zone);
        let (self_northing, self_easting, _convergence) = to_utm_wgs84(self_lat, self_lon, zone);
        Ok(Vec2::new(
            (easting - self_easting) as f32,
            (northing - self_northing) as f32,
        ))
    }

    /// Returns size of tile in meters.
    pub fn tile_size(&self) -> (f32, f32) {
        let (lat1, lon1) = self.get_nw_corner();
        let dy = {
            let (lat2, lon2) = self.get_sw_corner();
            if let Ok(res1) = self.get_transform_from_lat_lon(lat1, lon1) {
                if let Ok(res2) = self.get_transform_from_lat_lon(lat2, lon2) {
                    (res1 - res2).length()
                } else {
                    haversine_distance(lat1, lon1, lat2, lon2) * 1000.0
                }
            } else {
                haversine_distance(lat1, lon1, lat2, lon2) * 1000.0
            }
        };
        let dx = {
            let (lat2, lon2) = self.get_ne_corner();
            if let Ok(res1) = self.get_transform_from_lat_lon(lat1, lon1) {
                if let Ok(res2) = self.get_transform_from_lat_lon(lat2, lon2) {
                    (res1 - res2).length()
                } else {
                    haversine_distance(lat1, lon1, lat2, lon2) * 1000.0
                }
            } else {
                haversine_distance(lat1, lon1, lat2, lon2) * 1000.0
            }
        };
        (dy, dx)
    }

    pub fn from_latlon(zoom: i32, lat_deg: f32, lon_deg: f32) -> Self {
        let n = 2.0f32.powi(zoom);

        // X tile stuff
        let xtile = n * ((lon_deg + 180.0) / 360.0);
        let xtile = xtile.floor() as i32;
        let lat_rad = lat_deg.to_radians();

        // Y tile stuff
        let trig = (lat_rad.tan() + (1f32 / lat_rad.cos())).ln();
        let inner = 1f32 - (trig / PI);
        let ytile = inner * 2f32.powi(zoom - 1);
        let ytile = ytile.floor() as i32;

        Self { xtile, ytile, zoom }
    }

    pub async fn get_map_image<'a, 'b>(&'b self) -> Result<Box<Reader<'a>>, AssetReaderError> {
        let cache_ok: bool;
        let mut cache_full_path: PathBuf;
        #[cfg(not(target_arch = "wasm32"))]
        {
            let cache_file_name =
                format!("tile_cache_{}_{}_{}.png", self.zoom, self.xtile, self.ytile);
            cache_full_path = cache_path().clone();
            cache_full_path.push("slippy_maps");
            let err = std::fs::create_dir_all(cache_full_path.clone());
            cache_ok = err.is_ok();
            cache_full_path.push(cache_file_name);
            // TODO(luca) check if we can have an async file read here instead?
            if std::path::Path::new(&cache_full_path).exists() && cache_ok {
                return Ok(Box::new(VecReader::new(
                    std::fs::read(&cache_full_path).map_err(std::io::Error::other)?,
                )));
            }
        }

        // TODO(arjoc): make configurable
        let uri = format!(
            "https://tile.openstreetmap.org/{}/{}/{}.png",
            self.zoom, self.xtile, self.ytile
        );

        let request = ehttp::Request::get(uri);
        let bytes = ehttp::fetch_async(request)
            .await
            .map_err(std::io::Error::other)?
            .bytes;

        #[cfg(not(target_arch = "wasm32"))]
        {
            if cache_ok {
                let file = std::fs::File::create(cache_full_path);
                if let Ok(mut file) = file {
                    if file.write_all(&bytes).is_err() {
                        println!("Could not save cache");
                    }
                }
            }
        }

        Ok(Box::new(VecReader::new(bytes)))
    }
}

pub fn zigzag_iter(start: i32, end: i32) -> impl Iterator<Item = i32> {
    let diff = end - start;
    let center = start + diff / 2;
    (1..diff + 2).map(move |d| center + d / 2 * (-1i32).pow(d as u32))
}

#[test]
fn test_zigzag_iter() {
    let v: Vec<_> = zigzag_iter(1, 5).collect();
    assert_eq!(v, vec![3, 4, 2, 5, 1]);
    let v: Vec<_> = zigzag_iter(1, 4).collect();
    assert_eq!(v, vec![2, 3, 1, 4]);
}

pub fn generate_map_tiles(
    lat1: f32,
    lon1: f32,
    lat2: f32,
    lon2: f32,
    zoom: i32,
) -> impl Iterator<Item = OSMTile> {
    let start_tile = OSMTile::from_latlon(zoom, lat1, lon1);
    let end_tile = OSMTile::from_latlon(zoom, lat2, lon2);

    //TODO(arjo): Support world's end
    zigzag_iter(end_tile.ytile, start_tile.ytile + 1).flat_map(move |y| {
        zigzag_iter(start_tile.xtile, end_tile.xtile + 1).map(move |x| OSMTile {
            xtile: x,
            ytile: y,
            zoom,
        })
    })
}

#[test]
fn test_north_eastern_hemisphere() {
    // Singapore coordinates
    let (lat, lon) = (1.343_746, 103.824_04);
    let tile = OSMTile::from_latlon(11, lat, lon);

    assert!(tile.xtile == 1614);
    assert!(tile.ytile == 1016);

    let (nw_lat, nw_lon) = tile.get_nw_corner();
    let (se_lat, se_lon) = tile.get_se_corner();

    assert!(nw_lat > lat);
    assert!(nw_lon < lon);

    assert!(se_lat < lat);
    assert!(se_lon > lon);
}
