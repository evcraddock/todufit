//! Document type enumeration for Automerge storage.

/// Document types that can be stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocType {
    Dishes,
    MealPlans,
    MealLogs,
}

impl DocType {
    /// Returns the filename for this document type.
    pub fn filename(&self) -> &'static str {
        match self {
            DocType::Dishes => "dishes.automerge",
            DocType::MealPlans => "mealplans.automerge",
            DocType::MealLogs => "meallogs.automerge",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doc_type_filename() {
        assert_eq!(DocType::Dishes.filename(), "dishes.automerge");
        assert_eq!(DocType::MealPlans.filename(), "mealplans.automerge");
        assert_eq!(DocType::MealLogs.filename(), "meallogs.automerge");
    }
}
