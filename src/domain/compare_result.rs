/// Aligned log messages with equal-length columns and newline placeholders.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CompareResult {
    left_container: Vec<String>,
    right_container: Vec<String>,
    timestamps: Vec<String>,
    length: usize,
    max_timestamp_length: usize,
}

impl CompareResult {
    pub(crate) fn new(
        left_container: Vec<String>,
        right_container: Vec<String>,
        timestamps: Vec<String>,
    ) -> Result<Self, &'static str> {
        let length = left_container.len();
        if length != right_container.len() || length != timestamps.len() {
            return Err("comparison columns must have the same length");
        }

        let max_timestamp_length = timestamps
            .iter()
            .map(|text| text.chars().count())
            .max()
            .unwrap_or(0);
        Ok(Self {
            left_container,
            right_container,
            timestamps,
            length,
            max_timestamp_length,
        })
    }

    pub(crate) fn left_container(&self) -> &[String] {
        &self.left_container
    }

    pub(crate) fn right_container(&self) -> &[String] {
        &self.right_container
    }

    pub(crate) fn length(&self) -> usize {
        self.length
    }

    pub(crate) fn timestamps(&self) -> &[String] {
        &self.timestamps
    }

    pub(crate) fn max_timestamp_length(&self) -> usize {
        self.max_timestamp_length
    }
}

#[cfg(test)]
mod tests {
    use super::CompareResult;

    #[test]
    fn stores_equal_length_columns_and_computes_length() {
        let result = CompareResult::new(
            vec!["left".into()],
            vec!["right".into()],
            vec!["timestamp".into()],
        )
        .unwrap();

        assert_eq!(result.left_container(), ["left"]);
        assert_eq!(result.right_container(), ["right"]);
        assert_eq!(result.timestamps(), ["timestamp"]);
        assert_eq!(result.length(), 1);
        assert_eq!(result.max_timestamp_length(), 9);
    }

    #[test]
    fn accepts_empty_columns() {
        let result = CompareResult::new(vec![], vec![], vec![]).unwrap();

        assert!(result.left_container().is_empty());
        assert!(result.right_container().is_empty());
        assert!(result.timestamps().is_empty());
        assert_eq!(result.length(), 0);
        assert_eq!(result.max_timestamp_length(), 0);
    }

    #[test]
    fn rejects_mismatched_lengths() {
        for (left, right) in [
            (vec!["left".into()], vec![]),
            (vec![], vec!["right".into()]),
            (vec!["left".into()], vec!["right".into(), "extra".into()]),
        ] {
            assert!(CompareResult::new(left, right, vec!["timestamp".into()]).is_err());
        }
    }

    #[test]
    fn rejects_mismatched_timestamps() {
        for timestamps in [vec![], vec!["first".into(), "second".into()]] {
            assert!(
                CompareResult::new(vec!["left".into()], vec!["right".into()], timestamps).is_err()
            );
        }
    }
}
