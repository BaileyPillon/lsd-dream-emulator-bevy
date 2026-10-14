/// Tables loaded from the user's identified original executable; no game data is bundled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PsxMathTables {
    pub sin_cos: Box<[u32; 4096]>,
    /// Halfwords from sqrt-table base - 640 bytes through base + 384 bytes.
    /// The preceding words preserve the SDK's negative-input lookup behavior.
    pub sqrt_window: Box<[i16; 512]>,
}

impl PsxMathTables {
    pub fn sin_cos(&self, angle: i16) -> (i32, i32) {
        let index = usize::from(angle.unsigned_abs() & 4095);
        let word = self.sin_cos[index];
        let sine = i32::from(word as i16);
        (
            if angle < 0 { -sine } else { sine },
            i32::from((word >> 16) as i16),
        )
    }

    pub fn rotation(&self, angles: [i16; 3]) -> FixedMatrix {
        let (sx, cx) = self.sin_cos(angles[0]);
        let (sy, cy) = self.sin_cos(angles[1]);
        let (sz, cz) = self.sin_cos(angles[2]);
        let product = |left: i32, right: i32| left.wrapping_mul(right) >> 12;
        let negative_product =
            |left: i32, right: i32| left.wrapping_mul(right).wrapping_neg() >> 12;
        let cy_cz = product(cz, cy);
        let sy_cz = product(cz, -sy);
        let sy_sz = product(sz, -sy);
        FixedMatrix([
            [cy_cz as i16, negative_product(sz, cy) as i16, sy as i16],
            [
                product(sz, cx).wrapping_sub(product(sy_cz, sx)) as i16,
                product(cz, cx).wrapping_add(product(sy_sz, sx)) as i16,
                negative_product(cy, sx) as i16,
            ],
            [
                product(sy_cz, cx).wrapping_add(product(sz, sx)) as i16,
                product(cz, sx).wrapping_sub(product(sy_sz, cx)) as i16,
                product(cy, cx) as i16,
            ],
        ])
    }

    pub fn square_root(&self, value: i32) -> u32 {
        let leading = if value < 0 {
            (!value as u32).leading_zeros()
        } else {
            (value as u32).leading_zeros()
        };
        if leading == 32 {
            return 0;
        }
        let even = leading & !1;
        let exponent = (31 - even) / 2;
        let normalized = if even >= 24 {
            value.wrapping_shl(even - 24)
        } else {
            value >> (24 - even)
        };
        let index = usize::try_from(normalized + 256).expect("SDK normalized square-root index");
        (i32::from(self.sqrt_window[index]) as u32).wrapping_shl(exponent) >> 12
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedMatrix(pub [[i16; 3]; 3]);

impl FixedMatrix {
    pub const IDENTITY: Self = Self([[4096, 0, 0], [0, 4096, 0], [0, 0, 4096]]);

    /// MAC results before the GTE's optional IR saturation, for short local vectors.
    pub fn apply_short(self, vector: [i16; 3]) -> [i32; 3] {
        self.0.map(|row| {
            let sum: i64 = row
                .iter()
                .zip(vector)
                .map(|(&coefficient, value)| i64::from(coefficient) * i64::from(value))
                .sum();
            (sum >> 12) as i32
        })
    }

    pub fn apply_saturated(self, vector: [i16; 3]) -> [i16; 3] {
        self.apply_short(vector)
            .map(|value| value.clamp(-32768, 32767) as i16)
    }

    /// SDK MulMatrix2 multiplies each right-hand column and stores saturated IR halfwords.
    pub fn multiply(self, right: Self) -> Self {
        let columns: [[i16; 3]; 3] = std::array::from_fn(|column| {
            self.apply_saturated(std::array::from_fn(|row| right.0[row][column]))
        });
        Self(std::array::from_fn(|row| {
            std::array::from_fn(|column| columns[column][row])
        }))
    }
}

impl PsxMathTables {
    /// Local inverse rotation followed by each parent's inverse, retaining SDK rounding.
    pub fn inverse_rotation_chain(&self, local: [i16; 3], parents: &[[i16; 3]]) -> FixedMatrix {
        let mut matrix = self.rotation(local.map(i16::wrapping_neg));
        for parent in parents {
            matrix = self
                .rotation(parent.map(i16::wrapping_neg))
                .multiply(matrix);
        }
        matrix
    }
}

/// The transform setter reads each word as a signed numerator/denominator pair.
pub fn transform_number(word: u32) -> Option<i32> {
    let numerator = i32::from(word as i16);
    let denominator = i32::from((word >> 16) as i16);
    if denominator == 0 {
        return None;
    }
    Some(
        (numerator / denominator)
            .wrapping_shl(12)
            .wrapping_add((numerator % denominator).wrapping_shl(12) / denominator),
    )
}
