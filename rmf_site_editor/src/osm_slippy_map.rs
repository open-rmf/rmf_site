/*
 * Copyright (C) 2023 Intrinsic LLC
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

use std::f32::consts::PI;
use std::io::Write;

const EARTH_RADIUS: f32 = 6371.0;

fn haversine_distance(lat1: f32, lon1: f32, lat2: f32, lon2: f32) -> f32 {
    let lat1 = lat1.to_radians();
    let lon1 = lon1.to_radians();
    let lat2 = lat2.to_radians();
    let lon2 = lon2.to_radians();

    let dLat = lat2 - lat1;
    let dLon = lon2 - lon1;

    let a = (dLat/2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dLon/2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    return c * EARTH_RADIUS;
}

enum DistanceError {
    DifferentZones
}

#[test]
fn test_haversine() {
    
    // Distance from Singapore to Kolkata (about 2891KM)
    let d = haversine_distance(1.3521, 103.8198, 22.5726, 88.3639);
    assert!( (d - 2891.0).abs() < 1.0 );

    // Distance from hyundai_factory to whale_museum (about 3.48KM)
    let hyundai_factory = (35.503201188171076, 129.3809451273798);
    let whale_museum = (35.53330554519475, 129.38965867799482);

    let d = haversine_distance(hyundai_factory.0, hyundai_factory.1, whale_museum.0, whale_museum.1);
    assert!( (d - 3.48).abs() < 0.1 );

    // Distance from one-north mrt in Singapore to SUTD (about 20.2KM)
    let one_north = (1.2991849898682075, 103.78709256771138);
    let sutd = (1.3417113432463037, 103.96381226270485);

    let d = haversine_distance(one_north.0, one_north.1, sutd.0, sutd.1);
    println!("{}", d);
    assert!( (d - 20.2).abs() < 0.1 );

}

#[derive(Debug, Clone)]
pub struct OSMTile {
    xtile: i32,
    ytile: i32,
    zoom: i32,
}

impl OSMTile {
    /// Returns the northwest corner
    pub fn get_nw_corner(&self) -> (f32, f32) {
        let n = 2f32.powi(self.zoom);
        let lon_deg = self.xtile as f32 / n * 360.0 - 180.0;
        let lat_rad = (PI * (1.0 - 2.0 * self.ytile as f32 / n)).sinh().atan();
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

    pub fn tile_size(&self) -> (f32, f32) {
        let (lat1, lon1)= self.get_nw_corner();
        let dy = {
            let (lat2, lon2)= self.get_sw_corner();
            haversine_distance(lat1, lon1, lat2, lon2)
        };
        let dx = {
            let (lat2, lon2)= self.get_ne_corner();
            haversine_distance(lat1, lon1, lat2, lon2)
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

    pub async fn get_map_image(&self) -> Result<Vec<u8>, surf::Error> {
        
        /*let cache_file_name = format!("tile_cache_{}_{}_{}.png", self.zoom, self.xtile, self.ytile);
        if std::path::Path::new(&cache_file_name).exists() {
            return Ok(std::fs::read(&cache_file_name).await?);
        }*/

        let uri = format!(
            "https://tile.openstreetmap.org/{}/{}/{}.png",
            self.zoom, self.xtile, self.ytile
        );
    
        println!("Getting URI {uri}");
    
        let mut result = surf::get(uri).await?;
    
        let bytes = result.body_bytes().await?;
    
        /*let mut file = std::fs::File::create(cache_file_name)?;
        file.write_all(&bytes)?;*/
    
        Ok(bytes)
    }
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
