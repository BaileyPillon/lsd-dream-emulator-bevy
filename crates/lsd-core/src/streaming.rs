/// Inclusive bounds: two signed halfword minima and two signed word maxima.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellBounds {
    pub min_x: i16,
    pub min_z: i16,
    pub max_x: i32,
    pub max_z: i32,
}

impl CellBounds {
    /// Original record at 0x80087150. Rendering these bounds is a separate task.
    pub const STAGE_ZERO: Self = Self {
        min_x: 0,
        min_z: 0,
        max_x: 8,
        max_z: 9,
    };

    pub const fn contains(self, point: [i8; 2]) -> bool {
        let x = point[0] as i32;
        let z = point[1] as i32;
        x >= self.min_x as i32 && x <= self.max_x && z >= self.min_z as i32 && z <= self.max_z
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidLayer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamSelection {
    layers: [i16; 3],
    len: usize,
}

impl StreamSelection {
    pub fn as_slice(&self) -> &[i16] {
        &self.layers[..self.len]
    }
}

/// Original slot order for a valid current layer: current, optional next, previous.
/// This selects requested layers; it does not load files or determine residency.
pub fn select_stream_layers(
    current: i16,
    layer_count: i16,
    point: [i8; 2],
    exclusion: Option<CellBounds>,
) -> Result<StreamSelection, InvalidLayer> {
    if current < 0 || layer_count <= 0 || current >= layer_count {
        return Err(InvalidLayer);
    }
    let mut selection = StreamSelection {
        layers: [current, 0, 0],
        len: 1,
    };
    if exclusion.is_none_or(|bounds| !bounds.contains(point)) && current < layer_count - 1 {
        selection.layers[selection.len] = current + 1;
        selection.len += 1;
    }
    if current > 0 {
        selection.layers[selection.len] = current - 1;
        selection.len += 1;
    }
    Ok(selection)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_layers_outside_the_reviewed_domain() {
        for (current, count) in [(-1, 7), (7, 7), (0, 0), (0, -1)] {
            assert_eq!(
                select_stream_layers(current, count, [0, 0], None),
                Err(InvalidLayer)
            );
        }
    }

    #[test]
    fn full_grid_has_ninety_excluded_cells_and_preserves_row_order() {
        let bounds = CellBounds::STAGE_ZERO;
        let inside = (0..20)
            .flat_map(|x| (0..20).map(move |z| [x, z]))
            .filter(|point| bounds.contains(*point))
            .count();
        assert_eq!(inside, 90);
        assert_eq!(
            select_stream_layers(3, 7, [8, 9], Some(bounds))
                .unwrap()
                .as_slice(),
            [3, 2]
        );
        assert_eq!(
            select_stream_layers(3, 7, [9, 8], Some(bounds))
                .unwrap()
                .as_slice(),
            [3, 4, 2]
        );
        assert_eq!(
            select_stream_layers(0, 1, [0, 0], None).unwrap().as_slice(),
            [0]
        );
    }
}
