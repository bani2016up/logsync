/// Aligned log messages with equal-length columns and newline placeholders.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CompareResult {
    containers: Vec<Vec<String>>,
    timestamps: Vec<String>,
    length: usize,
    max_timestamp_length: usize,
}

impl CompareResult {
    pub(crate) fn new(
        containers: Vec<Vec<String>>,
        timestamps: Vec<String>,
    ) -> Result<Self, String> {
        let length = containers.first().map_or(0, Vec::len);
        for (index, container) in containers.iter().enumerate() {
            if container.len() != length {
                return Err(format!(
                    "Container {index} has length {}, expected {length}",
                    container.len()
                ));
            }
        }
        if timestamps.len() != length {
            return Err(format!(
                "Timestamp column has length {}, expected {length}",
                timestamps.len()
            ));
        }
        let max_timestamp_length = timestamps
            .iter()
            .map(|text| text.chars().count())
            .max()
            .unwrap_or(0);

        Ok(Self {
            containers,
            timestamps,
            length,
            max_timestamp_length,
        })
    }

    pub(crate) fn containers(&self) -> &[Vec<String>] {
        &self.containers
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
            vec![
                vec!["first".into()],
                vec!["second".into()],
                vec!["third".into()],
            ],
            vec!["timestamp".into()],
        )
        .unwrap();

        assert_eq!(result.containers()[0], ["first"]);
        assert_eq!(result.containers()[1], ["second"]);
        assert_eq!(result.containers()[2], ["third"]);
        assert_eq!(result.timestamps(), ["timestamp"]);
        assert_eq!(result.length(), 1);
        assert_eq!(result.max_timestamp_length(), 9);
    }

    #[test]
    fn accepts_empty_columns() {
        for count in [0, 1, 2, 4] {
            let result = CompareResult::new(vec![vec![]; count], vec![]).unwrap();
            assert_eq!(result.containers().len(), count);
            assert!(result.containers().iter().all(Vec::is_empty));
            assert!(result.timestamps().is_empty());
            assert_eq!(result.length(), 0);
            assert_eq!(result.max_timestamp_length(), 0);
        }
    }

    #[test]
    fn rejects_mismatched_lengths() {
        for containers in [
            vec![vec!["first".into()], vec![]],
            vec![vec![], vec!["second".into()]],
            vec![vec!["first".into()], vec!["second".into()], vec![]],
            vec![vec!["first".into()], vec!["second".into(), "extra".into()]],
        ] {
            assert!(CompareResult::new(containers, vec!["timestamp".into()]).is_err());
        }
    }

    #[test]
    fn rejects_mismatched_timestamps() {
        for timestamps in [vec![], vec!["first".into(), "second".into()]] {
            assert!(
                CompareResult::new(
                    vec![vec!["first".into()], vec!["second".into()]],
                    timestamps
                )
                .is_err()
            );
        }
        assert!(CompareResult::new(vec![], vec!["timestamp".into()]).is_err());
    }
}
