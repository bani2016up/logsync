/// Aligned log messages with equal-length columns and newline placeholders.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CompareResult {
    left_container: Vec<String>,
    right_container: Vec<String>,
    length: usize,
}

impl CompareResult {
    pub(crate) fn new(
        left_container: Vec<String>,
        right_container: Vec<String>,
    ) -> Result<Self, &'static str> {
        let length = left_container.len();
        if length != right_container.len() {
            return Err("comparison columns must have the same length");
        }

        Ok(Self {
            left_container,
            right_container,
            length,
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
}

#[cfg(test)]
mod tests {
    use super::CompareResult;

    #[test]
    fn stores_equal_length_columns_and_computes_length() {
        let result = CompareResult::new(vec!["left".into()], vec!["right".into()]).unwrap();

        assert_eq!(result.left_container(), ["left"]);
        assert_eq!(result.right_container(), ["right"]);
        assert_eq!(result.length(), 1);
    }

    #[test]
    fn accepts_empty_columns() {
        let result = CompareResult::new(vec![], vec![]).unwrap();

        assert!(result.left_container().is_empty());
        assert!(result.right_container().is_empty());
        assert_eq!(result.length(), 0);
    }

    #[test]
    fn rejects_mismatched_lengths() {
        for (left, right) in [
            (vec!["left".into()], vec![]),
            (vec![], vec!["right".into()]),
            (vec!["left".into()], vec!["right".into(), "extra".into()]),
        ] {
            assert!(CompareResult::new(left, right).is_err());
        }
    }
}
