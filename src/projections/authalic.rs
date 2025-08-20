// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use crate::coordinate_systems::Radians;

// Authalic conversion coefficients obtained from: https://arxiv.org/pdf/2212.05818
// See: authalic_constants.py for the derivation of the coefficients
const GEODETIC_TO_AUTHALIC: [f64; 6] = [
    -2.2392098386786394e-03,
    2.1308606513250217e-06,
    -2.5592576864212742e-09,
    3.3701965267802837e-12,
    -4.6675453126112487e-15,
    6.6749287038481596e-18,
];

const AUTHALIC_TO_GEODETIC: [f64; 6] = [
    2.2392089963541657e-03,
    2.8831978048607556e-06,
    5.0862207399726603e-09,
    1.0201812377816100e-11,
    2.1912872306767718e-14,
    4.9284235482523806e-17,
];

// Adaptation of applyCoefficients from DGGAL project: authalic.ec
//
// BSD 3-Clause License
//
// Copyright (c) 2014-2025, Ecere Corporation
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//
// 1. Redistributions of source code must retain the above copyright notice, this
//    list of conditions and the following disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright notice,
//    this list of conditions and the following disclaimer in the documentation
//    and/or other materials provided with the distribution.
//
// 3. Neither the name of the copyright holder nor the names of its
//    contributors may be used to endorse or promote products derived from
//    this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
// OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

/// Authalic projection implementation that converts between geodetic and authalic latitudes.
pub struct AuthalicProjection;

impl AuthalicProjection {
    /// Applies coefficients using Clenshaw summation algorithm (order 6)
    ///
    /// # Arguments
    ///
    /// * `phi` - Angle in radians
    /// * `c` - Array of coefficients
    ///
    /// # Returns
    ///
    /// Transformed angle in radians
    fn apply_coefficients(&self, phi: Radians, c: &[f64; 6]) -> Radians {
        let sin_phi = phi.get().sin();
        let cos_phi = phi.get().cos();
        let x = 2.0 * (cos_phi - sin_phi) * (cos_phi + sin_phi);

        let u0 = x * c[5] + c[4];
        let u1 = x * u0 + c[3];
        let u0 = x * u1 - u0 + c[2];
        let u1 = x * u0 - u1 + c[1];
        let u0 = x * u1 - u0 + c[0];

        Radians::new_unchecked(phi.get() + 2.0 * sin_phi * cos_phi * u0)
    }

    /// Converts geodetic latitude to authalic latitude
    ///
    /// # Arguments
    ///
    /// * `phi` - Geodetic latitude in radians
    ///
    /// # Returns
    ///
    /// Authalic latitude in radians
    pub fn forward(&self, phi: Radians) -> Radians {
        self.apply_coefficients(phi, &GEODETIC_TO_AUTHALIC)
    }

    /// Converts authalic latitude to geodetic latitude
    ///
    /// # Arguments
    ///
    /// * `phi` - Authalic latitude in radians
    ///
    /// # Returns
    ///
    /// Geodetic latitude in radians
    pub fn inverse(&self, phi: Radians) -> Radians {
        self.apply_coefficients(phi, &AUTHALIC_TO_GEODETIC)
    }
}
