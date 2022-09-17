/*
 * Copyright (C) 2022 Open Source Robotics Foundation
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

/// The edge trait is used to unify the APIs of several edge-like site elements,
/// such as lane, wall, door, measurement, and lift. These elements have a
/// common theme: their position is (at least partially) defined by a pair of
/// anchors. This trait gives a unified way of accessing and mutating those
/// anchors.
///
/// Types that implement this trait should only implement [`endpoints`] and
/// [`endpoints_mut`]. All other functions should keep their standard
/// implementation.
pub trait Edge<T> {
    /// Get the endpoints of this edge.
    fn endpoints(&self) -> (T, T);

    /// Get mutable references to the endpoints of this edge.
    fn endpoints_mut(&mut self) -> (&mut T, &mut T);

    /// Create a new edge of this type using the given anchors. All other
    /// properties of the edge should have sensible default values.
    fn new(anchors: (T, T)) -> Self;

    fn left(&self) -> T {
        self.endpoints().0
    }

    fn left_mut(&mut self) -> &mut T {
        self.endpoints_mut().0
    }

    fn right(&self) -> T {
        self.endpoints().1
    }

    fn right_mut(&mut self) -> &mut T {
        self.endpoints_mut().1
    }

    fn start(&self) -> T {
        self.left()
    }

    fn start_mut(&mut self) -> &mut T {
        self.left_mut()
    }

    fn end(&self) -> T {
        self.right()
    }

    fn end_mut(&mut self) -> &mut T {
        self.right_mut()
    }
}
